<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { hackerMenuOpen } from "../stores/nav";

  let volume = 0;
  let brightnessVal = 0;
  let wifiOn = true;
  let clock = "";

  function tickClock() {
    const now = new Date();
    clock = now.toLocaleTimeString("pl-PL", { hour: "2-digit", minute: "2-digit" });
  }

  onMount(() => {
    tickClock();
    const interval = setInterval(tickClock, 15000);
    invoke<number>("audio_get_volume").then((v) => (volume = v)).catch(() => {});
    invoke<number>("brightness_get").then((v) => (brightnessVal = v)).catch(() => {});
    invoke<boolean>("wifi_is_enabled").then((v) => (wifiOn = v)).catch(() => {});
    return () => clearInterval(interval);
  });
</script>

<header class="topbar">
  <div class="left">
    <span class="eyebrow">HACKER MODE</span>
  </div>

  <div class="right">
    <span class="status-chip" title="WiFi">{wifiOn ? "📶" : "🚫"}</span>
    <span class="status-chip" title="Głośność">{volume}%</span>
    <span class="status-chip" title="Jasność">☀ {brightnessVal}%</span>
    <span class="clock">{clock}</span>
    <button
      class="hacker-menu-btn hm-focusable"
      on:click={() => hackerMenuOpen.set(true)}
    >
      <span class="glyph">●</span> Hacker Menu
    </button>
  </div>
</header>

<style>
  .topbar {
    height: var(--topbar-h);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 24px;
    background: linear-gradient(180deg, rgba(19, 25, 32, 0.9), rgba(19, 25, 32, 0.4));
  }
  .right {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .status-chip {
    font-size: 13px;
    color: var(--text-secondary);
  }
  .clock {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--text-primary);
  }
  .hacker-menu-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-radius: 999px;
    background: var(--bg-panel-raised);
    color: var(--text-primary);
    font-weight: 700;
    font-size: 13px;
  }
  .hacker-menu-btn .glyph {
    color: var(--accent);
  }
  .hacker-menu-btn:hover {
    background: var(--bg-panel-hover);
  }
</style>
