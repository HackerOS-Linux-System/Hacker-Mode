import { Component, createSignal, onCleanup, onMount, Show, For } from "solid-js";
import { useNavigate, useParams } from "@solidjs/router";
import {
  api,
  resolveCoverSrc,
  PLATFORM_LABELS,
  PROTONDB_SUPPORTED_PLATFORMS,
  PROTONDB_TIER_INFO,
  COMPAT_TOOL_SUPPORTED_PLATFORMS,
  RELIABLE_RUNNING_STATE_PLATFORMS,
  type GameDetails,
  type Platform,
  type ProtonDbSummary,
  type Achievement,
  type CompatToolOptions,
  type BackupEntry,
  type ConnectedController,
  type SessionEntry,
} from "@/lib/tauri";
import { gamesStore } from "@/stores/gamesStore";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";
import { IconDownload, IconPlay, IconTrash, IconBack } from "@/components/Icons";

/** Platformy, dla których Hacker Mode potrafi zbudować komendę
 * odinstalowania gry — odpowiednik `Platform::slug()` zbioru obsługiwanego
 * przez `build_uninstall_command` po stronie Rust. Wszystkie 7 platform
 * jest tu dziś obsługiwane, ale w bardzo różny sposób — patrz backendowe
 * `uninstall_command` każdego providera:
 * - Steam/Epic/GOG/Amazon: prawdziwe odinstalowanie przez CLI danego
 *   klienta (`steam`/`legendary`/`gogdl`/`nile`).
 * - Lutris: oznaczenie jako nieinstalowanej w bazie Lutrisa (`pga.db`),
 *   BEZ kasowania plików — jak "Remove from library" w samym Lutrisie.
 * - EA app / Battle.net: usunięcie katalogu instalacji gry na dysku (brak
 *   prywatnego API EA/Blizzarda do "prawdziwego" odinstalowania). */
const UNINSTALL_SUPPORTED: Platform[] = ["steam", "epic", "gog", "amazon", "lutris", "ea", "battlenet"];

/** Platformy, dla których "Uruchom" otwiera sam klient (EA app / Battle.net
 * launcher), a nie bezpośrednio grę — patrz moduł-dokumentacja
 * `launchers-cli/ea-cli`/`launchers-cli/bnet`: bez znanego prywatnego API
 * logowania/uprawnień tych platform nie da się uruchomić pojedynczej gry z
 * pominięciem ich własnego klienta. */
const LAUNCHES_EXTERNAL_CLIENT: Platform[] = ["ea", "battlenet"];

