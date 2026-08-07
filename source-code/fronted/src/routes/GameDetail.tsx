import { Component, createSignal, onMount, Show } from "solid-js";
import { useNavigate, useParams } from "@solidjs/router";
import { api, type GameDetails, type Platform } from "@/lib/tauri";
import { gamesStore } from "@/stores/gamesStore";

const GameDetail: Component = () => {
  const params = useParams<{ platform: Platform; id: string }>();
  const navigate = useNavigate();
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal<string | null>(null);
  const [details, setDetails] = createSignal<GameDetails | null>(null);
  const [detailsLoading, setDetailsLoading] = createSignal(true);

  const game = () => gamesStore.games().find((g) => g.platform === params.platform && g.id === params.id);

  onMount(async () => {
    try {
      const result = await api.fetchGameDetails(params.platform, params.id);
      setDetails(result);
    } catch (err) {
      console.warn("Nie udało się pobrać opisu gry:", err);
    } finally {
      setDetailsLoading(false);
    }
  });

  async function run(action: "launch" | "install" | "uninstall") {
    const g = game();
    if (!g) return;
    setBusy(true);
    setMessage(null);
    try {
      if (action === "launch") await api.launchGame(g.platform, g.id);
      if (action === "install") await api.installGame(g.platform, g.id);
      if (action === "uninstall") {
        await api.uninstallGame(g.platform, g.id);
        await gamesStore.refresh();
      }
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div class="content">
      <button class="ghost-btn" onClick={() => navigate(-1)} style={{ "margin-bottom": "20px" }}>
        ← Wróć
      </button>

      <Show when={game()} fallback={<div class="empty-state">Nie znaleziono gry (może biblioteka jeszcze się ładuje?).</div>}>
        {(g) => (
          <div style={{ display: "grid", "grid-template-columns": "260px 1fr", gap: "32px" }}>
            <div class="cover" style={{ height: "360px" }}>
              <Show when={g().cover_path} fallback={<span>{g().title}</span>}>
                <img
                  src={g().cover_path!}
                  alt={g().title}
                  style={{ width: "100%", height: "100%", "object-fit": "cover", "border-radius": "10px" }}
                />
              </Show>
            </div>

            <div>
              <span class="badge">{g().platform}</span>
              <h1 style={{ margin: "10px 0" }}>{g().title}</h1>

              {/* Opis/zrzuty ekranu dociągane z publicznego Steam Store API —
                  dla Epic/GOG/Amazon `details()` pozostanie `null`, patrz
                  `commands::stores::fetch_game_details`. */}
              <Show
                when={!detailsLoading()}
                fallback={<p style={{ color: "var(--text-muted)" }}>Ładowanie opisu…</p>}
              >
                <Show
                  when={details()}
                  fallback={
                    <p style={{ color: "var(--text-muted)", "max-width": "520px" }}>
                      {g().install_dir ?? "Brak informacji o katalogu instalacji."}
                    </p>
                  }
                >
                  {(d) => (
                    <>
                      <p style={{ color: "var(--text-muted)", "max-width": "600px" }}>{d().description}</p>
                      <Show when={d().screenshots.length > 0}>
                        <div style={{ display: "flex", gap: "10px", "overflow-x": "auto", "padding-bottom": "8px" }}>
                          {d().screenshots.map((src) => (
                            <img
                              src={src}
                              alt=""
                              style={{ height: "90px", "border-radius": "8px", flex: "0 0 auto" }}
                            />
                          ))}
                        </div>
                      </Show>
                    </>
                  )}
                </Show>
              </Show>

              <div style={{ display: "flex", gap: "10px", "margin-top": "20px" }}>
                <Show
                  when={g().installed}
                  fallback={
                    <button class="primary-btn" disabled={busy()} onClick={() => run("install")}>
                      ⬇ Zainstaluj
                    </button>
                  }
                >
                  <button class="primary-btn" disabled={busy()} onClick={() => run("launch")}>
                    ▶ Uruchom
                  </button>
                  <button class="ghost-btn" disabled={busy()} onClick={() => run("uninstall")} style={{ color: "var(--danger)" }}>
                    🗑 Odinstaluj
                  </button>
                </Show>
              </div>

              {message() && <div style={{ color: "var(--danger)", "margin-top": "14px" }}>{message()}</div>}
            </div>
          </div>
        )}
      </Show>
    </div>
  );
};

export default GameDetail;
