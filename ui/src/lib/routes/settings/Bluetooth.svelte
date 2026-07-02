<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  interface BtDevice {
    address: string;
    name: string;
    paired: boolean;
    connected: boolean;
    rssi: number | null;
  }

  let enabled = true;
  let devices: BtDevice[] = [];
  let scanning = false;

  async function load() {
    enabled = await invoke<boolean>("bt_is_enabled");
    if (enabled) await scan();
  }

  async function scan() {
    scanning = true;
    try {
      devices = await invoke<BtDevice[]>("bt_scan");
    } finally {
      scanning = false;
    }
  }

  async function toggle() {
    enabled = !enabled;
    await invoke("bt_set_enabled", { enabled });
    if (enabled) await scan();
  }

  async function connect(d: BtDevice) {
    await invoke("bt_connect", { address: d.address });
    await scan();
  }

  async function forget(d: BtDevice) {
    await invoke("bt_forget", { address: d.address });
    await scan();
  }

  onMount(load);
</script>

<div class="section">
  <div class="row">
    <div>
      <h2>Bluetooth</h2>
      <p class="muted">Pady, słuchawki, klawiatury (BlueZ)</p>
    </div>
    <button class="switch hm-focusable" class:on={enabled} on:click={toggle}>
      <span class="knob" />
    </button>
  </div>

  {#if enabled}
    <div class="list-header">
      <span class="muted">Urządzenia w zasięgu</span>
      <button class="link hm-focusable" on:click={scan} disabled={scanning}>
        {scanning ? "Skanowanie…" : "Odśwież"}
      </button>
    </div>
    <ul class="list">
      {#each devices as d}
        <li>
          <button
            class="device-row hm-focusable"
            on:click={() => (d.connected ? forget(d) : connect(d))}
          >
            <span class="device-name">{d.name || d.address}</span>
            {#if d.connected}<span class="badge">Połączono</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  @import "../settings/shared.css";
</style>
