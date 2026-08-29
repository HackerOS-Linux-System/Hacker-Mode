import { Component, createMemo, onMount, Show } from "solid-js";
import { gamesStore } from "@/stores/gamesStore";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";
import GameCard from "@/components/GameCard";
import { api, PLATFORM_LABELS, type Platform } from "@/lib/tauri";

const Home: Component = () => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);

  onMount(() => {
    void gamesStore.refresh();
  });

  const recent = () => gamesStore.games().slice(0, 6);

  // Prosty pulpit — dawniej Home.tsx pokazywał tylko rząd przycisków
  // launcherów i 6 pierwszych gier z listy, bez żadnego kontekstu o
  // bibliotece jako całości. Statystyki liczone z tego, co już mamy w
  // `gamesStore` (bez dodatkowych zapytań do backendu).
  const stats = createMemo(() => {
    const games = gamesStore.games();
    const installed = games.filter((g) => g.installed);
    const totalMinutes = games.reduce((sum, g) => sum + (g.playtime_minutes ?? 0), 0);
    const byPlatform = new Map<Platform, number>();
    for (const g of games) {
      byPlatform.set(g.platform, (byPlatform.get(g.platform) ?? 0) + 1);
    }
    return {
      total: games.length,
      installed: installed.length,
      hours: Math.round(totalMinutes / 6) / 10,
      byPlatform: [...byPlatform.entries()].sort((a, b) => b[1] - a[1]),
    };
  });

  return (
    <div class="content">
      <div class="page-title">{t("title")}</div>

      <Show when={!gamesStore.isLoading() && stats().total > 0}>
        <div class="grid" style={{ "grid-template-columns": "repeat(auto-fill, minmax(160px, 1fr))", "margin-bottom": "28px" }}>
          <div class="settings-panel" style={{ "margin-bottom": "0", "text-align": "center" }}>
            <div style={{ "font-size": "28px", "font-weight": "800", color: "var(--accent)" }}>{stats().total}</div>
            <div style={{ "font-size": "12px", color: "var(--text-muted)" }}>gier w bibliotece</div>
          </div>
          <div class="settings-panel" style={{ "margin-bottom": "0", "text-align": "center" }}>
            <div style={{ "font-size": "28px", "font-weight": "800", color: "var(--accent)" }}>{stats().installed}</div>
            <div style={{ "font-size": "12px", color: "var(--text-muted)" }}>zainstalowanych</div>
          </div>
          <Show when={stats().hours > 0}>
            <div class="settings-panel" style={{ "margin-bottom": "0", "text-align": "center" }}>
              <div style={{ "font-size": "28px", "font-weight": "800", color: "var(--accent)" }}>{stats().hours}</div>
              <div style={{ "font-size": "12px", color: "var(--text-muted)" }}>godzin gry (Steam)</div>
            </div>
          </Show>
          <div class="settings-panel" style={{ "margin-bottom": "0" }}>
            <div style={{ "font-size": "11px", color: "var(--text-muted)", "margin-bottom": "6px" }}>Wg platformy</div>
            {stats().byPlatform.slice(0, 3).map(([platform, count]) => (
              <div style={{ display: "flex", "justify-content": "space-between", "font-size": "12px", padding: "2px 0" }}>
                <span>{PLATFORM_LABELS[platform]}</span>
                <span style={{ color: "var(--text-muted)" }}>{count}</span>
              </div>
            ))}
          </div>
        </div>
      </Show>

      <div class="tabs" style={{ "margin-bottom": "30px", "flex-wrap": "wrap" }}>
        <button data-focusable tabIndex={0} class="ghost-btn launcher-btn" onClick={() => api.launchStoreClient("steam")}>
          Steam
        </button>
        <button data-focusable tabIndex={0} class="ghost-btn launcher-btn" onClick={() => api.launchStoreClient("heroic")}>
          Heroic (Epic / GOG / Amazon)
        </button>
        <button data-focusable tabIndex={0} class="ghost-btn launcher-btn" onClick={() => api.launchStoreClient("lutris")}>
          Lutris
        </button>
        <button data-focusable tabIndex={0} class="ghost-btn launcher-btn" onClick={() => api.launchStoreClient("ea")}>
          EA app
        </button>
        <button data-focusable tabIndex={0} class="ghost-btn launcher-btn" onClick={() => api.launchStoreClient("battlenet")}>
          Battle.net
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
