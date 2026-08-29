import { Component, createSignal, onCleanup, onMount, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { api, resolveCoverSrc, PLATFORM_LABELS, RELIABLE_RUNNING_STATE_PLATFORMS, type Game } from "@/lib/tauri";
import { useFocusable } from "@/lib/focusNav";
import { IconDownload, IconPlay } from "@/components/Icons";

/** Sesja krótsza niż to (sekundy) po starcie sugeruje, że gra się nie
 * uruchomiła poprawnie (crash przy starcie), a nie że użytkownik po
 * prostu szybko skończył grać — patrz `onGameExited` w `onMount`. Czysto
 * heurystyczne (niektóre gry/menu launcherów faktycznie zamykają się
 * szybciej), stąd tylko OSTRZEŻENIE, nie twarde stwierdzenie błędu. */
const CRASH_THRESHOLD_SECONDS = 8;

const GameCard: Component<{
  game: Game;
  selectMode?: boolean;
  selected?: boolean;
  onToggleSelect?: () => void;
}> = (props) => {
  const navigate = useNavigate();
  const [launching, setLaunching] = createSignal(false);
  const [installing, setInstalling] = createSignal(false);
  const [progressLine, setProgressLine] = createSignal<string | null>(null);
  const [progressPercent, setProgressPercent] = createSignal<number | null>(null);
  const [errorMsg, setErrorMsg] = createSignal<string | null>(null);

  // Stan "w trakcie gry" — patrz `RELIABLE_RUNNING_STATE_PLATFORMS`
  // (Steam WYŁĄCZONY: proces `steam -applaunch` kończy się niemal
  // natychmiast, gdy Steam już działał, więc ten stan byłby dla Steam
  // aktywnie MYLĄCY — migałby i gasł, mimo że gra dalej działa).
  const trackable = () => RELIABLE_RUNNING_STATE_PLATFORMS.includes(props.game.platform);
  const [running, setRunning] = createSignal(false);
  const [stopping, setStopping] = createSignal(false);
  // Ustawiane 5s po kliknięciu zwykłego "Zatrzymaj" (SIGTERM), jeśli gra
  // WCIĄŻ nie odpowiedziała — wtedy pokazujemy "Wymuś zamknięcie"
  // (SIGKILL) zamiast dalej czekać w nieskończoność na coś, co może się
  // nigdy nie zdarzyć (proces zawieszony/ignorujący SIGTERM).
  const [forceAvailable, setForceAvailable] = createSignal(false);
  let forceTimer: ReturnType<typeof setTimeout> | undefined;

  // W trybie zaznaczania (`props.selectMode`, patrz `Library.tsx`) klik na
  // kartę PRZEŁĄCZA zaznaczenie zamiast uruchamiać/instalować grę —
  // uruchomienie kilkunastu gier naraz przez przypadkowe kliknięcie w
  // trakcie zaznaczania byłoby dużo gorszym skutkiem ubocznym niż odwrotna
  // pomyłka (kliknięcie karty poza trybem zaznaczania, gdy chciało się coś
  // zaznaczyć — ale wtedy zaznaczanie jeszcze nie jest aktywne).
  const primaryAction = () => (props.selectMode ? props.onToggleSelect?.() : props.game.installed ? launch() : install());
  const focusable = useFocusable<HTMLDivElement>(() => primaryAction());

  onMount(() => {
    const progressUnlisten = api.onInstallProgress((label, line, percent) => {
      if (label.endsWith(`:${props.game.id}`)) {
        setProgressLine(line);
        if (percent !== null) setProgressPercent(percent);
      }
    });
    const finishedUnlisten = api.onInstallFinished((label, ok) => {
      if (label.endsWith(`:${props.game.id}`)) {
        setInstalling(false);
        setProgressLine(null);
        setProgressPercent(null);
        if (!ok) setErrorMsg("Instalacja nie powiodła się.");
      }
    });

    // Stan początkowy — na wypadek, gdyby gra była już uruchomiona ZANIM
    // ta karta się zamontowała (np. powrót do biblioteki po przełączeniu
    // widoku) — zdarzenia Tauri nie mają historii, sam listener niżej by
    // tego nie złapał.
    if (trackable()) {
      void api.isGameRunning(props.game.platform, props.game.id).then(setRunning);
    }

    const launchedUnlisten = api.onGameLaunched((platform, gameId) => {
      if (platform === props.game.platform && gameId === props.game.id) {
        setRunning(true);
        setErrorMsg(null);
      }
    });
    const exitedUnlisten = api.onGameExited((platform, gameId, ok, secondsRan) => {
      if (platform === props.game.platform && gameId === props.game.id) {
        setRunning(false);
        setStopping(false);
        setForceAvailable(false);
        if (forceTimer) clearTimeout(forceTimer);
        if (!ok || secondsRan < CRASH_THRESHOLD_SECONDS) {
          setErrorMsg(
            `Gra zamknęła się po ${secondsRan}s${!ok ? " z błędem" : ""} — mogła się nie uruchomić poprawnie.`,
          );
        }
      }
    });

    onCleanup(() => {
      void progressUnlisten.then((u) => u());
      void finishedUnlisten.then((u) => u());
      void launchedUnlisten.then((u) => u());
      void exitedUnlisten.then((u) => u());
      if (forceTimer) clearTimeout(forceTimer);
    });
  });

  async function launch() {
    setLaunching(true);
    setErrorMsg(null);
    try {
      await api.launchGame(props.game.platform, props.game.id);
      // Dla platform bez niezawodnego śledzenia stanu (Steam) nie ma co
      // czekać na `game-launched` (mogłoby nie nadejść znacząco, patrz
      // wyżej) — krótki spinner to jedyna informacja zwrotna, jaką da się
      // tu uczciwie pokazać.
      if (!trackable()) {
        setErrorMsg(
          `Uruchomiono przez ${PLATFORM_LABELS[props.game.platform]} — Hacker Mode nie może śledzić stanu sesji w czasie rzeczywistym.`,
        );
      }
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setLaunching(false);
    }
  }

  async function stop(force: boolean) {
    setStopping(true);
    try {
      await api.stopGame(props.game.platform, props.game.id, force);
    } catch (err) {
      setErrorMsg(String(err));
    }
    if (!force) {
      // Jeśli po 5s gra WCIĄŻ nie zniknęła z `running` (czyli nie
      // przyszedł jeszcze `game-exited`), dajemy opcję wymuszenia —
      // patrz komentarz przy `forceAvailable`.
      forceTimer = setTimeout(() => {
        if (running()) setForceAvailable(true);
      }, 5000);
    } else {
      setForceAvailable(false);
    }
  }

  async function install() {
    setInstalling(true);
    setErrorMsg(null);
    try {
      await api.installGame(props.game.platform, props.game.id);
    } catch (err) {
      setErrorMsg(String(err));
      setInstalling(false);
    }
  }

  return (
    <div class="game-card" ref={focusable.ref} tabIndex={0} data-focusable>
      <button class="cover" onClick={primaryAction} disabled={launching() || installing()}>
        {props.game.cover_path ? (
          <img
            src={resolveCoverSrc(props.game.cover_path)}
            alt={props.game.title}
            style={{ width: "100%", height: "100%", "object-fit": "cover", "border-radius": "10px" }}
          />
        ) : (
          <span>{props.game.title}</span>
        )}
        <Show when={props.selectMode}>
          <span
            style={{
              position: "absolute", top: "6px", right: "6px", width: "22px", height: "22px",
              "border-radius": "6px", display: "flex", "align-items": "center", "justify-content": "center",
              background: props.selected ? "var(--accent)" : "rgba(0,0,0,0.5)",
              border: "1px solid rgba(255,255,255,0.4)", "font-size": "13px", color: "var(--bg)",
            }}
          >
            {props.selected ? "✓" : ""}
          </span>
        </Show>
        {/* Wskaźnik "w trakcie gry" — pulsująca kropka w rogu okładki,
            widoczna dopóki nie nadejdzie `game-exited`. Tylko dla platform
            z niezawodnym śledzeniem (patrz `trackable`). */}
        <Show when={running() && !props.selectMode}>
          <span
            style={{
              position: "absolute", top: "6px", left: "6px", display: "flex", "align-items": "center", gap: "4px",
              background: "rgba(0,0,0,0.65)", "border-radius": "999px", padding: "2px 8px", "font-size": "10px", color: "#4ade80",
            }}
          >
            <span style={{ width: "6px", height: "6px", "border-radius": "50%", background: "#4ade80" }} />
            W trakcie gry
          </span>
        </Show>
      </button>
      <span class="badge">{PLATFORM_LABELS[props.game.platform]}</span>
      <div class="title">{props.game.title}</div>
      <Show when={props.game.playtime_minutes !== null && props.game.playtime_minutes! > 0}>
        <span style={{ "font-size": "11px", color: "var(--text-muted)" }}>
          {Math.round((props.game.playtime_minutes ?? 0) / 6) / 10} godz.
        </span>
      </Show>
      <button
        class="ghost-btn"
        style={{ padding: "4px 10px", "font-size": "11px" }}
        onClick={() => navigate(`/game/${props.game.platform}/${props.game.id}`)}
        disabled={props.selectMode}
      >
        ℹ Szczegóły
      </button>

      <Show when={errorMsg()}>
        <div style={{ color: "var(--danger)", "font-size": "11px" }}>{errorMsg()}</div>
      </Show>
      <Show when={progressLine()}>
        <div style={{ color: "var(--text-muted)", "font-size": "10px", "white-space": "nowrap", overflow: "hidden", "text-overflow": "ellipsis" }}>
          {progressLine()}
        </div>
        <Show when={progressPercent() !== null}>
          <div style={{ height: "4px", background: "rgba(255,255,255,0.08)", "border-radius": "999px", overflow: "hidden" }}>
            <div
              style={{
                height: "100%",
                width: `${progressPercent()}%`,
                background: "var(--accent)",
                transition: "width 0.2s",
              }}
            />
          </div>
        </Show>
      </Show>

      <Show when={!props.selectMode}>
        <Show
          when={!(running() && trackable())}
          fallback={
            <Show
              when={!forceAvailable()}
              fallback={
                <button class="ghost-btn" style={{ color: "var(--danger)" }} onClick={() => stop(true)}>
                  Wymuś zamknięcie
                </button>
              }
            >
              <button class="ghost-btn" onClick={() => stop(false)} disabled={stopping()}>
                {stopping() ? "Zatrzymywanie…" : "■ Zatrzymaj"}
              </button>
            </Show>
          }
        >
          <Show
            when={props.game.installed}
            fallback={
              <button class="ghost-btn" onClick={install} disabled={installing()}>
                {installing() ? "Instalowanie…" : <><IconDownload size={14} /> Zainstaluj</>}
              </button>
            }
          >
            <button class="primary-btn" onClick={launch} disabled={launching()}>
              {launching() ? "…" : <><IconPlay size={14} /> Uruchom</>}
            </button>
          </Show>
        </Show>
      </Show>
    </div>
  );
};

export default GameCard;
