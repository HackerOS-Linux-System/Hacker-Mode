<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  let volume = 50;
  let muted = false;

  async function load() {
    volume = await invoke<number>("audio_get_volume");
    muted = await invoke<boolean>("audio_get_muted");
  }

  async function onVolumeChange(e: Event) {
    volume = Number((e.target as HTMLInputElement).value);
    await invoke("audio_set_volume", { percent: volume });
  }

  async function toggleMute() {
    muted = !muted;
    await invoke("audio_set_muted", { muted });
  }

  onMount(load);
</script>

<div class="section">
  <div>
    <h2>Dźwięk</h2>
    <p class="muted">PipeWire / WirePlumber (wpctl)</p>
  </div>

  <div class="slider-row">
    <button class="glyph hm-focusable" on:click={toggleMute}>{muted ? "🔇" : "🔊"}</button>
    <input type="range" min="0" max="100" value={volume} on:input={onVolumeChange} />
    <span class="value">{volume}%</span>
  </div>
</div>

<style>
  @import "../settings/shared.css";
</style>
