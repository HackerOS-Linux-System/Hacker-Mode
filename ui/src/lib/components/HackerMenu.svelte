<script lang="ts">
  // Odpowiednik menu wywoływanego przyciskiem "Steam" w Steam GamepadUI:
  // pełnoekranowa nakładka z szybkim dostępem do zasilania, ustawień
  // i powrotu do biblioteki, bez zamykania aktualnie działającej gry.
  import { invoke } from "@tauri-apps/api/core";
  import { hackerMenuOpen, currentRoute } from "../stores/nav";

  function close() {
    hackerMenuOpen.set(false);
  }

  function goSettings() {
    currentRoute.set("settings");
    close();
  }

  async function suspend() {
    await invoke("suspend_system");
  }
  async function shutdown() {
    await invoke("shutdown_system");
  }
  async function quitToDesktop() {
    await invoke("quit_to_desktop");
  }
</script>

{#if $hackerMenuOpen}
  <div class="overlay" on:click|self={close}>
    <div class="panel">
      <div class="panel-header">
        <span class="eyebrow">Hacker Menu</span>
        <button class="close hm-focusable" on:click={close} aria-label="Zamknij">✕</button>
      </div>

      <div class="grid">
        <button class="tile hm-focusable" on:click={() => currentRoute.set("library")}>
          <span class="tile-icon">🎮</span>
          Biblioteka
        </button>
        <button class="tile hm-focusable" on:click={() => currentRoute.set("store")}>
          <span class="tile-icon">🛒</span>
          Sklepy
        </button>
        <button class="tile hm-focusable" on:click={goSettings}>
          <span class="tile-icon">⚙</span>
          Ustawienia
        </button>
        <button class="tile hm-focusable" on:click={suspend}>
          <span class="tile-icon">⏾</span>
          Uśpij
        </button>
        <button class="tile hm-focusable" on:click={quitToDesktop}>
          <span class="tile-icon">↩</span>
          Wyjdź do pulpitu
        </button>
        <button class="tile hm-focusable danger" on:click={shutdown}>
          <span class="tile-icon">⏻</span>
          Wyłącz
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(6, 9, 12, 0.72);
    backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }
  .panel {
    width: min(560px, 90vw);
    background: var(--bg-panel);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: var(--radius-lg);
    padding: 24px;
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.5);
  }
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 18px;
  }
  .close {
    color: var(--text-secondary);
    font-size: 16px;
    padding: 6px;
    border-radius: var(--radius-sm);
  }
  .close:hover {
    color: var(--text-primary);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 20px 8px;
    background: var(--bg-panel-raised);
    border-radius: var(--radius-md);
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }
  .tile:hover {
    background: var(--bg-panel-hover);
  }
  .tile.danger:hover {
    color: var(--danger);
  }
  .tile-icon {
    font-size: 24px;
  }
</style>
