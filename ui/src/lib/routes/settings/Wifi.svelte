<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  interface WifiNetwork {
    ssid: string;
    signal: number;
    secured: boolean;
    in_use: boolean;
    saved: boolean;
  }

  let enabled = true;
  let networks: WifiNetwork[] = [];
  let scanning = false;
  let connectingSsid: string | null = null;
  let passwordPrompt: WifiNetwork | null = null;
  let passwordValue = "";

  async function load() {
    enabled = await invoke<boolean>("wifi_is_enabled");
    if (enabled) await scan();
  }

  async function scan() {
    scanning = true;
    try {
      networks = (await invoke<WifiNetwork[]>("wifi_scan")).sort((a, b) => b.signal - a.signal);
    } finally {
      scanning = false;
    }
  }

  async function toggle() {
    enabled = !enabled;
    await invoke("wifi_set_enabled", { enabled });
    if (enabled) await scan();
  }

  async function connect(net: WifiNetwork) {
    if (net.secured && !net.saved) {
      passwordPrompt = net;
      return;
    }
    connectingSsid = net.ssid;
    try {
      await invoke("wifi_connect", { ssid: net.ssid, password: null });
      await scan();
    } finally {
      connectingSsid = null;
    }
  }

  async function confirmPassword() {
    if (!passwordPrompt) return;
    connectingSsid = passwordPrompt.ssid;
    try {
      await invoke("wifi_connect", { ssid: passwordPrompt.ssid, password: passwordValue });
      await scan();
    } finally {
      connectingSsid = null;
      passwordPrompt = null;
      passwordValue = "";
    }
  }

  onMount(load);
</script>

<div class="section">
  <div class="row">
    <div>
      <h2>Wi-Fi</h2>
      <p class="muted">Wrapper nad nmcli / NetworkManager</p>
    </div>
    <button class="switch hm-focusable" class:on={enabled} on:click={toggle}>
      <span class="knob" />
    </button>
  </div>

  {#if enabled}
    <div class="list-header">
      <span class="muted">Dostępne sieci</span>
      <button class="link hm-focusable" on:click={scan} disabled={scanning}>
        {scanning ? "Skanowanie…" : "Odśwież"}
      </button>
    </div>
    <ul class="list">
      {#each networks as net}
        <li>
          <button class="net-row hm-focusable" on:click={() => connect(net)}>
            <span class="signal">{net.signal >= 66 ? "▂▄▆" : net.signal >= 33 ? "▂▄" : "▂"}</span>
            <span class="ssid">{net.ssid}</span>
            {#if net.secured}<span class="lock">🔒</span>{/if}
            {#if net.in_use}<span class="badge">Połączono</span>{/if}
            {#if connectingSsid === net.ssid}<span class="badge">Łączenie…</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if passwordPrompt}
    <div class="modal-backdrop" on:click|self={() => (passwordPrompt = null)}>
      <div class="modal">
        <h3>Hasło do sieci „{passwordPrompt.ssid}”</h3>
        <input
          type="password"
          bind:value={passwordValue}
          placeholder="Hasło"
          class="hm-focusable"
        />
        <div class="modal-actions">
          <button class="btn hm-focusable" on:click={() => (passwordPrompt = null)}>Anuluj</button>
          <button class="btn primary hm-focusable" on:click={confirmPassword}>Połącz</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  @import "../settings/shared.css";
</style>
