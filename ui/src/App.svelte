<script lang="ts">
  import "./app.css";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import TopBar from "./lib/components/TopBar.svelte";
  import HackerMenu from "./lib/components/HackerMenu.svelte";
  import Library from "./lib/routes/Library.svelte";
  import Store from "./lib/routes/Store.svelte";
  import Settings from "./lib/routes/Settings.svelte";
  import { currentRoute } from "./lib/stores/nav";
</script>

<div class="shell">
  <Sidebar />

  <div class="main">
    <TopBar />
    <div class="view">
      {#if $currentRoute === "library"}
        <Library />
      {:else if $currentRoute === "store"}
        <Store />
      {:else if $currentRoute === "settings"}
        <Settings />
      {/if}
    </div>
  </div>
</div>

<HackerMenu />

<style>
  .shell {
    display: flex;
    height: 100vh;
    width: 100vw;
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .view {
    flex: 1;
    min-height: 0;
  }
</style>
