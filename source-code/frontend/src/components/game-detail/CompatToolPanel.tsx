import { Component, createEffect, createSignal, For, Show } from "solid-js";
import { api, COMPAT_TOOL_SUPPORTED_PLATFORMS, type CompatToolOptions, type Platform } from "@/lib/tauri";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";

/** Wersja Proton (Steam) / Wine (Lutris) — patrz `COMPAT_TOOL_SUPPORTED_PLATFORMS`
 * i moduł-dokumentacja `compat_tools.rs`. Dla Steam lista opcji obejmuje
 * WYŁĄCZNIE narzędzia z `compatibilitytools.d/` (Proton-GE i podobne). */
const CompatToolPanel: Component<{ platform: Platform; id: string }> = (props) => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);
  const [options, setOptions] = createSignal<CompatToolOptions | null>(null);
  const [saving, setSaving] = createSignal(false);
  const [message, setMessage] = createSignal<string | null>(null);

  createEffect(() => {
    const platform = props.platform;
    const id = props.id;
    setOptions(null);
    setMessage(null);
    if (!COMPAT_TOOL_SUPPORTED_PLATFORMS.includes(platform)) return;
    let cancelled = false;
    api
      .getCompatToolOptions(platform, id)
      .then((o) => {
        if (!cancelled) setOptions(o);
      })
      .catch((err) => console.warn("Nie udało się pobrać opcji Proton/Wine:", err));
    return () => {
      cancelled = true;
    };
  });

  async function save(value: string) {
    setSaving(true);
    setMessage(null);
    try {
      // `warning` niepuste = zapis się udał, ale klient działał w trakcie
      // (patrz `compat_tools`); błąd (np. odmowa zapisu przy działającym
      // Steamie, domyślnie) trafia do `catch` i jest pokazany w całości.
      const warning = await api.setCompatTool(props.platform, props.id, value);
      setOptions((prev) => (prev ? { ...prev, current: value } : prev));
      if (warning) setMessage(warning);
    } catch (err) {
      setMessage(String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Show when={COMPAT_TOOL_SUPPORTED_PLATFORMS.includes(props.platform) && options()}>
      {(opts) => (
        <div class="settings-panel" role="group" aria-labelledby="compat-tool-heading" style={{ "max-width": "420px", margin: "16px 0" }}>
          <h3 id="compat-tool-heading" style={{ margin: "0 0 8px", "font-size": "13px" }}>
            {props.platform === "steam" ? t("compat_tool_panel_title_steam") : t("compat_tool_panel_title_lutris")}
          </h3>
          <Show
            when={opts().options.length > 0}
            fallback={
              <p style={{ "font-size": "11px", color: "var(--text-muted)", margin: "0" }}>
                {props.platform === "steam" ? t("compat_tool_empty_steam") : t("compat_tool_empty_lutris")}
              </p>
            }
          >
            <select
              data-focusable
              tabIndex={0}
              aria-labelledby="compat-tool-heading"
              value={opts().current ?? ""}
              disabled={saving()}
              onChange={(e) => save(e.currentTarget.value)}
              style={{ width: "100%", padding: "8px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
            >
              <option value="" disabled>
                {opts().current ?? "— wybierz —"}
              </option>
              <For each={opts().options}>{(o) => <option value={o.value}>{o.label}</option>}</For>
            </select>
          </Show>
          <Show when={message()}>
            <p role="alert" style={{ color: "var(--danger)", "font-size": "11px", margin: "8px 0 0" }}>{message()}</p>
          </Show>
        </div>
      )}
    </Show>
  );
};

export default CompatToolPanel;
