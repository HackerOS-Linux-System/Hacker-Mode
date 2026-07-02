<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  let brightnessVal = 70;

  async function load() {
    brightnessVal = await invoke<number>("brightness_get");
  }

  async function onChange(e: Event) {
    brightnessVal = Number((e.target as HTMLInputElement).value);
    await invoke("brightness_set", { percent: brightnessVal });
  }

  onMount(load);
</script>

<div class="section">
  <div>
    <h2>Jasność</h2>
    <p class="muted">brightnessctl</p>
  </div>

  <div class="slider-row">
    <span class="glyph">☀</span>
    <input type="range" min="1" max="100" value={brightnessVal} on:input={onChange} />
    <span class="value">{brightnessVal}%</span>
  </div>
</div>

<style>
  @import "../settings/shared.css";
</style>
