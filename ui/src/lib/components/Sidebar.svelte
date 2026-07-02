<script lang="ts">
  import { currentRoute, type Route } from "../stores/nav";

  const items: { id: Route; label: string; icon: string }[] = [
    { id: "library", label: "Biblioteka", icon: "🎮" },
    { id: "store", label: "Sklepy", icon: "🛒" },
    { id: "settings", label: "Ustawienia", icon: "⚙" },
  ];
</script>

<nav class="sidebar">
  <div class="brand" aria-hidden="true">HM</div>
  <ul>
    {#each items as item}
      <li>
        <button
          class="nav-item hm-focusable"
          class:active={$currentRoute === item.id}
          on:click={() => currentRoute.set(item.id)}
          aria-label={item.label}
        >
          <span class="icon">{item.icon}</span>
          <span class="label">{item.label}</span>
        </button>
      </li>
    {/each}
  </ul>
</nav>

<style>
  .sidebar {
    width: var(--sidebar-w);
    height: 100%;
    background: var(--bg-panel);
    border-right: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 20px 0;
  }
  .brand {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-md);
    background: var(--accent-soft);
    color: var(--accent);
    display: grid;
    place-items: center;
    font-weight: 800;
    font-size: 13px;
    margin-bottom: 28px;
    letter-spacing: 0.02em;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
  }
  .nav-item {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 4px;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
  }
  .nav-item .icon {
    font-size: 20px;
  }
  .nav-item .label {
    font-size: 10px;
    font-weight: 600;
  }
  .nav-item.active {
    background: var(--bg-panel-raised);
    color: var(--accent);
  }
  .nav-item:hover {
    background: var(--bg-panel-hover);
    color: var(--text-primary);
  }
</style>
