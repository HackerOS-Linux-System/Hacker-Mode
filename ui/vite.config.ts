import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  // build celuje w WebKitGTK (Tauri na Linuksie) — trzymamy target konserwatywny
  build: {
    target: "es2021",
    outDir: "dist",
  },
});
