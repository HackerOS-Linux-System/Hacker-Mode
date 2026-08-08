import { Component, onMount, Show } from "solid-js";
import { gamesStore } from "@/stores/gamesStore";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";
import GameCard from "@/components/GameCard";
import { api } from "@/lib/tauri";

const Home: Component = () => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);

  onMount(() => {
    void gamesStore.refresh();
  });

  const recent = () => gamesStore.games().slice(0, 6);

  return (
    <div class="content">
      <div class="page-title">{t("title")}</div>

      <div class="tabs" style={{ "margin-bottom": "30px" }}>
        <button class="ghost-btn" onClick={() => api.launchStoreClient("steam")}>
          Steam
        </button>
        <button class="ghost-btn" onClick={() => api.launchStoreClient("heroic")}>
          Heroic (Epic / GOG / Amazon)
        </button>
        <button class="ghost-btn" onClick={() => api.launchStoreClient("lutris")}>
          Lutris
        </button>
      </div>

      <div class="page-title" style={{ "font-size": "18px" }}>
        {t("library")}
      </div>

      <Show
        when={!gamesStore.isLoading()}
        fallback={<div class="empty-state">…</div>}
      >
        <Show when={recent().length > 0} fallback={<div class="empty-state">{t("no_games_found")}</div>}>
          <div class="grid">
            {recent().map((game) => (
              <GameCard game={game} />
            ))}
          </div>
        </Show>
      </Show>
    </div>
  );
};

export default Home;
