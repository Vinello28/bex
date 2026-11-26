#!/usr/bin/env python3
"""Bundle non-system macOS dylib dependencies into src-tauri/target/macos-frameworks
and rewrite Mach-O load commands before Tauri packages and signs the .app bundle.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from collections import deque
from pathlib import Path

BINARY_NAME = "bex-latex-studio"
SYSTEM_PREFIXES = ("/System/", "/usr/lib/")


def run_cmd(cmd: list[str]) -> str:
    """Run a subprocess command and return stdout, surfacing stderr on failure."""
    try:
        res = subprocess.run(cmd, check=True, text=True, capture_output=True)
        return res.stdout
    except subprocess.CalledProcessError as exc:
        msg = [
            f"Command failed ({exc.returncode}): {' '.join(cmd)}",
            f"stdout:\n{exc.stdout.strip()}" if exc.stdout else "",
            f"stderr:\n{exc.stderr.strip()}" if exc.stderr else "",
        ]
        raise RuntimeError("\n".join(part for part in msg if part)) from exc


def is_system_lib(dep: str) -> bool:
    """Return True if dep is a macOS system library or framework."""
    return dep.startswith(SYSTEM_PREFIXES)


def get_install_id(macho_path: Path) -> str | None:
    """Return the Mach-O install ID (otool -D) for dylibs, or None for executables."""
    out = run_cmd(["otool", "-D", str(macho_path)])
    lines = [line.strip() for line in out.splitlines()[1:] if line.strip()]
    return lines[0] if lines else None


def get_dylib_deps(macho_path: Path) -> list[str]:
    """Return all linked dylib paths for macho_path, excluding its own install ID."""
    out = run_cmd(["otool", "-L", str(macho_path)])
    install_id = get_install_id(macho_path)
    deps: list[str] = []
    skipped_self = False
    for line in out.splitlines()[1:]:
        stripped = line.strip()
        if not stripped:
            continue
        dep = stripped.split(" (", 1)[0]
        if install_id and not skipped_self and dep == install_id:
            skipped_self = True
            continue
        deps.append(dep)
    return deps


def get_rpaths(macho_path: Path) -> list[str]:
    """Extract LC_RPATH entries from a Mach-O binary via otool -l."""
    out = run_cmd(["otool", "-l", str(macho_path)])
    rpaths: list[str] = []
    in_rpath_cmd = False
    for line in out.splitlines():
        stripped = line.strip()
        if stripped == "cmd LC_RPATH":
            in_rpath_cmd = True
            continue
        if in_rpath_cmd:
            if stripped.startswith("cmd "):
                in_rpath_cmd = False
            elif stripped.startswith("path "):
                rest = stripped[len("path ") :]
                rpath = rest.rsplit(" (offset", 1)[0].strip()
                rpaths.append(rpath)
                in_rpath_cmd = False
    return rpaths


def find_target_binary(src_tauri: Path) -> Path:
    """Locate the newest compiled target binary matching the active Tauri profile."""
    profile = "debug" if os.environ.get("TAURI_ENV_DEBUG", "").lower() == "true" else "release"
    triple = os.environ.get("TAURI_ENV_TARGET_TRIPLE", "").strip()
    target_root = src_tauri / "target"

    candidates: list[Path] = []
    if triple:
        candidates.append(target_root / triple / profile / BINARY_NAME)
    candidates.append(target_root / profile / BINARY_NAME)
    if not triple and target_root.is_dir():
        candidates.extend(target_root.glob(f"*/{profile}/{BINARY_NAME}"))

    existing: dict[Path, float] = {}
    for candidate in candidates:
        if candidate.is_file():
            resolved = candidate.resolve()
            existing[candidate] = resolved.stat().st_mtime

    if not existing:
        searched = ", ".join(str(c) for c in candidates)
        raise FileNotFoundError(
            f"Could not find compiled binary '{BINARY_NAME}' for profile '{profile}'. Checked: {searched}"
        )

    return max(existing.items(), key=lambda item: item[1])[0]


def expand_rpath_entry(rpath: str, owner_dir: Path, exe_dir: Path) -> Path:
    """Expand @loader_path / @executable_path prefixes inside an LC_RPATH entry."""
    if rpath.startswith("@loader_path"):
        suffix = rpath[len("@loader_path") :].lstrip("/")
        return owner_dir / suffix if suffix else owner_dir
    if rpath.startswith("@executable_path"):
        suffix = rpath[len("@executable_path") :].lstrip("/")
        return exe_dir / suffix if suffix else exe_dir
    return Path(rpath)


def resolve_dep_path(
    dep: str,
    owner_path: Path,
    owner_rpaths: list[str],
    binary_path: Path,
    frameworks_dir: Path,
) -> Path:
    """Resolve a Mach-O dependency reference to an existing file on disk."""
    basename = Path(dep).name
    owner_real_dir = owner_path.resolve().parent
    owner_raw_dir = owner_path.parent
    exe_dir = binary_path.resolve().parent

    candidates: list[Path] = []
    if dep.startswith("@executable_path/../Frameworks/"):
        candidates.append(frameworks_dir / basename)
    elif dep.startswith("@loader_path/"):
        rel = dep[len("@loader_path/") :]
        candidates.append(owner_real_dir / rel)
        if owner_raw_dir != owner_real_dir:
            candidates.append(owner_raw_dir / rel)
    elif dep.startswith("@rpath/"):
        rel = dep[len("@rpath/") :]
        for rp in owner_rpaths:
            candidates.append(expand_rpath_entry(rp, owner_real_dir, exe_dir) / rel)
            if owner_raw_dir != owner_real_dir:
                candidates.append(expand_rpath_entry(rp, owner_raw_dir, exe_dir) / rel)
        candidates.append(owner_real_dir / rel)
        candidates.append(frameworks_dir / basename)
    elif dep.startswith("/"):
        candidates.append(Path(dep))
    else:
        raise RuntimeError(f"Unhandled dylib install name '{dep}' referenced by {owner_path}")

    for candidate in candidates:
        if candidate.is_file():
            return candidate.resolve()

    tried = ", ".join(str(c) for c in candidates)
    raise FileNotFoundError(
        f"Failed to resolve dylib dependency '{dep}' referenced by '{owner_path}'. Tried: {tried}"
    )


def collect_dependencies(binary_path: Path, frameworks_dir: Path) -> dict[str, Path]:
    """Recursively collect all non-system dylib dependencies of binary_path."""
    resolved_by_name: dict[str, Path] = {}
    queue: deque[Path] = deque([binary_path])
    visited_owners: set[Path] = set()

    while queue:
        owner = queue.popleft()
        owner_real = owner.resolve()
        if owner_real in visited_owners:
            continue
        visited_owners.add(owner_real)

        rpaths = get_rpaths(owner)
        for dep in get_dylib_deps(owner):
            if is_system_lib(dep):
                continue
            resolved = resolve_dep_path(dep, owner, rpaths, binary_path, frameworks_dir)
            basename = Path(dep).name
            if basename in resolved_by_name:
                if resolved_by_name[basename] != resolved:
                    raise RuntimeError(
                        f"Conflicting sources for '{basename}': {resolved_by_name[basename]} vs {resolved}"
                    )
            else:
                resolved_by_name[basename] = resolved
                queue.append(resolved)

    return resolved_by_name


def stage_and_patch_dylibs(
    resolved_by_name: dict[str, Path], staging_dir: Path, frameworks_dir: Path
) -> None:
    """Copy dylibs into staging_dir, rewrite install names to @loader_path, ad-hoc sign, and swap."""
    if staging_dir.exists():
        shutil.rmtree(staging_dir)
    staging_dir.mkdir(parents=True, exist_ok=True)

    for basename in sorted(resolved_by_name):
        src = resolved_by_name[basename]
        dst = staging_dir / basename
        shutil.copy2(src, dst)
        # Homebrew Cellar dylibs are installed 0444; make writable before install_name_tool/codesign.
        os.chmod(dst, 0o755)

    for basename in sorted(resolved_by_name):
        dst = staging_dir / basename
        run_cmd(["install_name_tool", "-id", f"@loader_path/{basename}", str(dst)])
        for dep in get_dylib_deps(dst):
            if is_system_lib(dep):
                continue
            target = f"@loader_path/{Path(dep).name}"
            if dep != target:
                run_cmd(["install_name_tool", "-change", dep, target, str(dst)])
        run_cmd(["codesign", "--force", "--sign", "-", "--timestamp=none", str(dst)])

    if frameworks_dir.exists():
        shutil.rmtree(frameworks_dir)
    staging_dir.rename(frameworks_dir)


def patch_main_binary(binary_path: Path) -> None:
    """Rewrite non-system dylib references in binary_path to @executable_path/../Frameworks and re-sign."""
    os.chmod(binary_path, binary_path.stat().st_mode | 0o200)
    for dep in get_dylib_deps(binary_path):
        if is_system_lib(dep):
            continue
        target = f"@executable_path/../Frameworks/{Path(dep).name}"
        if dep != target:
            run_cmd(["install_name_tool", "-change", dep, target, str(binary_path)])
    run_cmd(["codesign", "--force", "--sign", "-", "--timestamp=none", str(binary_path)])


def verify_bundle(binary_path: Path, frameworks_dir: Path) -> list[str]:
    """Verify binary_path and all staged dylibs have zero external links and valid ad-hoc signatures."""
    for dep in get_dylib_deps(binary_path):
        if is_system_lib(dep):
            continue
        prefix = "@executable_path/../Frameworks/"
        if not dep.startswith(prefix):
            raise RuntimeError(f"Main binary still references non-bundled dylib: {dep}")
        name = dep[len(prefix) :]
        if not (frameworks_dir / name).is_file():
            raise RuntimeError(f"Main binary references missing framework dylib: {frameworks_dir / name}")

    bundled = sorted(frameworks_dir.glob("*.dylib"))
    for dylib in bundled:
        for dep in get_dylib_deps(dylib):
            if is_system_lib(dep):
                continue
            prefix = "@loader_path/"
            if not dep.startswith(prefix):
                raise RuntimeError(f"Bundled dylib {dylib.name} still references external path: {dep}")
            name = dep[len(prefix) :]
            if not (frameworks_dir / name).is_file():
                raise RuntimeError(
                    f"Bundled dylib {dylib.name} references missing sibling dylib: {frameworks_dir / name}"
                )
        run_cmd(["codesign", "--verify", "--strict", str(dylib)])

    run_cmd(["codesign", "--verify", "--strict", str(binary_path)])
    return [d.name for d in bundled]


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    src_tauri = repo_root / "src-tauri"
    frameworks_dir = src_tauri / "target" / "macos-frameworks"
    staging_dir = src_tauri / "target" / "macos-frameworks.tmp"

    binary_path = find_target_binary(src_tauri)
    resolved_by_name = collect_dependencies(binary_path, frameworks_dir)
    stage_and_patch_dylibs(resolved_by_name, staging_dir, frameworks_dir)
    patch_main_binary(binary_path)
    bundled_names = verify_bundle(binary_path, frameworks_dir)

    print(
        f"[bundle-macos-dylibs] Patched {binary_path} and bundled {len(bundled_names)} dylib(s) into {frameworks_dir}: "
        + (", ".join(bundled_names) if bundled_names else "(none)")
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
