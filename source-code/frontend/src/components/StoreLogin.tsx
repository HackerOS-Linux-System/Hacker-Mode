import { Component, createSignal, For, onCleanup, onMount } from "solid-js";
import { api, type Platform } from "@/lib/tauri";
import { gamesStore } from "@/stores/gamesStore";

interface StoreDef {
  id: Platform;
  label: string;
}

const STORES: StoreDef[] = [
  { id: "steam", label: "Steam" },
  { id: "epic", label: "Epic Games" },
  { id: "gog", label: "GOG" },
  { id: "amazon", label: "Amazon Games" },
  { id: "lutris", label: "Lutris" },
];

const StoreLogin: Component = () => {
  const [status, setStatus] = createSignal<Record<string, boolean>>({});
  const [pendingCode, setPendingCode] = createSignal<Platform | null>(null);
  const [codeInput, setCodeInput] = createSignal("");
  const [busy, setBusy] = createSignal<Platform | null>(null);
  const [message, setMessage] = createSignal<string | null>(null);

  async function refreshStatus() {
    const entries = await Promise.all(
      STORES.map(async (s) => [s.id, await api.storeIsLoggedIn(s.id)] as const),
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
      <For each={STORES}>
        {(store) => (
          <div class="settings-row">
            <span>
              {store.label}{" "}
              <span
                class="badge"
                style={{
                  background: status()[store.id] ? "rgba(255,59,59,0.15)" : "rgba(255,91,91,0.12)",
                  color: status()[store.id] ? "var(--accent)" : "var(--danger)",
                }}
              >
                {status()[store.id] ? "Zalogowano" : "Niezalogowano"}
              </span>
            </span>
            <button class="ghost-btn" disabled={busy() === store.id} onClick={() => startLogin(store.id)}>
              {busy() === store.id ? "…" : "Zaloguj"}
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
            <input
              type="text"
              value={codeInput()}
              onInput={(e) => setCodeInput(e.currentTarget.value)}
              placeholder="Kod autoryzacyjny"
              style={{ flex: "1", padding: "8px", "border-radius": "8px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
            />
            <button class="primary-btn" onClick={submitCode}>
              Wyślij
            </button>
          </div>
        </div>
      )}

      {message() && <div style={{ "font-size": "12px", color: "var(--text-muted)", "margin-top": "10px" }}>{message()}</div>}
    </div>
  );
};

export default StoreLogin;
