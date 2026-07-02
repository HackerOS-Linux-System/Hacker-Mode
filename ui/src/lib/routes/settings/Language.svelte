<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  let current = "";
  let locales: string[] = [];
  let notice = "";

  async function load() {
    current = await invoke<string>("locale_get");
    locales = await invoke<string[]>("locale_list");
  }

  async function onChange(e: Event) {
    const locale = (e.target as HTMLSelectElement).value;
    await invoke("locale_set", { locale });
    current = locale;
    notice = "Zmiana języka wymaga ponownego zalogowania.";
  }

  onMount(load);
</script>

<div class="section">
  <div>
    <h2>Język systemu</h2>
    <p class="muted">localectl</p>
  </div>

  <select class="hm-focusable" value={current} on:change={onChange}>
    {#each locales as l}
      <option value={l}>{l}</option>
    {/each}
  </select>

  {#if notice}
    <p class="muted">{notice}</p>
  {/if}
</div>

<style>
  @import "../settings/shared.css";
</style>
