import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  // Bez tego <script lang="ts"> w plikach .svelte nie kompiluje się —
  // to była przyczyna błędu "Unexpected token" przy `type Route`.
  preprocess: vitePreprocess(),
};
