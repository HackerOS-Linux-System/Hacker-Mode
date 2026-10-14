import { Component, createEffect, createSignal, For, Show } from "solid-js";
import { api, PROTONDB_SUPPORTED_PLATFORMS, type Achievement, type Platform } from "@/lib/tauri";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";

/** Osiągnięcia Steam — wymagają klucza Steam Web API + SteamID64 (Ustawienia →
 * Steam). Panel pokazuje się tylko, gdy faktycznie coś dostaliśmy: brak
 * klucza, prywatny profil i gra bez osiągnięć kończą się tym samym (`null`). */
const AchievementsPanel: Component<{ platform: Platform; id: string }> = (props) => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);
  const [achievements, setAchievements] = createSignal<Achievement[] | null>(null);
  const [loading, setLoading] = createSignal(false);

  createEffect(() => {
    const platform = props.platform;
    const id = props.id;
    setAchievements(null);
    if (!PROTONDB_SUPPORTED_PLATFORMS.includes(platform)) return; // te same warunki co Steam Web API
    const s = settingsStore.settings();
    if (!(s.steam_api_key && s.steam_id64)) return;
    let cancelled = false;
    setLoading(true);
    api
      .fetchSteamAchievements(id)
      .then((list) => {
        if (!cancelled) setAchievements(list.length > 0 ? list : null);
      })
      .catch((err) => console.warn("Nie udało się pobrać osiągnięć Steam:", err))
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  });

  return (
    <>
      <Show when={loading()}>
        <div role="status" style={{ "font-size": "11px", color: "var(--text-muted)", margin: "8px 0" }}>
          {t("achievements_loading")}
        </div>
      </Show>
      <Show when={achievements()}>
        {(list) => {
          const unlocked = () => list().filter((a) => a.achieved).length;
          return (
            <div class="settings-panel" role="group" aria-labelledby="achievements-heading" style={{ "max-width": "600px", margin: "16px 0" }}>
              <h3 id="achievements-heading" style={{ margin: "0 0 10px", "font-size": "13px" }}>
                {t("achievements_title")} ({unlocked()}/{list().length})
              </h3>
              <div style={{ display: "grid", "grid-template-columns": "repeat(auto-fill, minmax(48px, 1fr))", gap: "8px" }}>
                <For each={list()}>
                  {(a) => (
                    <img
                      src={a.icon_url}
                      alt={a.display_name}
                      title={`${a.display_name}${a.description ? " — " + a.description : ""}`}
                      style={{ width: "48px", height: "48px", "border-radius": "6px", opacity: a.achieved ? "1" : "0.35" }}
                    />
                  )}
                </For>
              </div>
            </div>
          );
        }}
      </Show>
    </>
  );
};

export default AchievementsPanel;
