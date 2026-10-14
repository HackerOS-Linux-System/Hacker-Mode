import { Component, createSignal, For, onMount, Show } from "solid-js";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";
import { api, type FormatCheck, type ToolCheck } from "@/lib/tauri";

/** Lista zewnętrznych narzędzi (z podpowiedzią instalacji dla brakujących)
 * oraz przycisk uruchamiający TYLKO-DO-ODCZYTU sprawdzenie, czy parsery
 * czytają prawdziwe pliki Steam/Lutris/Heroic/Bottles na tej maszynie. */
const DependenciesPanel: Component = () => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);
  const [tools, setTools] = createSignal<ToolCheck[]>([]);
  const [formats, setFormats] = createSignal<FormatCheck[] | null>(null);
  const [busy, setBusy] = createSignal(false);

  async function loadTools() {
    try {
      setTools(await api.checkDependencies());
    } catch (err) {
      console.warn("Nie udało się sprawdzić zależności:", err);
    }
  }

  async function runFormats() {
    setBusy(true);
    try {
      setFormats(await api.verifyExternalFormats());
    } catch (err) {
      console.warn("Nie udało się sprawdzić formatów plików:", err);
    } finally {
      setBusy(false);
    }
  }

  onMount(() => void loadTools());

  const statusText = (tool: ToolCheck) =>
    tool.found ? t("dependency_found") : tool.critical ? `${t("dependency_missing")} (${t("dependency_required")})` : t("dependency_missing");

  return (
    <>
      <div class="settings-panel" role="group" aria-labelledby="deps-heading">
        <h3 id="deps-heading">{t("dependencies_panel_title")}</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", margin: "0 0 10px" }}>{t("dependencies_panel_body")}</p>
        <ul style={{ "list-style": "none", padding: 0, margin: 0 }}>
          <For each={tools()}>
            {(tool) => (
              <li class="settings-row" style={{ "flex-wrap": "wrap" }}>
                <span>
                  <code>{tool.id}</code> — {tool.purpose}
                </span>
                <strong
                  role="status"
                  style={{ color: tool.found ? "var(--accent, #4caf50)" : tool.critical ? "#e57373" : "var(--text-muted)" }}
                >
                  {statusText(tool)}
                </strong>
                <Show when={!tool.found}>
                  <small style={{ width: "100%", color: "var(--text-muted)" }}>{tool.hint}</small>
                </Show>
              </li>
            )}
          </For>
        </ul>
        <button data-focusable tabIndex={0} onClick={() => void loadTools()}>
          {t("dependencies_recheck")}
        </button>
      </div>

      <div class="settings-panel" role="group" aria-labelledby="formats-heading">
        <h3 id="formats-heading">{t("formats_panel_title")}</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", margin: "0 0 10px" }}>{t("formats_panel_body")}</p>
        <button data-focusable tabIndex={0} disabled={busy()} aria-busy={busy()} onClick={() => void runFormats()}>
          {busy() ? t("formats_running") : t("formats_run")}
        </button>
        <Show when={formats()}>
          {(list) => (
            <ul style={{ "list-style": "none", padding: 0, margin: "10px 0 0" }} aria-live="polite">
              <For each={list()}>
                {(f) => (
                  <li class="settings-row" style={{ "flex-wrap": "wrap" }}>
                    <span>
                      {f.ok === true ? "✔" : f.ok === false ? "✖" : "–"} {f.name}
                    </span>
                    <small style={{ width: "100%", color: "var(--text-muted)" }}>{f.detail}</small>
                  </li>
                )}
              </For>
            </ul>
          )}
        </Show>
      </div>
    </>
  );
};

export default DependenciesPanel;
