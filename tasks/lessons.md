# Bex LaTeX Studio — Lessons Learned

## UI Color Schemes (True Black Dark Mode & Pure Neutral Light Mode)
- Never default to bluish Slate palettes (e.g., `#0f141c`, `#161c26`, `#0f172a`, `#1e293b`, `#f1f5f9`) for neutral UI surfaces, borders, text, or CodeMirror themes when designing dark or light themes.
- When building dark mode, use true black and neutral grayscale (`#000000`, `#0a0a0a`, `#121212`, `#1a1a1a`, `#222222`) without cool/blue undertones; mirror this in light mode with crisp white and pure neutral grays (`#ffffff`, `#fafafa`, `#ececec`, `#dfdfdf`), and avoid blue accent defaults unless requested.

## Branding & App Icons
- Ensure the main mascot/object fills the icon canvas prominently (around 80–85% of the viewBox width/height) so it reads clearly at small sizes (`28px` header icon, `32px` taskbar/dock icon) instead of rendering a skinny silhouette inside an oversized background card.
- Follow explicit brand references faithfully: use Homebrew-style warm golden-amber beer and overflowing foam (not green glass) paired with big expressive cartoon eyes ("occhioni") when crafting the Bex mascot.

## Ownership of Deliverables (Don't Hand Work Back)
- When a deliverable needs assets (screenshots, sample data, real outputs), produce them yourself: launch/install the app, drive it, capture it. Never propose "the user supplies them" as the recommended option — the user delegates precisely to avoid doing the work.
- Before asking the user to do something manual, check whether it can be done from this machine (installed release, WebView2/CDP remote debugging, local services like LM Studio). Only ask for what truly requires them (credentials, physical access, decisions).
