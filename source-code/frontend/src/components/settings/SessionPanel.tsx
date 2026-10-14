import { Component, createSignal, For, onMount, Show } from "solid-js";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";
import { api, type Compositor } from "@/lib/tauri";

const COMPOSITORS: { id: Compositor; labelKey: "compositor_labwc" | "compositor_comphwde" }[] = [
  { id: "labwc", labelKey: "compositor_labwc" },
  { id: "comphwde", labelKey: "compositor_comphwde" },
];

/** Wybór kompozytora sesji (zapisywany w `~/.config/Hacker-Mode/config.json`
 * razem z resztą ustawień) oraz informacja, który faktycznie działa teraz. */
const SessionPanel: Component = () => {
  const t = (key: Parameters<typeof getText>[1], vars?: Record<string, string | number>) =>
    getText(settingsStore.lang(), key, vars);
  const [active, setActive] = createSignal<Compositor | null>(null);

  onMount(async () => {
    try {
      setActive(await api.getActiveCompositor());
    } catch {
      // Poza Tauri (vite dev w przeglądarce) - bez informacji o aktywnym kompozytorze.
    }
  });

  return (
    <div class="settings-panel" role="group" aria-labelledby="compositor-heading">
      <h3 id="compositor-heading">{t("compositor_label")}</h3>
      <div class="settings-row">
        <label for="compositor-select">{t("compositor_label")}</label>
        <select
          id="compositor-select"
          data-focusable
          tabIndex={0}
          value={settingsStore.settings().compositor}
          onChange={(e) => settingsStore.update({ compositor: e.currentTarget.value as Compositor })}
        >
          <For each={COMPOSITORS}>{(c) => <option value={c.id}>{t(c.labelKey)}</option>}</For>
        </select>
      </div>
      <p style={{ "font-size": "12px", color: "var(--text-muted)", margin: "6px 0 0" }}>{t("compositor_hint")}</p>
      <Show when={active()}>
        {(name) => (
          <p style={{ "font-size": "12px", color: "var(--text-muted)", margin: "4px 0 0" }} aria-live="polite">
            {t("compositor_active_now", { name: name() })}
          </p>
        )}
      </Show>
    </div>
  );
};

export default SessionPanel;
