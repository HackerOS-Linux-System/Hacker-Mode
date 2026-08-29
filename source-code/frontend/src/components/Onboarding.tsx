import { Component, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { api, type LauncherToolStatus } from "@/lib/tauri";
import { settingsStore } from "@/stores/settingsStore";
import { getText, LANGUAGES, type Lang } from "@/i18n";

/**
 * Ekran pokazywany zamiast normalnego UI przy pierwszym uruchomieniu Hacker
 * Mode (patrz `first_run.rs` po stronie backendu — sprawdzane przez
 * `api.isFirstRun()`, zamykane przez `api.markFirstRunComplete()`, co
 * zapisuje `~/.hackeros/Hacker-Mode/.no-first-time`, żeby ten ekran nie
 * pokazał się ponownie).
 *
 * Trzy kroki: powitanie/wybór języka → instalacja narzędzi CLI do
 * venv (`legendary`/`gogdl`/`nile`, patrz `first_run.rs`) → gotowe.
 * Steam jest tu celowo pominięty (patrz moduł-dokumentacja backendu) —
 * ten ekran o nim w ogóle nie wspomina poza jednym zdaniem wyjaśnienia.
 */
const Onboarding: Component<{ onDone: () => void }> = (props) => {
  const [step, setStep] = createSignal<"welcome" | "tools" | "done">("welcome");
  const [tools, setTools] = createSignal<LauncherToolStatus[]>([]);
  const [installing, setInstalling] = createSignal(false);
  const [log, setLog] = createSignal<string[]>([]);
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);

  onMount(async () => {
    setTools(await api.firstRunToolStatus());
    const unlistenProgress = await api.onFirstRunProgress((line) => {
      setLog((prev) => [...prev.slice(-30), line]);
    });
    const unlistenFinished = await api.onFirstRunFinished(async (ok) => {
      setInstalling(false);
      setTools(await api.firstRunToolStatus());
      setLog((prev) => [...prev, ok ? "Gotowe." : "Część narzędzi mogła się nie zainstalować — możesz to zrobić później ręcznie."]);
    });
    onCleanup(() => {
      void unlistenProgress();
      void unlistenFinished();
    });
  });

  async function startInstall() {
    setInstalling(true);
    setLog([]);
    await api.installLauncherTools();
  }

  async function finish() {
    await api.markFirstRunComplete();
    props.onDone();
  }

  return (
    <div
      style={{
        position: "fixed",
        inset: "0",
        background: "var(--bg)",
        color: "var(--text)",
        display: "flex",
        "align-items": "center",
        "justify-content": "center",
        "z-index": "1000",
        padding: "40px",
      }}
    >
      <div style={{ "max-width": "560px", width: "100%" }}>
        <Show when={step() === "welcome"}>
          <h1 style={{ "font-size": "32px", "margin-bottom": "4px" }}>{t("onboarding_welcome_title")}</h1>
          <p style={{ color: "var(--text-muted)", "line-height": "1.5" }}>{t("onboarding_welcome_body")}</p>

          <div class="settings-panel" style={{ "margin-top": "24px" }}>
            <h3>{t("onboarding_language_step")}</h3>
            <div class="settings-row">
              <span>{t("language")}</span>
              <select
                data-focusable
                tabIndex={0}
                value={settingsStore.lang()}
                onChange={(e) => settingsStore.update({ language: e.currentTarget.value as Lang })}
              >
                <For each={LANGUAGES}>{(l) => <option value={l.id}>{l.label}</option>}</For>
              </select>
            </div>
          </div>

          <button
            class="primary-btn"
            data-focusable
            tabIndex={0}
            style={{ "margin-top": "24px", width: "100%" }}
            onClick={() => setStep("tools")}
          >
            →
          </button>
        </Show>

        <Show when={step() === "tools"}>
          <h1 style={{ "font-size": "26px", "margin-bottom": "4px" }}>{t("onboarding_tools_title")}</h1>
          <p style={{ color: "var(--text-muted)", "line-height": "1.5" }}>{t("onboarding_tools_body")}</p>
          <p style={{ color: "var(--text-muted)", "font-size": "13px", "font-style": "italic" }}>{t("onboarding_steam_note")}</p>

          <div class="settings-panel" style={{ "margin-top": "16px" }}>
            <For each={tools()}>
              {(tool) => (
                <div class="settings-row">
                  <span>{tool.label}</span>
                  <span style={{ color: tool.available ? "var(--accent)" : "var(--text-muted)", "font-size": "12px" }}>
                    {tool.available ? t("onboarding_already_available") : "—"}
                  </span>
                </div>
              )}
            </For>
          </div>

          <Show when={log().length > 0}>
            <div
              class="settings-panel"
              style={{
                "margin-top": "12px",
                "max-height": "140px",
                "overflow-y": "auto",
                "font-family": "monospace",
                "font-size": "11px",
                color: "var(--text-muted)",
              }}
            >
              <For each={log()}>{(line) => <div>{line}</div>}</For>
            </div>
          </Show>

          <div style={{ display: "flex", gap: "10px", "margin-top": "20px" }}>
            <button class="primary-btn" data-focusable tabIndex={0} style={{ flex: "1" }} disabled={installing()} onClick={startInstall}>
              {installing() ? t("onboarding_installing") : t("onboarding_install")}
            </button>
            <button class="ghost-btn" data-focusable tabIndex={0} onClick={() => setStep("done")}>
              {t("onboarding_skip")}
            </button>
          </div>

          <Show when={!installing() && log().length > 0}>
            <button class="primary-btn" data-focusable tabIndex={0} style={{ "margin-top": "12px", width: "100%" }} onClick={() => setStep("done")}>
              →
            </button>
          </Show>
        </Show>

        <Show when={step() === "done"}>
          <h1 style={{ "font-size": "28px" }}>{t("onboarding_welcome_title")} 🎮</h1>
          <button class="primary-btn" data-focusable tabIndex={0} style={{ "margin-top": "20px", width: "100%" }} onClick={finish}>
            {t("onboarding_finish")}
          </button>
        </Show>
      </div>
    </div>
  );
};

export default Onboarding;
