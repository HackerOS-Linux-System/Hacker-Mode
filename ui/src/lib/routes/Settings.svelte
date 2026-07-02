<script lang="ts">
  import Wifi from "./settings/Wifi.svelte";
  import Bluetooth from "./settings/Bluetooth.svelte";
  import Audio from "./settings/Audio.svelte";
  import Brightness from "./settings/Brightness.svelte";
  import Power from "./settings/Power.svelte";
  import Timezone from "./settings/Timezone.svelte";
  import Language from "./settings/Language.svelte";

  const tabs = [
    { id: "wifi", label: "Wi-Fi", icon: "📶", component: Wifi },
    { id: "bluetooth", label: "Bluetooth", icon: "🔵", component: Bluetooth },
    { id: "audio", label: "Dźwięk", icon: "🔊", component: Audio },
    { id: "brightness", label: "Jasność", icon: "☀", component: Brightness },
    { id: "power", label: "Zasilanie", icon: "⚡", component: Power },
    { id: "timezone", label: "Strefa czasowa", icon: "🕒", component: Timezone },
    { id: "language", label: "Język", icon: "🌐", component: Language },
  ];

  let activeId = tabs[0].id;
  $: activeTab = tabs.find((t) => t.id === activeId)!;
</script>

<div class="settings-page">
  <aside class="tab-list">
    <span class="eyebrow">Ustawienia</span>
    {#each tabs as tab}
      <button
        class="tab-item hm-focusable"
        class:active={tab.id === activeId}
        on:click={() => (activeId = tab.id)}
      >
        <span class="icon">{tab.icon}</span>
        {tab.label}
      </button>
    {/each}
  </aside>

  <div class="tab-content">
    <svelte:component this={activeTab.component} />
  </div>
</div>

<style>
  .settings-page {
    display: grid;
    grid-template-columns: 240px 1fr;
    height: 100%;
  }
  .tab-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 24px 12px;
    border-right: 1px solid rgba(255, 255, 255, 0.05);
  }
  .tab-list .eyebrow {
    padding: 0 12px 12px;
  }
  .tab-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 600;
    text-align: left;
  }
  .tab-item .icon {
    width: 18px;
    text-align: center;
  }
  .tab-item.active {
    background: var(--bg-panel-raised);
    color: var(--accent);
  }
  .tab-item:hover {
    background: var(--bg-panel-hover);
    color: var(--text-primary);
  }
  .tab-content {
    padding: 28px 32px;
    overflow-y: auto;
  }
</style>
