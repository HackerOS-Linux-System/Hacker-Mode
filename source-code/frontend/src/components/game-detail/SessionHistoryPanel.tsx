import { Component, createEffect, createSignal, For, Show } from "solid-js";
import { api, type Platform, type SessionEntry } from "@/lib/tauri";

/** Historia sesji — patrz `playtime::get_session_history`. Dostępna dla
 * wszystkich platform (lokalny licznik Hacker Mode, niezależny od sklepu);
 * obejmuje TYLKO sesje uruchomione przez Hacker Mode (patrz `playtime.rs`). */
const SessionHistoryPanel: Component<{ platform: Platform; id: string }> = (props) => {
  const [history, setHistory] = createSignal<SessionEntry[]>([]);

  createEffect(() => {
    const platform = props.platform;
    const id = props.id;
    setHistory([]);
    let cancelled = false;
    api
      .getGameSessionHistory(platform, id)
      .then((h) => {
        if (!cancelled) setHistory(h);
      })
      .catch((err) => console.warn("Nie udało się pobrać historii sesji:", err));
    return () => {
      cancelled = true;
    };
  });

  return (
    <Show when={history().length > 0}>
      <div class="settings-panel" role="group" aria-labelledby="session-history-heading" style={{ "max-width": "420px", margin: "16px 0" }}>
        <h3 id="session-history-heading" style={{ margin: "0 0 8px", "font-size": "13px" }}>Historia sesji</h3>
        <ul style={{ display: "flex", "flex-direction": "column", gap: "4px", "max-height": "140px", "overflow-y": "auto", "list-style": "none", padding: 0, margin: 0 }}>
          <For each={history()}>
            {(s) => (
              <li style={{ display: "flex", "justify-content": "space-between", "font-size": "11px", color: "var(--text-muted)" }}>
                <span>{new Date(s.at * 1000).toLocaleString()}</span>
                <span>{s.minutes} min</span>
              </li>
            )}
          </For>
        </ul>
      </div>
    </Show>
  );
};

export default SessionHistoryPanel;
