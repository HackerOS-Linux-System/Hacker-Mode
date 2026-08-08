import { Component, createSignal, For, onMount, Show } from "solid-js";
import { gamesStore } from "@/stores/gamesStore";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";
import GameCard from "@/components/GameCard";
import type { Platform } from "@/lib/tauri";

const PLATFORMS: { id: Platform | "all"; label: string }[] = [
  { id: "all", label: "Wszystko" },
  { id: "steam", label: "Steam" },
  { id: "epic", label: "Epic Games" },
  { id: "gog", label: "GOG" },
  { id: "amazon", label: "Amazon Games" },
  { id: "lutris", label: "Lutris" },
];

const Library: Component = () => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);
  const [tab, setTab] = createSignal<Platform | "all">("all");

  onMount(() => {
    void gamesStore.refresh();
  });

  const filtered = () =>
    tab() === "all" ? gamesStore.games() : gamesStore.gamesByPlatform(tab() as Platform);

  return (
    <div class="content">
      <div class="page-title">{t("library")}</div>

      <div class="tabs">
        <For each={PLATFORMS}>
          {(p) => (
            <button
              class={`tab ${tab() === p.id ? "active" : ""}`}
              onClick={() => setTab(p.id)}
            >
              {p.label}
            </button>
          )}
        </For>
        <button class="ghost-btn" onClick={() => gamesStore.refresh()} style={{ "margin-left": "auto" }}>
          ⟳
        </button>
      </div>

      <Show when={gamesStore.error()}>
        <div class="empty-state">{gamesStore.error()}</div>
      </Show>

      <Show
        when={!gamesStore.isLoading()}
        fallback={<div class="empty-state">…</div>}
      >
        <Show when={filtered().length > 0} fallback={<div class="empty-state">{t("no_games_found")}</div>}>
          <div class="grid">
            <For each={filtered()}>{(game) => <GameCard game={game} />}</For>
          </div>
        </Show>
      </Show>
    </div>
  );
};

export default Library;
