import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const pkg = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8")
) as { version: string };

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**", "**/sample_project/**"],
    },
  },
  optimizeDeps: {
    exclude: ["pdfjs-dist"],
  },
  build: {
    target: "esnext",
  },
});