const GameDetail: Component = () => {
  const params = useParams<{ platform: Platform; id: string }>();
  const navigate = useNavigate();
  const t = (key: Parameters<typeof getText>[1], vars?: Record<string, string | number>) =>
    getText(settingsStore.lang(), key, vars);
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal<string | null>(null);

  // Stan "w trakcie gry" — patrz identyczny mechanizm i wyjaśnienie
  // ograniczenia dla Steam w `GameCard.tsx`/`RELIABLE_RUNNING_STATE_PLATFORMS`.
  const trackable = () => RELIABLE_RUNNING_STATE_PLATFORMS.includes(params.platform);
  const [running, setRunning] = createSignal(false);
  const [stopping, setStopping] = createSignal(false);
  const [forceAvailable, setForceAvailable] = createSignal(false);
  let forceTimer: ReturnType<typeof setTimeout> | undefined;
  const [details, setDetails] = createSignal<GameDetails | null>(null);
  const [detailsLoading, setDetailsLoading] = createSignal(true);
  const [editingCover, setEditingCover] = createSignal(false);
  const [coverInput, setCoverInput] = createSignal("");
  // Krok pośredni przed deinstalacją — patrz `uninstallWarning` niżej.
  // `false` = przycisk „Odinstaluj” w zwykłym stanie; `true` = pokazany
  // inline dialog potwierdzenia zamiast niego. Deinstalacja Hacker Mode
  // jest NIEODWRACALNA dla większości platform (patrz treść ostrzeżenia
  // per platforma), więc jeden klik nigdy nie powinien wystarczyć.
  const [confirmingUninstall, setConfirmingUninstall] = createSignal(false);

  // ProtonDB (Steam) — patrz `PROTONDB_SUPPORTED_PLATFORMS`.
  const [protondb, setProtondb] = createSignal<ProtonDbSummary | null>(null);
  const [protondbLoading, setProtondbLoading] = createSignal(false);

  // Osiągnięcia Steam — puste, dopóki nie skonfigurowano klucza API
  // (patrz Ustawienia → Steam), wtedy sekcja po prostu się nie pokazuje.
  const [achievements, setAchievements] = createSignal<Achievement[] | null>(null);
  const [achievementsLoading, setAchievementsLoading] = createSignal(false);

  // Historia sesji — patrz `playtime::get_session_history`.
  const [sessionHistory, setSessionHistory] = createSignal<SessionEntry[]>([]);

  // Wersja Proton/Wine (Steam/Lutris) — patrz `COMPAT_TOOL_SUPPORTED_PLATFORMS`.
  const [compatOptions, setCompatOptions] = createSignal<CompatToolOptions | null>(null);
  const [compatSaving, setCompatSaving] = createSignal(false);
  const [compatMessage, setCompatMessage] = createSignal<string | null>(null);

  // Tagi użytkownika (`Settings::game_tags`) — czytane/zapisywane wprost
  // przez `settingsStore`, nie przez osobny lokalny stan listy: klucz
  // `tagKey()` zawsze wskazuje na aktualne ustawienia, więc nawigacja
  // między grami bez przeładowania komponentu (SolidJS reużywa go przy
  // zmianie parametrów trasy) nie pokazuje "przeterminowanych" tagów.
  const [tagInput, setTagInput] = createSignal("");
  const tagKey = () => `${params.platform}:${params.id}`;
  const currentTags = () => settingsStore.settings().game_tags[tagKey()] ?? [];

  // Kopie zapasowe zapisu — patrz moduł-dokumentacja `cloud_saves.rs`.
  const [savePathInput, setSavePathInput] = createSignal("");
  const [backups, setBackups] = createSignal<BackupEntry[]>([]);
  const [backupBusy, setBackupBusy] = createSignal(false);
  const [backupMessage, setBackupMessage] = createSignal<string | null>(null);
  const [confirmingRestore, setConfirmingRestore] = createSignal<string | null>(null);
  const savedSavePath = () => settingsStore.settings().game_save_paths[tagKey()] ?? null;

  // Mapowanie kontrolera — patrz moduł-dokumentacja `controllers.rs`.
  const [controllerConfigInput, setControllerConfigInput] = createSignal("");
  const [connectedControllers, setConnectedControllers] = createSignal<ConnectedController[]>([]);
  const savedControllerConfig = () => settingsStore.settings().game_controller_configs[tagKey()] ?? null;

  async function refreshBackups() {
    try {
      setBackups(await api.listGameSaveBackups(params.platform, params.id));
    } catch (err) {
      console.warn("Nie udało się pobrać listy kopii zapasowych:", err);
    }
  }

  async function saveSavePath() {
    const value = savePathInput().trim();
    try {
      await api.setGameSavePath(params.platform, params.id, value || null);
      setSavePathInput("");
      await settingsStore.reload();
      await refreshBackups();
    } catch (err) {
      setBackupMessage(String(err));
    }
  }

  async function createBackupNow() {
    setBackupBusy(true);
    setBackupMessage(null);
    try {
      await api.backupGameSave(params.platform, params.id);
      await refreshBackups();
    } catch (err) {
      setBackupMessage(String(err));
    } finally {
      setBackupBusy(false);
    }
  }

  async function restoreBackup(fileName: string) {
    setBackupBusy(true);
    setConfirmingRestore(null);
    setBackupMessage(null);
    try {
      await api.restoreGameSaveBackup(params.platform, params.id, fileName);
      setBackupMessage("Przywrócono kopię zapasową.");
      await refreshBackups();
    } catch (err) {
      setBackupMessage(String(err));
    } finally {
      setBackupBusy(false);
    }
  }

  async function saveControllerConfig() {
    const value = controllerConfigInput().trim();
    try {
      await api.setGameControllerConfig(params.platform, params.id, value || null);
      setControllerConfigInput("");
      await settingsStore.reload();
    } catch (err) {
      console.warn("Nie udało się zapisać mapowania kontrolera:", err);
    }
  }


  const game = () => gamesStore.games().find((g) => g.platform === params.platform && g.id === params.id);

  /** Treść ostrzeżenia pokazywanego przed deinstalacją — różna per
   * platforma, bo backendowe `uninstall_command` robi coś innego dla
   * każdej z nich (patrz komentarz przy `UNINSTALL_SUPPORTED` wyżej):
   * dla większości to prawdziwe odinstalowanie przez CLI klienta, ale
   * EA app/Battle.net kasują cały katalog gry z dysku (`rm -rf`), a
   * Lutris modyfikuje bezpośrednio swoją bazę `pga.db` — użytkownik
   * powinien wiedzieć DOKŁADNIE, co się stanie, zanim potwierdzi, nie
   * tylko że "coś się odinstaluje". */
  function uninstallWarning(platform: Platform): string {
    if (platform === "ea" || platform === "battlenet") {
      return t("uninstall_warning_destructive", { platform: PLATFORM_LABELS[platform] });
    }
    if (platform === "lutris") {
      return t("uninstall_warning_lutris");
    }
    return t("uninstall_warning_generic", { platform: PLATFORM_LABELS[platform] });
  }

  onMount(async () => {
    if (trackable()) {
      void api.isGameRunning(params.platform, params.id).then(setRunning);
    }
    const launchedUnlisten = api.onGameLaunched((platform, gameId) => {
      if (platform === params.platform && gameId === params.id) {
        setRunning(true);
        setMessage(null);
      }
    });
    const exitedUnlisten = api.onGameExited((platform, gameId, ok, secondsRan) => {
      if (platform === params.platform && gameId === params.id) {
        setRunning(false);
        setStopping(false);
        setForceAvailable(false);
        if (forceTimer) clearTimeout(forceTimer);
        if (!ok || secondsRan < settingsStore.settings().crash_detection_threshold_seconds) {
          setMessage(`Gra zamknęła się po ${secondsRan}s${!ok ? " z błędem" : ""} — mogła się nie uruchomić poprawnie.`);
        }
      }
    });
    onCleanup(() => {
      void launchedUnlisten.then((u) => u());
      void exitedUnlisten.then((u) => u());
      if (forceTimer) clearTimeout(forceTimer);
    });

    try {
      const result = await api.fetchGameDetails(params.platform, params.id);
      setDetails(result);
    } catch (err) {
      console.warn("Nie udało się pobrać opisu gry:", err);
    } finally {
      setDetailsLoading(false);
    }

    if (PROTONDB_SUPPORTED_PLATFORMS.includes(params.platform)) {
      setProtondbLoading(true);
      try {
        const appid = Number(params.id);
        if (!Number.isNaN(appid)) setProtondb(await api.fetchProtondbRating(appid));
      } catch (err) {
        console.warn("Nie udało się pobrać oceny ProtonDB:", err);
      } finally {
        setProtondbLoading(false);
      }

      // Osiągnięcia wymagają klucza Steam Web API + SteamID64 (Ustawienia
      // → Steam) — bez nich `fetch_steam_achievements` i tak zwróci pustą
      // listę po stronie backendu, ale sprawdzamy tu wcześniej, żeby nie
      // migać loaderem sekcji, która i tak zaraz okaże się pusta.
      const s = settingsStore.settings();
      if (s.steam_api_key && s.steam_id64) {
        setAchievementsLoading(true);
        try {
          const list = await api.fetchSteamAchievements(params.id);
          setAchievements(list.length > 0 ? list : null);
        } catch (err) {
          console.warn("Nie udało się pobrać osiągnięć Steam:", err);
        } finally {
          setAchievementsLoading(false);
        }
      }
    }

    if (COMPAT_TOOL_SUPPORTED_PLATFORMS.includes(params.platform)) {
      try {
        setCompatOptions(await api.getCompatToolOptions(params.platform, params.id));
      } catch (err) {
        console.warn("Nie udało się pobrać opcji Proton/Wine:", err);
      }
    }

    await refreshBackups();
    try {
      setConnectedControllers(await api.listConnectedControllers());
    } catch (err) {
      console.warn("Nie udało się wykryć kontrolerów:", err);
    }

    try {
      setSessionHistory(await api.getGameSessionHistory(params.platform, params.id));
    } catch (err) {
      console.warn("Nie udało się pobrać historii sesji:", err);
    }
  });

  async function saveCompatTool(value: string) {
    setCompatSaving(true);
    setCompatMessage(null);
    try {
      // `warning` niepuste = zapis się udał, ale Steam/Lutris był
      // uruchomiony w trakcie (patrz `compat_tools::set_steam_compat_tool`/
      // `set_lutris_wine_version`) — pokazujemy to jako ostrzeżenie, nie
      // błąd, bo zmiana i tak weszła w życie.
      const warning = await api.setCompatTool(params.platform, params.id, value);
      setCompatOptions((prev) => (prev ? { ...prev, current: value } : prev));
      if (warning) setCompatMessage(warning);
    } catch (err) {
      setCompatMessage(String(err));
    } finally {
      setCompatSaving(false);
    }
  }

  async function addTag() {
    const value = tagInput().trim();
    if (!value) return;
    const next = [...new Set([...currentTags(), value])];
    setTagInput("");
    try {
      await api.setGameTags(params.platform, params.id, next);
      await settingsStore.reload();
    } catch (err) {
      console.warn("Nie udało się zapisać tagu:", err);
    }
  }

  async function removeTag(tag: string) {
    const next = currentTags().filter((t) => t !== tag);
    try {
      await api.setGameTags(params.platform, params.id, next);
      await settingsStore.reload();
    } catch (err) {
      console.warn("Nie udało się usunąć tagu:", err);
    }
  }

  async function run(action: "launch" | "install" | "uninstall") {
    const g = game();
    if (!g) return;
    setBusy(true);
    setMessage(null);
    try {
      if (action === "launch") {
        await api.launchGame(g.platform, g.id);
        if (!trackable()) {
          setMessage(
            `Uruchomiono przez ${PLATFORM_LABELS[g.platform]} — Hacker Mode nie może śledzić stanu sesji w czasie rzeczywistym (ten klient przejmuje proces).`,
          );
        }
      }
      if (action === "install") await api.installGame(g.platform, g.id);
      if (action === "uninstall") {
        await api.uninstallGame(g.platform, g.id);
        await gamesStore.refresh();
      }
    } catch (err) {
      setMessage(String(err));
    } finally {
      setBusy(false);
      setConfirmingUninstall(false);
    }
  }

  async function stopGame(force: boolean) {
    const g = game();
    if (!g) return;
    setStopping(true);
    try {
      await api.stopGame(g.platform, g.id, force);
    } catch (err) {
      setMessage(String(err));
    }
    if (!force) {
      forceTimer = setTimeout(() => {
        if (running()) setForceAvailable(true);
      }, 5000);
    } else {
      setForceAvailable(false);
    }
  }

  async function saveCover() {
    const g = game();
    if (!g) return;
    const value = coverInput().trim();
    try {
      await api.setGameCover(g.platform, g.id, value || null);
    } catch (err) {
      // Backend złapał tu tylko "plik w ogóle nie istnieje" (literówka) —
      // patrz komentarz przy `commands::set_game_cover`.
      setMessage(String(err));
      return;
    }

    // BUGFIX: samo zapisanie w `Settings::cover_overrides` mogło się udać
    // (plik istnieje, URL wygląda poprawnie), a mimo to obrazek nigdy by
    // się nie wyrenderował, jeśli lokalna ścieżka leży POZA
    // `app.security.assetProtocol.scope` z `tauri.conf.json` (np. plik z
    // Pulpitu zamiast `~/Games`/cache) — Tauri po prostu odmawia
    // wczytania takiego `asset://`, bez żadnego JS-owego błędu do
    // złapania w `catch`. Wcześniej użytkownik widział tylko trwale pustą
    // okładkę, bez wyjaśnienia dlaczego. Tu faktycznie PRÓBUJEMY załadować
    // obrazek (prawdziwy `Image()`, nie zgadywanie) i jeśli się nie uda —
    // mówimy wprost, co się stało, zamiast cichej porażki.
    if (value) {
      const ok = await new Promise<boolean>((resolve) => {
        const probe = new Image();
        probe.onload = () => resolve(true);
        probe.onerror = () => resolve(false);
        probe.src = resolveCoverSrc(value);
      });
      if (!ok) {
        setMessage(
          "Zapisano, ale nie udało się wczytać tego obrazka. Jeśli to lokalny plik, musi leżeć w jednym z dozwolonych katalogów (m.in. ~/Games, ~/.local/share/lutris, ~/.local/share/Steam) — spróbuj skopiować plik tam, albo użyj bezpośredniego linku https://.",
        );
      }
    }

    setEditingCover(false);
    await gamesStore.refresh();
  }

  async function clearCover() {
    const g = game();
    if (!g) return;
    try {
      await api.setGameCover(g.platform, g.id, null);
      setEditingCover(false);
      await gamesStore.refresh();
    } catch (err) {
      setMessage(String(err));
    }
  }

  return (
    <div class="content">
      <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => navigate(-1)} style={{ "margin-bottom": "20px" }}>
        <IconBack size={14} /> Wróć
      </button>

      <Show when={game()} fallback={<div class="empty-state">Nie znaleziono gry (może biblioteka jeszcze się ładuje?).</div>}>
        {(g) => (
          <div style={{ display: "grid", "grid-template-columns": "260px 1fr", gap: "32px" }}>
            <div>
              <div class="cover" style={{ height: "360px" }}>
                <Show when={g().cover_path} fallback={<span>{g().title}</span>}>
                  <img
                    src={resolveCoverSrc(g().cover_path!)}
                    alt={g().title}
                    style={{ width: "100%", height: "100%", "object-fit": "cover", "border-radius": "10px" }}
                  />
                </Show>
              </div>

              {/* Ustawienia > obsługa zdjęć dla gier: ręczne nadpisanie
                  okładki (URL albo lokalna ścieżka pliku), patrz
                  `commands::set_game_cover`. Przydatne zwłaszcza dla
                  Lutris/EA/Battle.net, gdzie automatyczne wykrycie okładki
                  bywa niepełne (albo, dla EA/Battle.net, nie istnieje
                  wcale — patrz `Platform::Ea`/`Platform::BattleNet`). */}
              <Show
                when={editingCover()}
                fallback={
                  <button data-focusable tabIndex={0}
                    class="ghost-btn"
                    style={{ "margin-top": "10px", width: "100%", "font-size": "12px" }}
                    onClick={() => {
                      setCoverInput(g().cover_path ?? "");
                      setEditingCover(true);
                    }}
                  >
                    Zmień okładkę
                  </button>
                }
              >
                <div style={{ "margin-top": "10px", display: "flex", "flex-direction": "column", gap: "6px" }}>
                  <input data-focusable tabIndex={0}
                    type="text"
                    value={coverInput()}
                    onInput={(e) => setCoverInput(e.currentTarget.value)}
                    placeholder="Link do obrazka (https://…) lub ścieżka pliku"
                    style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)", "font-size": "12px" }}
                  />
                  <div style={{ display: "flex", gap: "6px" }}>
                    <button data-focusable tabIndex={0} class="primary-btn" style={{ flex: "1", "font-size": "12px" }} onClick={saveCover}>
                      Zapisz
                    </button>
                    <button data-focusable tabIndex={0} class="ghost-btn" style={{ "font-size": "12px" }} onClick={clearCover}>
                      Domyślna
                    </button>
                    <button data-focusable tabIndex={0} class="ghost-btn" style={{ "font-size": "12px" }} onClick={() => setEditingCover(false)}>
                      Anuluj
                    </button>
                  </div>
                </div>
              </Show>
            </div>

            <div>
              <span class="badge">{PLATFORM_LABELS[g().platform]}</span>
              <h1 style={{ margin: "10px 0" }}>{g().title}</h1>

              {/* Plakietka ProtonDB — wyłącznie Steam, patrz
                  `PROTONDB_SUPPORTED_PLATFORMS`/moduł-dokumentacja
                  `protondb.rs`. `null` może znaczyć zarówno "gra nie ma
                  jeszcze zgłoszeń w ProtonDB", jak i błąd sieciowy — w obu
                  przypadkach po prostu nie pokazujemy nic zamiast
                  zgadywać, który to przypadek. */}
              <Show when={PROTONDB_SUPPORTED_PLATFORMS.includes(g().platform) && protondb()}>
                {(rating) => {
                  const info = () => PROTONDB_TIER_INFO[rating().tier] ?? { label: rating().tier, color: "var(--text-muted)" };
                  return (
                    <a
                      href={`https://www.protondb.com/app/${g().id}`}
                      target="_blank"
                      rel="noreferrer"
                      style={{
                        display: "inline-flex",
                        "align-items": "center",
                        gap: "6px",
                        "font-size": "12px",
                        padding: "3px 10px",
                        "border-radius": "999px",
                        border: `1px solid ${info().color}`,
                        color: info().color,
                        "text-decoration": "none",
                        "margin-bottom": "8px",
                      }}
                    >
                      ProtonDB: {info().label}
                    </a>
                  );
                }}
              </Show>
              <Show when={protondbLoading()}>
                <div style={{ "font-size": "11px", color: "var(--text-muted)", "margin-bottom": "8px" }}>
                  Sprawdzam ocenę ProtonDB…
                </div>
              </Show>

              {/* Tagi użytkownika — patrz `Settings::game_tags`. Czysto
                  organizacyjne, filtrowane w `Library.tsx`; Hacker Mode
                  nigdy nie interpretuje ich treści. */}
              <div style={{ display: "flex", "flex-wrap": "wrap", gap: "6px", "align-items": "center", "margin-bottom": "12px" }}>
                <For each={currentTags()}>
                  {(tag) => (
                    <span
                      style={{
                        display: "inline-flex", "align-items": "center", gap: "4px",
                        "font-size": "11px", padding: "2px 8px", "border-radius": "999px",
                        background: "rgba(255,255,255,0.08)", color: "var(--text)",
                      }}
                    >
                      {tag}
                      <button data-focusable tabIndex={0} onClick={() => removeTag(tag)} style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", padding: "0", "font-size": "12px" }}>
                        ✕
                      </button>
                    </span>
                  )}
                </For>
                <input data-focusable tabIndex={0}
                  type="text"
                  value={tagInput()}
                  onInput={(e) => setTagInput(e.currentTarget.value)}
                  onKeyDown={(e) => e.key === "Enter" && addTag()}
                  placeholder={t("tag_input_placeholder")}
                  style={{ width: "80px", "font-size": "11px", padding: "3px 8px", "border-radius": "999px", border: "1px dashed rgba(255,255,255,0.2)", background: "transparent", color: "var(--text)" }}
                />
              </div>

              {/* Opis/zrzuty ekranu: Steam (publiczne Store API), GOG
                  (publiczne `api.gog.com`) i Epic (lokalny cache metadanych
                  `legendary`, bez zapytania sieciowego) mają realne dane —
                  dla Amazon/Lutris/EA/Battle.net `details()` pozostanie
                  `null` (brak udokumentowanego API katalogowego), patrz
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

              <Show when={LAUNCHES_EXTERNAL_CLIENT.includes(g().platform)}>
                <p style={{ color: "var(--text-muted)", "font-size": "12px", "max-width": "520px" }}>
                  ℹ Uruchomienie otworzy {PLATFORM_LABELS[g().platform]} — Hacker Mode nie potrafi
                  uruchomić tej gry z pominięciem jego własnego klienta (brak publicznie
                  udokumentowanego API logowania tej platformy). Wybierz grę już z jego poziomu.
                </p>
              </Show>

              {/* Wersja Proton (Steam) / Wine (Lutris) — patrz
                  `COMPAT_TOOL_SUPPORTED_PLATFORMS`/moduł-dokumentacja
                  `compat_tools.rs`. Dla Steam lista opcji obejmuje
                  WYŁĄCZNIE narzędzia z `compatibilitytools.d/`
                  (Proton-GE i podobne) — świadome ograniczenie, patrz ten
                  sam moduł-dokumentacja po uzasadnienie. */}
              <Show when={COMPAT_TOOL_SUPPORTED_PLATFORMS.includes(g().platform) && compatOptions()}>
                {(opts) => (
                  <div class="settings-panel" style={{ "max-width": "420px", margin: "16px 0" }}>
                    <h3 style={{ margin: "0 0 8px", "font-size": "13px" }}>
                      {g().platform === "steam" ? t("compat_tool_panel_title_steam") : t("compat_tool_panel_title_lutris")}
                    </h3>
                    <Show
                      when={opts().options.length > 0}
                      fallback={
                        <p style={{ "font-size": "11px", color: "var(--text-muted)", margin: "0" }}>
                          {g().platform === "steam"
                            ? t("compat_tool_empty_steam")
                            : t("compat_tool_empty_lutris")}
                        </p>
                      }
                    >
                      <select data-focusable tabIndex={0}
                        value={opts().current ?? ""}
                        disabled={compatSaving()}
                        onChange={(e) => saveCompatTool(e.currentTarget.value)}
                        style={{ width: "100%", padding: "8px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
                      >
                        <option value="" disabled>
                          {opts().current ?? "— wybierz —"}
                        </option>
                        <For each={opts().options}>
                          {(o) => <option value={o.value}>{o.label}</option>}
                        </For>
                      </select>
                    </Show>
                    <Show when={compatMessage()}>
                      <p style={{ color: "var(--danger)", "font-size": "11px", margin: "8px 0 0" }}>{compatMessage()}</p>
                    </Show>
                  </div>
                )}
              </Show>

              {/* Osiągnięcia Steam — patrz `PROTONDB_SUPPORTED_PLATFORMS`
                  (te same warunki dotyczą Steam Web API) i
                  `steam::fetch_achievements`. Pokazuje się tylko, gdy
                  faktycznie coś dostaliśmy — brak klucza API, prywatny
                  profil i gra bez osiągnięć wszystkie kończą się tym
                  samym: `achievements()` zostaje `null`. */}
              <Show when={achievementsLoading()}>
                <div style={{ "font-size": "11px", color: "var(--text-muted)", margin: "8px 0" }}>
                  {t("achievements_loading")}
                </div>
              </Show>
              <Show when={achievements()}>
                {(list) => {
                  const unlocked = () => list().filter((a) => a.achieved).length;
                  return (
                    <div class="settings-panel" style={{ "max-width": "600px", margin: "16px 0" }}>
                      <h3 style={{ margin: "0 0 10px", "font-size": "13px" }}>
                        {t("achievements_title")} ({unlocked()}/{list().length})
                      </h3>
                      <div style={{ display: "grid", "grid-template-columns": "repeat(auto-fill, minmax(48px, 1fr))", gap: "8px" }}>
                        <For each={list()}>
                          {(a) => (
                            <img
                              src={a.icon_url}
                              alt={a.display_name}
                              title={`${a.display_name}${a.description ? " — " + a.description : ""}`}
                              style={{ width: "48px", height: "48px", "border-radius": "6px", opacity: a.achieved ? "1" : "0.35" }}
                            />
                          )}
                        </For>
                      </div>
                    </div>
                  );
                }}
              </Show>

              {/* Historia sesji — patrz `playtime::get_session_history`.
                  Dostępna dla wszystkich platform (to lokalny licznik
                  Hacker Mode, niezależny od sklepu) — patrz
                  moduł-dokumentacja `playtime.rs` po zastrzeżenie, że
                  obejmuje TYLKO sesje uruchomione przez Hacker Mode. */}
              <Show when={sessionHistory().length > 0}>
                <div class="settings-panel" style={{ "max-width": "420px", margin: "16px 0" }}>
                  <h3 style={{ margin: "0 0 8px", "font-size": "13px" }}>Historia sesji</h3>
                  <div style={{ display: "flex", "flex-direction": "column", gap: "4px", "max-height": "140px", "overflow-y": "auto" }}>
                    <For each={sessionHistory()}>
                      {(s) => (
                        <div style={{ display: "flex", "justify-content": "space-between", "font-size": "11px", color: "var(--text-muted)" }}>
                          <span>{new Date(s.at * 1000).toLocaleString()}</span>
                          <span>{s.minutes} min</span>
                        </div>
                      )}
                    </For>
                  </div>
                </div>
              </Show>

              {/* Kopie zapasowe zapisu — patrz moduł-dokumentacja
                  `cloud_saves.rs`. Dostępne dla WSZYSTKICH platform (w
                  przeciwieństwie do ProtonDB/osiągnięć/Proton-Wine, to nie
                  jest zależne od konkretnej platformy — działa tak samo
                  dla Steam, Epic, Lutrisa itd., bo to WYŁĄCZNIE mechanizm
                  Hacker Mode, nieoparty na żadnym API sklepu). */}
              <div class="settings-panel" style={{ "max-width": "520px", margin: "16px 0" }}>
                <h3 style={{ margin: "0 0 8px", "font-size": "13px" }}>Kopie zapasowe zapisu</h3>
                <Show
                  when={settingsStore.settings().cloud_saves_backup_dir}
                  fallback={
                    <p style={{ "font-size": "11px", color: "var(--text-muted)", margin: "0" }}>
                      Ustaw najpierw katalog na kopie zapasowe w Ustawieniach → Kopie zapasowe.
                    </p>
                  }
                >
                  <Show
                    when={savedSavePath()}
                    fallback={
                      <>
                        <p style={{ "font-size": "11px", color: "var(--text-muted)", margin: "0 0 8px" }}>
                          Wskaż katalog, w którym ta gra trzyma zapisy — Hacker Mode nie zgaduje go
                          automatycznie (patrz moduł-dokumentacja `cloud_saves.rs`).
                        </p>
                        <div style={{ display: "flex", gap: "8px" }}>
                          <input data-focusable tabIndex={0}
                            type="text"
                            value={savePathInput()}
                            onInput={(e) => setSavePathInput(e.currentTarget.value)}
                            placeholder="/home/deck/.local/share/mojagra/saves"
                            style={{ flex: "1", padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
                          />
                          <button data-focusable tabIndex={0} class="ghost-btn" onClick={saveSavePath}>
                            Zapisz
                          </button>
                        </div>
                      </>
                    }
                  >
                    <p style={{ "font-size": "11px", color: "var(--text-muted)", margin: "0 0 8px", "word-break": "break-all" }}>
                      Katalog zapisu: {savedSavePath()}{" "}
                      <button
                        data-focusable tabIndex={0}
                        onClick={() => api.setGameSavePath(params.platform, params.id, null).then(() => settingsStore.reload())}
                        style={{ background: "none", border: "none", color: "var(--accent)", cursor: "pointer", padding: "0", "font-size": "11px" }}
                      >
                        (zmień)
                      </button>
                    </p>
                    <div style={{ display: "flex", gap: "8px", "margin-bottom": "10px" }}>
                      <button data-focusable tabIndex={0} class="ghost-btn" onClick={createBackupNow} disabled={backupBusy()}>
                        {backupBusy() ? "…" : "Utwórz kopię teraz"}
                      </button>
                    </div>
                    <Show when={backupMessage()}>
                      <p style={{ "font-size": "11px", color: "var(--danger)", margin: "0 0 8px" }}>{backupMessage()}</p>
                    </Show>
                    <Show when={backups().length > 0} fallback={<p style={{ "font-size": "11px", color: "var(--text-muted)" }}>Brak kopii zapasowych.</p>}>
                      <div style={{ display: "flex", "flex-direction": "column", gap: "6px", "max-height": "160px", "overflow-y": "auto" }}>
                        <For each={backups()}>
                          {(b) => (
                            <div style={{ display: "flex", "align-items": "center", "justify-content": "space-between", "font-size": "11px" }}>
                              <span>
                                {new Date(b.created_at * 1000).toLocaleString()} ({Math.round(b.size_bytes / 1024)} KB)
                              </span>
                              <Show
                                when={confirmingRestore() === b.file_name}
                                fallback={
                                  <button data-focusable tabIndex={0} class="ghost-btn" style={{ padding: "2px 8px" }} onClick={() => setConfirmingRestore(b.file_name)} disabled={backupBusy()}>
                                    Przywróć
                                  </button>
                                }
                              >
                                <span style={{ display: "flex", gap: "4px" }}>
                                  <button data-focusable tabIndex={0} class="ghost-btn" style={{ padding: "2px 8px", color: "var(--danger)" }} onClick={() => restoreBackup(b.file_name)}>
                                    Tak, nadpisz
                                  </button>
                                  <button data-focusable tabIndex={0} class="ghost-btn" style={{ padding: "2px 8px" }} onClick={() => setConfirmingRestore(null)}>
                                    Anuluj
                                  </button>
                                </span>
                              </Show>
                            </div>
                          )}
                        </For>
                      </div>
                    </Show>
                  </Show>
                </Show>
              </div>

              {/* Mapowanie kontrolera — patrz moduł-dokumentacja
                  `controllers.rs`. Też dostępne dla wszystkich platform
                  (SDL_GAMECONTROLLERCONFIG to mechanizm SDL, nie sklepu). */}
              <div class="settings-panel" style={{ "max-width": "520px", margin: "16px 0" }}>
                <h3 style={{ margin: "0 0 8px", "font-size": "13px" }}>Mapowanie kontrolera</h3>
                <Show when={connectedControllers().length > 0}>
                  <p style={{ "font-size": "11px", color: "var(--text-muted)", margin: "0 0 8px" }}>
                    Wykryto: {connectedControllers().map((c) => c.name).join(", ")}
                  </p>
                </Show>
                <Show
                  when={savedControllerConfig()}
                  fallback={
                    <>
                      <p style={{ "font-size": "11px", color: "var(--text-muted)", margin: "0 0 8px" }}>
                        Wklej string SDL_GAMECONTROLLERCONFIG wygenerowany np. narzędziem „SDL2 Gamepad
                        Tool” — zostanie ustawiony jako zmienna środowiskowa TYLKO dla tej gry.
                      </p>
                      <div style={{ display: "flex", gap: "8px" }}>
                        <input data-focusable tabIndex={0}
                          type="text"
                          value={controllerConfigInput()}
                          onInput={(e) => setControllerConfigInput(e.currentTarget.value)}
                          placeholder="030000005e0400008e02000010010000,Xbox 360 Controller,..."
                          style={{ flex: "1", padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
                        />
                        <button data-focusable tabIndex={0} class="ghost-btn" onClick={saveControllerConfig}>
                          Zapisz
                        </button>
                      </div>
                    </>
                  }
                >
                  <p style={{ "font-size": "11px", color: "var(--text-muted)", margin: "0", "word-break": "break-all" }}>
                    Ustawiono własne mapowanie.{" "}
                    <button
                      data-focusable tabIndex={0}
                      onClick={() => api.setGameControllerConfig(params.platform, params.id, null).then(() => settingsStore.reload())}
                      style={{ background: "none", border: "none", color: "var(--accent)", cursor: "pointer", padding: "0", "font-size": "11px" }}
                    >
                      (usuń)
                    </button>
                  </p>
                </Show>
              </div>

              <div style={{ display: "flex", gap: "10px", "margin-top": "20px" }}>
                <Show
                  when={g().installed}
                  fallback={
                    <button data-focusable tabIndex={0} class="primary-btn" disabled={busy()} onClick={() => run("install")}>
                      <IconDownload size={14} /> Zainstaluj
                    </button>
                  }
                >
                  <Show
                    when={!(running() && trackable())}
                    fallback={
                      <div style={{ display: "flex", "align-items": "center", gap: "10px" }}>
                        <span
                          style={{
                            display: "flex", "align-items": "center", gap: "6px",
                            "font-size": "13px", color: "#4ade80",
                          }}
                        >
                          <span style={{ width: "8px", height: "8px", "border-radius": "50%", background: "#4ade80" }} />
                          W trakcie gry
                        </span>
                        <Show
                          when={!forceAvailable()}
                          fallback={
                            <button data-focusable tabIndex={0} class="ghost-btn" style={{ color: "var(--danger)" }} onClick={() => stopGame(true)}>
                              Wymuś zamknięcie
                            </button>
                          }
                        >
                          <button data-focusable tabIndex={0} class="ghost-btn" disabled={stopping()} onClick={() => stopGame(false)}>
                            {stopping() ? "Zatrzymywanie…" : "■ Zatrzymaj"}
                          </button>
                        </Show>
                      </div>
                    }
                  >
                    <button data-focusable tabIndex={0} class="primary-btn" disabled={busy()} onClick={() => run("launch")}>
                      <IconPlay size={14} /> Uruchom
                    </button>
                  </Show>
                  <Show
                    when={UNINSTALL_SUPPORTED.includes(g().platform)}
                    fallback={
                      <span style={{ color: "var(--text-muted)", "font-size": "13px", "align-self": "center" }}>
                        Odinstaluj z poziomu {PLATFORM_LABELS[g().platform]} (Hacker Mode tego nie obsługuje).
                      </span>
                    }
                  >
                    <button data-focusable tabIndex={0} class="ghost-btn" disabled={busy()} onClick={() => setConfirmingUninstall(true)} style={{ color: "var(--danger)" }}>
                      <IconTrash size={14} /> Odinstaluj
                    </button>
                  </Show>
                </Show>
              </div>

              <Show when={confirmingUninstall() && game()}>
                {(g) => (
                  <div
                    style={{
                      "margin-top": "14px",
                      padding: "12px",
                      "border-radius": "8px",
                      border: "1px solid var(--danger)",
                      background: "rgba(255,80,80,0.08)",
                      "max-width": "480px",
                    }}
                  >
                    <p style={{ margin: "0 0 10px", "font-size": "13px" }}>{uninstallWarning(g().platform)}</p>
                    <div style={{ display: "flex", gap: "8px" }}>
                      <button
                        data-focusable tabIndex={0}
                        class="primary-btn"
                        style={{ background: "var(--danger)" }}
                        disabled={busy()}
                        onClick={() => run("uninstall")}
                      >
                        {busy() ? "Odinstalowuję…" : "Tak, odinstaluj"}
                      </button>
                      <button data-focusable tabIndex={0} class="ghost-btn" disabled={busy()} onClick={() => setConfirmingUninstall(false)}>
                        Anuluj
                      </button>
                    </div>
                  </div>
                )}
              </Show>

              {message() && <div style={{ color: "var(--danger)", "margin-top": "14px" }}>{message()}</div>}
            </div>
          </div>
        )}
      </Show>
    </div>
  );
};

export default GameDetail;
