<script lang="ts">
  // Sklepy otwierane WEWNĄTRZ Hacker Mode (nie w zewnętrznej przeglądarce),
  // tak jak w Heroic Games Launcher. Steam nie ma tu własnej zakładki
  // sklepu, bo gry ze Steama są importowane bezpośrednio z lokalnej
  // instalacji klienta Steam (zakładka Biblioteka).
  import StoreWebview from "../components/StoreWebview.svelte";

  const stores = [
    { id: "epic", label: "Epic Games", url: "https://store.epicgames.com" },
    { id: "gog", label: "GOG", url: "https://www.gog.com" },
    { id: "amazon", label: "Amazon Games", url: "https://gaming.amazon.com" },
  ];

  let active = stores[0];
</script>

<div class="store-page">
  <div class="tabs">
    {#each stores as s}
      <button
        class="tab hm-focusable"
        class:active={active.id === s.id}
        on:click={() => (active = s)}
      >
        {s.label}
      </button>
    {/each}
  </div>
  <div class="content">
    <StoreWebview url={active.url} title={active.label} />
  </div>
</div>

<style>
  .store-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 20px 28px 28px;
  }
  .tabs {
    display: flex;
    gap: 8px;
    margin-bottom: 14px;
  }
  .tab {
    padding: 8px 16px;
    border-radius: var(--radius-sm);
    background: var(--bg-panel-raised);
    color: var(--text-secondary);
    font-weight: 700;
    font-size: 13px;
  }
  .tab.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .content {
    flex: 1;
    min-height: 0;
  }
</style>
