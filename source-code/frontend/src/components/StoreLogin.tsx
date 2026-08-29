import { Component, createSignal, For, onCleanup, onMount } from "solid-js";
import { api, PLATFORM_LABELS, type Platform } from "@/lib/tauri";
import { gamesStore } from "@/stores/gamesStore";

/** Platformy, dla których Hacker Mode zarządza logowaniem z własnego UI —
 * mają realną implementację `login_start`/`is_logged_in` po stronie Rust
 * (patrz `Platform::supports_managed_login`). */
const MANAGED_STORES: Platform[] = ["steam", "epic", "gog", "amazon"];

/** BUGFIX: Lutris/EA app/Battle.net logują się same, wewnątrz swojego
 * okna — `is_logged_in`/`login_start` po stronie Rust dla nich to
 * domyślna implementacja z trait `StoreProvider` (`is_logged_in` zawsze
 * `true`, `login_start` zawsze błąd "logowanie nie jest obsługiwane").
 * Wcześniej ten komponent traktował wszystkie platformy jednakowo —
 * Lutris zawsze pokazywał fałszywy zielony badge "Zalogowano", a kliknięcie
 * "Zaloguj" zawsze kończyło się czerwonym błędem. Rozdzielamy więc listę na
 * dwie sekcje zamiast pokazywać złudny/zepsuty UI dla tych platform. */
const SELF_MANAGED_STORES: Platform[] = ["lutris", "ea", "battlenet"];

const StoreLogin: Component = () => {
  const [status, setStatus] = createSignal<Record<string, boolean>>({});
  const [pendingCode, setPendingCode] = createSignal<Platform | null>(null);
  const [codeInput, setCodeInput] = createSignal("");
  const [busy, setBusy] = createSignal<Platform | null>(null);
  const [message, setMessage] = createSignal<string | null>(null);

  async function refreshStatus() {
    const entries = await Promise.all(
      MANAGED_STORES.map(async (id) => [id, await api.storeIsLoggedIn(id)] as const),
    );
    setStatus(Object.fromEntries(entries));
  }

  onMount(() => {
    void refreshStatus();
    const unlistenPromise = api.onStoreLoginFinished((platform, ok) => {
      setBusy(null);
      setPendingCode(null);
      setMessage(ok ? `Zalogowano do ${platform}.` : `Logowanie do ${platform} nie powiodło się.`);
      void refreshStatus();
      void gamesStore.refresh();
    });
    onCleanup(() => {
      void unlistenPromise.then((unlisten) => unlisten());
    });
  });

  async function startLogin(platform: Platform) {
    setBusy(platform);
    setMessage(null);
    try {
      const flow = await api.storeLoginStart(platform);
      if (flow.url) {
        await api.openStoreLoginWindow(platform);
        if (flow.needs_code) {
          setPendingCode(platform);
        }
      } else {
        setBusy(null);
      }
      setMessage(flow.instructions);
    } catch (err) {
      setMessage(String(err));
      setBusy(null);
    }
  }

  async function submitCode() {
    const platform = pendingCode();
    if (!platform || !codeInput().trim()) return;
    try {
      await api.submitStoreLoginCode(platform, codeInput().trim());
      setCodeInput("");
    } catch (err) {
      setMessage(String(err));
    }
  }

  return (
    <div class="settings-panel">
      <h3>Sklepy — logowanie</h3>
      <For each={MANAGED_STORES}>
        {(id) => (
          <div class="settings-row">
            <span>
              {PLATFORM_LABELS[id]}{" "}
              <span
                class="badge"
                style={{
                  background: status()[id] ? "rgba(255,59,59,0.15)" : "rgba(255,91,91,0.12)",
                  color: status()[id] ? "var(--accent)" : "var(--danger)",
                }}
              >
                {status()[id] ? "Zalogowano" : "Niezalogowano"}
              </span>
            </span>
            <button data-focusable tabIndex={0} class="ghost-btn" disabled={busy() === id} onClick={() => startLogin(id)}>
              {busy() === id ? "…" : "Zaloguj"}
            </button>
          </div>
        )}
      </For>

      {pendingCode() && (
        <div class="settings-row" style={{ "flex-direction": "column", "align-items": "stretch", gap: "8px" }}>
          <span style={{ "font-size": "12px", color: "var(--text-muted)" }}>
            Jeśli okno logowania nie zamknęło się automatycznie, wklej tu skopiowany kod:
          </span>
          <div style={{ display: "flex", gap: "8px" }}>
            <input data-focusable tabIndex={0}
              type="text"
              value={codeInput()}
              onInput={(e) => setCodeInput(e.currentTarget.value)}
              placeholder="Kod autoryzacyjny"
              style={{ flex: "1", padding: "8px", "border-radius": "8px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
            />
            <button data-focusable tabIndex={0} class="primary-btn" onClick={submitCode}>
              Wyślij
            </button>
          </div>
        </div>
      )}

      {message() && <div style={{ "font-size": "12px", color: "var(--text-muted)", "margin-top": "10px" }}>{message()}</div>}

      <div style={{ "margin-top": "16px", "padding-top": "12px", "border-top": "1px solid rgba(255,255,255,0.08)" }}>
        <span style={{ "font-size": "11px", color: "var(--text-muted)", display: "block", "margin-bottom": "8px" }}>
          Te platformy logują się same, z poziomu własnego okna — Hacker Mode tylko je uruchamia:
        </span>
        <For each={SELF_MANAGED_STORES}>
          {(id) => (
            <div class="settings-row">
              <span>{PLATFORM_LABELS[id]}</span>
              <span style={{ "font-size": "11px", color: "var(--text-muted)" }}>zarządzane wewnętrznie</span>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};

export default StoreLogin;
