import { Component, createSignal, onCleanup, onMount, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { api, type Game } from "@/lib/tauri";
import { useFocusable } from "@/lib/focusNav";
import { IconDownload, IconPlay } from "@/components/Icons";

const GameCard: Component<{ game: Game }> = (props) => {
  const navigate = useNavigate();
  const [launching, setLaunching] = createSignal(false);
  const [installing, setInstalling] = createSignal(false);
  const [progressLine, setProgressLine] = createSignal<string | null>(null);
  const [progressPercent, setProgressPercent] = createSignal<number | null>(null);
  const [errorMsg, setErrorMsg] = createSignal<string | null>(null);
  const focusable = useFocusable<HTMLDivElement>(() => (props.game.installed ? launch() : install()));

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
    onCleanup(() => {
      void progressUnlisten.then((u) => u());
      void finishedUnlisten.then((u) => u());
    });
  });

  async function launch() {
    setLaunching(true);
    setErrorMsg(null);
    try {
      await api.launchGame(props.game.platform, props.game.id);
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setLaunching(false);
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
      <button class="cover" onClick={() => (props.game.installed ? launch() : install())} disabled={launching() || installing()}>
        {props.game.cover_path ? (
          <img
            src={props.game.cover_path}
            alt={props.game.title}
            style={{ width: "100%", height: "100%", "object-fit": "cover", "border-radius": "10px" }}
          />
        ) : (
          <span>{props.game.title}</span>
        )}
      </button>
      <span class="badge">{props.game.platform}</span>
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
    </div>
  );
};

export default GameCard;
