import { Component, createMemo, createSignal, For, onMount, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { gamesStore } from "@/stores/gamesStore";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";
import { api, resolveCoverSrc, PLATFORM_LABELS, type Platform } from "@/lib/tauri";

/** Widok statystyk biblioteki — WYŁĄCZNIE agregacja po stronie frontendu
 * (patrz `commands::get_playtime_last_played`, moduł-dokumentacja
 * `playtime.rs`): łączymy `gamesStore.games()` (który już ma
 * `playtime_minutes` scalony ze WSZYSTKICH źródeł — Steam Web API, baza
 * Lutrisa, lokalny licznik Hacker Mode, patrz `playtime::merge_into`) z
 * jedną dodatkową mapą znaczników "ostatnio grane" z tego samego lokalnego
 * licznika. Backend celowo nie liczy tu niczego sam — patrz komentarz przy
 * `get_playtime_last_played` po uzasadnienie.
 *
 * WAŻNE ograniczenie (patrz też moduł-dokumentacja `playtime.rs`):
 * "ostatnio grane" pochodzi WYŁĄCZNIE z lokalnego licznika Hacker Mode,
 * więc gry uruchamiane z pominięciem Hacker Mode (np. bezpośrednio z
 * osobno otwartego klienta Steam) nigdy się tu nie pojawią jako "ostatnio
 * grane", nawet jeśli realnie były grane wczoraj — Hacker Mode po prostu
 * nie ma jak się o tym dowiedzieć. */
const Stats: Component = () => {
  const navigate = useNavigate();
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);
  const [lastPlayed, setLastPlayed] = createSignal<Record<string, number>>({});

  onMount(async () => {
    if (gamesStore.games().length === 0) void gamesStore.refresh();
    try {
      setLastPlayed(await api.getPlaytimeLastPlayed());
    } catch (err) {
      console.warn("Nie udało się pobrać znaczników czasu ostatniej sesji:", err);
    }
  });

  function selectionKey(platform: Platform, id: string) {
    return `${platform}:${id}`;
  }

  const totalMinutes = createMemo(() => gamesStore.games().reduce((sum, g) => sum + (g.playtime_minutes ?? 0), 0));

  const mostPlayed = createMemo(() =>
    [...gamesStore.games()]
      .filter((g) => (g.playtime_minutes ?? 0) > 0)
      .sort((a, b) => (b.playtime_minutes ?? 0) - (a.playtime_minutes ?? 0))
      .slice(0, 5)
  );

  const recentlyPlayed = createMemo(() => {
    const marks = lastPlayed();
    return [...gamesStore.games()]
      .map((g) => ({ game: g, at: marks[selectionKey(g.platform, g.id)] ?? 0 }))
      .filter((e) => e.at > 0)
      .sort((a, b) => b.at - a.at)
      .slice(0, 8);
  });

  const byPlatform = createMemo(() => {
    const counts = new Map<Platform, { installed: number; total: number; minutes: number }>();
    for (const g of gamesStore.games()) {
      const entry = counts.get(g.platform) ?? { installed: 0, total: 0, minutes: 0 };
      entry.total += 1;
      if (g.installed) entry.installed += 1;
      entry.minutes += g.playtime_minutes ?? 0;
      counts.set(g.platform, entry);
    }
    return Array.from(counts.entries()).sort((a, b) => b[1].minutes - a[1].minutes);
  });

  const maxPlatformMinutes = createMemo(() => Math.max(1, ...byPlatform().map(([, v]) => v.minutes)));

  function formatHours(minutes: number): string {
    return `${Math.round((minutes / 60) * 10) / 10} godz.`;
  }

  function formatRelativeTime(unixSeconds: number): string {
    const diffMs = Date.now() - unixSeconds * 1000;
    const diffHours = diffMs / (1000 * 60 * 60);
    if (diffHours < 1) return "przed chwilą";
    if (diffHours < 24) return `${Math.round(diffHours)} godz. temu`;
    const diffDays = Math.round(diffHours / 24);
    if (diffDays < 30) return `${diffDays} dni temu`;
    return new Date(unixSeconds * 1000).toLocaleDateString();
  }

  return (
    <div class="content">
      <div class="page-title">{t("stats")}</div>

      <Show
        when={gamesStore.games().length > 0}
        fallback={<div class="empty-state">Biblioteka jest jeszcze pusta — nie ma czego zliczać.</div>}
      >
        <div style={{ display: "grid", "grid-template-columns": "repeat(auto-fit, minmax(160px, 1fr))", gap: "12px", "margin-bottom": "24px" }}>
          <div class="settings-panel">
            <div style={{ "font-size": "11px", color: "var(--text-muted)" }}>Łączny czas gry</div>
            <div style={{ "font-size": "24px", "font-weight": "600" }}>{formatHours(totalMinutes())}</div>
          </div>
          <div class="settings-panel">
            <div style={{ "font-size": "11px", color: "var(--text-muted)" }}>Gry w bibliotece</div>
            <div style={{ "font-size": "24px", "font-weight": "600" }}>{gamesStore.games().length}</div>
          </div>
          <div class="settings-panel">
            <div style={{ "font-size": "11px", color: "var(--text-muted)" }}>Zainstalowane</div>
            <div style={{ "font-size": "24px", "font-weight": "600" }}>
              {gamesStore.games().filter((g) => g.installed).length}
            </div>
          </div>
        </div>

        <div style={{ display: "grid", "grid-template-columns": "1fr 1fr", gap: "20px" }}>
          <div>
            <h3 style={{ "font-size": "14px", "margin-bottom": "10px" }}>Najczęściej grane</h3>
            <Show when={mostPlayed().length > 0} fallback={<p style={{ color: "var(--text-muted)", "font-size": "12px" }}>Brak zmierzonego czasu gry.</p>}>
              <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
                <For each={mostPlayed()}>
                  {(g) => (
                    <button
                      data-focusable tabIndex={0}
                      onClick={() => navigate(`/game/${g.platform}/${g.id}`)}
                      style={{
                        display: "flex", "align-items": "center", gap: "10px", padding: "6px",
                        "border-radius": "8px", background: "var(--bg-card)", border: "none",
                        color: "var(--text)", cursor: "pointer", "text-align": "left",
                      }}
                    >
                      {g.cover_path && (
                        <img src={resolveCoverSrc(g.cover_path)} alt="" style={{ width: "32px", height: "42px", "object-fit": "cover", "border-radius": "4px" }} />
                      )}
                      <div style={{ flex: "1" }}>
                        <div style={{ "font-size": "13px" }}>{g.title}</div>
                        <div style={{ "font-size": "11px", color: "var(--text-muted)" }}>
                          {PLATFORM_LABELS[g.platform]} · {formatHours(g.playtime_minutes ?? 0)}
                        </div>
                      </div>
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </div>

          <div>
            <h3 style={{ "font-size": "14px", "margin-bottom": "10px" }}>Ostatnio grane</h3>
            <Show
              when={recentlyPlayed().length > 0}
              fallback={
                <p style={{ color: "var(--text-muted)", "font-size": "12px" }}>
                  Brak sesji zarejestrowanych przez Hacker Mode jeszcze.
                </p>
              }
            >
              <div style={{ display: "flex", "flex-direction": "column", gap: "8px" }}>
                <For each={recentlyPlayed()}>
                  {(entry) => (
                    <button
                      data-focusable tabIndex={0}
                      onClick={() => navigate(`/game/${entry.game.platform}/${entry.game.id}`)}
                      style={{
                        display: "flex", "align-items": "center", gap: "10px", padding: "6px",
                        "border-radius": "8px", background: "var(--bg-card)", border: "none",
                        color: "var(--text)", cursor: "pointer", "text-align": "left",
                      }}
                    >
                      {entry.game.cover_path && (
                        <img src={resolveCoverSrc(entry.game.cover_path)} alt="" style={{ width: "32px", height: "42px", "object-fit": "cover", "border-radius": "4px" }} />
                      )}
                      <div style={{ flex: "1" }}>
                        <div style={{ "font-size": "13px" }}>{entry.game.title}</div>
                        <div style={{ "font-size": "11px", color: "var(--text-muted)" }}>{formatRelativeTime(entry.at)}</div>
                      </div>
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </div>
        </div>

        <h3 style={{ "font-size": "14px", "margin": "24px 0 10px" }}>Podział na platformy</h3>
        <div style={{ display: "flex", "flex-direction": "column", gap: "10px" }}>
          <For each={byPlatform()}>
            {([platform, v]) => (
              <div>
                <div style={{ display: "flex", "justify-content": "space-between", "font-size": "12px", "margin-bottom": "4px" }}>
                  <span>
                    {PLATFORM_LABELS[platform]} — {v.installed}/{v.total} zainstalowanych
                  </span>
                  <span style={{ color: "var(--text-muted)" }}>{formatHours(v.minutes)}</span>
                </div>
                <div style={{ height: "6px", background: "rgba(255,255,255,0.08)", "border-radius": "999px", overflow: "hidden" }}>
                  <div style={{ height: "100%", width: `${(v.minutes / maxPlatformMinutes()) * 100}%`, background: "var(--accent)" }} />
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default Stats;
