<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  type Profile = "PowerSaver" | "Balanced" | "Performance";

  interface BatteryInfo {
    percent: number;
    charging: boolean;
    time_remaining_minutes: number | null;
  }

  let profile: Profile = "Balanced";
  let battery: BatteryInfo | null = null;

  const profiles: { id: Profile; label: string; icon: string }[] = [
    { id: "PowerSaver", label: "Oszczędzanie", icon: "🌙" },
    { id: "Balanced", label: "Zbalansowany", icon: "⚖" },
    { id: "Performance", label: "Wydajność", icon: "⚡" },
  ];

  async function load() {
    profile = await invoke<Profile>("power_get_profile");
    battery = await invoke<BatteryInfo | null>("power_get_battery");
  }

  async function setProfile(p: Profile) {
    profile = p;
    await invoke("power_set_profile", { profile: p });
  }

  onMount(load);
</script>

<div class="section">
  <div>
    <h2>Zasilanie i wydajność</h2>
    <p class="muted">power-profiles-daemon</p>
  </div>

  <div class="profile-group">
    {#each profiles as p}
      <button
        class="profile-btn hm-focusable"
        class:active={profile === p.id}
        on:click={() => setProfile(p.id)}
      >
        <div style="font-size:20px;margin-bottom:4px">{p.icon}</div>
        {p.label}
      </button>
    {/each}
  </div>

  {#if battery}
    <div class="row">
      <span class="muted">Bateria</span>
      <span>{battery.percent}% {battery.charging ? "· ładowanie" : ""}</span>
    </div>
  {/if}
</div>

<style>
  @import "../settings/shared.css";
</style>
