<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  let current = "";
  let zones: string[] = [];

  async function load() {
    current = await invoke<string>("tz_get");
    zones = await invoke<string[]>("tz_list");
  }

  async function onChange(e: Event) {
    const tz = (e.target as HTMLSelectElement).value;
    await invoke("tz_set", { tz });
    current = tz;
  }

  onMount(load);
</script>

<div class="section">
  <div>
    <h2>Strefa czasowa</h2>
    <p class="muted">timedatectl</p>
  </div>

  <select class="hm-focusable" value={current} on:change={onChange}>
    {#each zones as z}
      <option value={z}>{z}</option>
    {/each}
  </select>
</div>

<style>
  @import "../settings/shared.css";
</style>
