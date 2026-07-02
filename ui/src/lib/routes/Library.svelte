<script lang="ts">
  import { onMount } from "svelte";
  import { games, loadingLibrary, refreshLibrary, platformLabels, type Platform } from "../stores/library";
  import GameCard from "../components/GameCard.svelte";

  const platformOrder: Platform[] = ["Steam", "Epic", "Gog", "Amazon", "Native"];
  let activeFilter: Platform | "all" = "all";

  onMount(() => {
    refreshLibrary();
  });

  $: filtered = $games.filter((g) => activeFilter === "all" || g.platform === activeFilter);
</script>

<div class="library">
  <div class="library-header">
    <div>
      <span class="eyebrow">Biblioteka</span>
      <h1>Twoje gry</h1>
    </div>
    <div class="filters">
      <button
        class="filter-chip hm-focusable"
        class:active={activeFilter === "all"}
        on:click={() => (activeFilter = "all")}
      >
        Wszystko
      </button>
      {#each platformOrder as p}
        <button
          class="filter-chip hm-focusable"
          class:active={activeFilter === p}
          on:click={() => (activeFilter = p)}
        >
          {platformLabels[p]}
        </button>
      {/each}
    </div>
  </div>

  {#if $loadingLibrary}
    <p class="hint">Wczytywanie biblioteki ze Steam / Epic / GOG / Amazon…</p>
  {:else if filtered.length === 0}
    <p class="hint">Brak gier w tej kategorii. Zainstaluj coś w zakładce Sklepy albo dodaj grę ręcznie.</p>
  {:else}
    <div class="grid">
      {#each filtered as game (game.id)}
        <GameCard {game} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .library {
    padding: 28px 32px 40px;
    height: 100%;
    overflow-y: auto;
  }
  .library-header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    margin-bottom: 24px;
    flex-wrap: wrap;
    gap: 14px;
  }
  h1 {
    margin: 4px 0 0;
    font-size: 26px;
  }
  .filters {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .filter-chip {
    padding: 8px 14px;
    border-radius: 999px;
    background: var(--bg-panel-raised);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 700;
  }
  .filter-chip.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 18px;
  }
  .hint {
    color: var(--text-muted);
  }
</style>
