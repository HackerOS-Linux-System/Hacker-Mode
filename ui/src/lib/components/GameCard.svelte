<script lang="ts">
  import type { Game } from "../stores/library";
  import { launchGame, platformLabels } from "../stores/library";

  export let game: Game;

  let launching = false;
  async function onLaunch() {
    launching = true;
    try {
      await launchGame(game);
    } finally {
      launching = false;
    }
  }
</script>

<button class="card hm-focusable" on:click={onLaunch} disabled={launching}>
  <div class="cover">
    {#if game.cover_url}
      <img src={game.cover_url} alt={game.title} loading="lazy" />
    {:else}
      <div class="cover-fallback">{game.title.slice(0, 1)}</div>
    {/if}
    {#if launching}
      <div class="launching-badge">Uruchamianie…</div>
    {/if}
  </div>
  <div class="meta">
    <span class="title">{game.title}</span>
    <span class="platform">{platformLabels[game.platform]}</span>
  </div>
</button>

<style>
  .card {
    display: flex;
    flex-direction: column;
    width: 190px;
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--bg-panel-raised);
    text-align: left;
  }
  .cover {
    position: relative;
    width: 100%;
    aspect-ratio: 2 / 3;
    background: var(--bg-panel);
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .cover-fallback {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    font-size: 42px;
    font-weight: 800;
    color: var(--text-muted);
  }
  .launching-badge {
    position: absolute;
    inset: auto 0 0 0;
    background: rgba(0, 0, 0, 0.7);
    color: var(--accent);
    font-size: 11px;
    font-weight: 700;
    text-align: center;
    padding: 6px 0;
  }
  .meta {
    padding: 10px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .title {
    font-size: 13px;
    font-weight: 700;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .platform {
    font-size: 11px;
    color: var(--text-muted);
  }
  .card:hover {
    background: var(--bg-panel-hover);
    transform: translateY(-2px);
  }
</style>
