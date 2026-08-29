import { Component, createMemo, createSignal, For, onMount, Show } from "solid-js";
import { gamesStore } from "@/stores/gamesStore";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";
import GameCard from "@/components/GameCard";
import { api, PLATFORM_LABELS, type Platform } from "@/lib/tauri";

const ALL_PLATFORMS: Platform[] = ["steam", "epic", "gog", "amazon", "lutris", "ea", "battlenet"];

type SortMode = "name" | "playtime" | "installed";

const Library: Component = () => {
  const t = (key: Parameters<typeof getText>[1], vars?: Record<string, string | number>) =>
    getText(settingsStore.lang(), key, vars);
  const [tab, setTab] = createSignal<Platform | "all">("all");
  const [search, setSearch] = createSignal("");
  const [sort, setSort] = createSignal<SortMode>("name");
  const [activeTag, setActiveTag] = createSignal<string | null>(null);

  // Tryb zaznaczania / operacje zbiorcze — patrz `bulkAction` niżej.
  // `selectMode` przełącza `GameCard` w tryb checkboxów (klik na kartę
  // zaznacza zamiast uruchamiać/instalować grę, patrz props przekazywane
  // do `GameCard` w `<For>` poniżej).
  const [selectMode, setSelectMode] = createSignal(false);
  const [selected, setSelected] = createSignal<Set<string>>(new Set());
  const [bulkRunning, setBulkRunning] = createSignal(false);
  const [bulkProgress, setBulkProgress] = createSignal<{ done: number; total: number } | null>(null);
  const [confirmingBulk, setConfirmingBulk] = createSignal<"install" | "uninstall" | null>(null);
  // Wyniki OSTATNIEJ operacji zbiorczej — patrz `bulkAction`. Prawdziwe
  // "cofnięcie" częściowo wykonanej operacji zbiorczej nie jest tu
  // możliwe w ogólnym przypadku (odinstalowanie z powrotem
  // zainstalowanej gry nie przywraca np. zapisów stanu gry usuniętych
  // przy deinstalacji) — zamiast fałszywie obiecywać "cofnij", Hacker
  // Mode jest tu w pełni przejrzysty co do tego, co się faktycznie
  // udało/nie udało, i pozwala ponowić TYLKO nieudane pozycje.
  const [bulkResults, setBulkResults] = createSignal<{ key: string; ok: boolean }[] | null>(null);
  const [lastBulkAction, setLastBulkAction] = createSignal<"install" | "uninstall" | null>(null);

  onMount(() => {
    void gamesStore.refresh();
  });

  function selectionKey(platform: Platform, id: string) {
    return `${platform}:${id}`;
  }

  function toggleSelected(platform: Platform, id: string) {
    const key = selectionKey(platform, id);
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  function exitSelectMode() {
    setSelectMode(false);
    setSelected(new Set<string>());
    setConfirmingBulk(null);
    setBulkResults(null);
  }

  /** Wykonuje instalację/deinstalację dla wszystkich zaznaczonych gier —
   * SEKWENCYJNIE (jedna po drugiej), nie równolegle: te same komendy
   * backendowe co pojedyncze przyciski w `GameCard`/`GameDetail`, które
   * uruchamiają zewnętrzne procesy (`steam`/`legendary`/`gogdl`/...) —
   * odpalenie kilkunastu naraz obciążyłoby dysk/sieć bez żadnej korzyści
   * (i tak trzeba czekać na najwolniejszą), a sekwencyjnie łatwo pokazać
   * postęp (`bulkProgress`, "3/8 ukończone"). Błąd pojedynczej gry NIE
   * przerywa reszty — zbierany jest do `bulkResults` (pokazywanego po
   * zakończeniu jako lista sukces/porażka, patrz JSX niżej), żeby jedna
   * zepsuta gra nie zablokowała operacji na reszcie zaznaczenia I żeby
   * użytkownik dokładnie wiedział, co się udało, a co nie — zamiast
   * cichego "gotowe" mimo częściowych błędów.
   */
  async function bulkAction(action: "install" | "uninstall", keysOverride?: string[]) {
    const keys = keysOverride ?? Array.from(selected());
    setBulkRunning(true);
    setConfirmingBulk(null);
    setBulkResults(null);
    setLastBulkAction(action);
    setBulkProgress({ done: 0, total: keys.length });
    const results: { key: string; ok: boolean }[] = [];
    for (let i = 0; i < keys.length; i++) {
      const [platform, ...rest] = keys[i].split(":");
      const id = rest.join(":");
      try {
        if (action === "install") await api.installGame(platform as Platform, id);
        else await api.uninstallGame(platform as Platform, id);
        results.push({ key: keys[i], ok: true });
      } catch (err) {
        console.warn(`Operacja zbiorcza (${action}) nie powiodła się dla ${keys[i]}:`, err);
        results.push({ key: keys[i], ok: false });
      }
      setBulkProgress({ done: i + 1, total: keys.length });
    }
    setBulkRunning(false);
    setBulkProgress(null);
    setBulkResults(results);
    setSelected(new Set<string>());
    await gamesStore.refresh();
  }

  function titleForKey(key: string): string {
    const [platform, ...rest] = key.split(":");
    const id = rest.join(":");
    const game = gamesStore.games().find((g) => g.platform === platform && g.id === id);
    return game?.title ?? key;
  }

  async function retryFailed() {
    const action = lastBulkAction();
    const failedKeys = bulkResults()?.filter((r) => !r.ok).map((r) => r.key) ?? [];
    if (!action || failedKeys.length === 0) return;
    await bulkAction(action, failedKeys);
  }

  // BUGFIX: zakładki dawniej były zahardkodowane niezależnie od
  // `Settings::enabled_platforms` — po stronie backendu biblioteka jest
  // już przefiltrowana wg tego ustawienia (patrz `commands::list_games`),
  // więc zakładka wyłączonej platformy zawsze pokazywała się jako pusta,
  // wyglądając jak błąd zamiast efektu świadomego ustawienia. Teraz
  // pokazujemy tylko zakładki platform faktycznie włączonych w
  // ustawieniach (pusta lista ustawień = pokaż wszystkie, tak samo jak
  // interpretuje to backend).
  const visiblePlatforms = () => {
    const enabled = settingsStore.settings().enabled_platforms;
    if (enabled.length === 0) return ALL_PLATFORMS;
    return ALL_PLATFORMS.filter((p) => enabled.includes(p));
  };

  /** Wszystkie tagi użyte w bibliotece, posortowane alfabetycznie — do
   * chipów filtra niżej. Liczone z `Settings::game_tags` wprost (nie z
   * `gamesStore.games()`, bo tagi NIE są mergowane do struktury `Game`,
   * patrz komentarz przy `commands::set_game_tags`), niezależnie od
   * aktualnego `tab()`/`search()` — filtr tagów powinien zawsze pokazywać
   * WSZYSTKIE dostępne tagi, nie tylko te pasujące do reszty filtrów,
   * inaczej użytkownik nie mógłby przez niego "wyjść" z pustego wyniku. */
  const allTags = createMemo(() => {
    const tags = new Set<string>();
    for (const list of Object.values(settingsStore.settings().game_tags)) {
      for (const tag of list) tags.add(tag);
    }
    return Array.from(tags).sort((a, b) => a.localeCompare(b));
  });

  const filtered = createMemo(() => {
    let games = tab() === "all" ? gamesStore.games() : gamesStore.gamesByPlatform(tab() as Platform);

    const query = search().trim().toLowerCase();
    if (query) {
      games = games.filter((g) => g.title.toLowerCase().includes(query));
    }

    const tag = activeTag();
    if (tag) {
      const tagsByKey = settingsStore.settings().game_tags;
      games = games.filter((g) => (tagsByKey[selectionKey(g.platform, g.id)] ?? []).includes(tag));
    }

    const sorted = [...games];
    switch (sort()) {
      case "playtime":
        sorted.sort((a, b) => (b.playtime_minutes ?? 0) - (a.playtime_minutes ?? 0));
        break;
      case "installed":
        sorted.sort((a, b) => Number(b.installed) - Number(a.installed));
        break;
      default:
        sorted.sort((a, b) => a.title.localeCompare(b.title));
    }
    return sorted;
  });

  return (
    <div class="content">
      <div class="page-title">{t("library")}</div>

      <div class="tabs">
        <button data-focusable tabIndex={0} class={`tab ${tab() === "all" ? "active" : ""}`} onClick={() => setTab("all")}>
          Wszystko
        </button>
        <For each={visiblePlatforms()}>
          {(p) => (
            <button data-focusable tabIndex={0}
              class={`tab ${tab() === p ? "active" : ""}`}
              onClick={() => setTab(p)}
            >
              {PLATFORM_LABELS[p]}
            </button>
          )}
        </For>
        <button
          data-focusable tabIndex={0}
          class={`tab ${selectMode() ? "active" : ""}`}
          onClick={() => (selectMode() ? exitSelectMode() : setSelectMode(true))}
          style={{ "margin-left": "auto" }}
        >
          {selectMode() ? t("bulk_cancel_select_mode") : t("bulk_select_mode")}
        </button>
        <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => gamesStore.refresh()}>
          ⟳
        </button>
      </div>

      <div style={{ display: "flex", gap: "10px", "margin-bottom": "12px" }}>
        <input data-focusable tabIndex={0}
          type="text"
          value={search()}
          onInput={(e) => setSearch(e.currentTarget.value)}
          placeholder={t("bulk_search_placeholder")}
          style={{ flex: "1", padding: "10px", "border-radius": "8px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg-card)", color: "var(--text)" }}
        />
        <select data-focusable tabIndex={0}
          value={sort()}
          onChange={(e) => setSort(e.currentTarget.value as SortMode)}
          style={{ padding: "10px", "border-radius": "8px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg-card)", color: "var(--text)" }}
        >
          <option value="name">{t("bulk_sort_name")}</option>
          <option value="playtime">{t("bulk_sort_playtime")}</option>
          <option value="installed">{t("bulk_sort_installed")}</option>
        </select>
      </div>

      <Show when={allTags().length > 0}>
        <div style={{ display: "flex", "flex-wrap": "wrap", gap: "6px", "margin-bottom": "20px" }}>
          <For each={allTags()}>
            {(tag) => (
              <button
                data-focusable tabIndex={0}
                onClick={() => setActiveTag(activeTag() === tag ? null : tag)}
                style={{
                  "font-size": "11px", padding: "3px 10px", "border-radius": "999px",
                  border: activeTag() === tag ? "1px solid var(--accent)" : "1px solid rgba(255,255,255,0.15)",
                  background: activeTag() === tag ? "var(--accent)" : "transparent",
                  color: activeTag() === tag ? "var(--bg)" : "var(--text-muted)",
                  cursor: "pointer",
                }}
              >
                {tag}
              </button>
            )}
          </For>
        </div>
      </Show>

      <Show when={gamesStore.error()}>
        <div class="empty-state">{gamesStore.error()}</div>
      </Show>

      {/* Ostrzeżenia per-platforma (np. zepsuta sesja `legendary`, brak
          `sqlite3` dla Lutrisa) — patrz `gamesStore.warnings` i komentarz
          przy `LibraryLoadResult` w `commands/stores/mod.rs`. Nie blokują
          widoku reszty biblioteki. */}
      <Show when={gamesStore.warnings().length > 0}>
        <div class="settings-panel" style={{ "margin-bottom": "16px" }}>
          <For each={gamesStore.warnings()}>
            {(w) => (
              <div style={{ color: "var(--danger)", "font-size": "12px", padding: "4px 0" }}>
                {PLATFORM_LABELS[w.platform] ?? w.platform}: {w.message}
              </div>
            )}
          </For>
        </div>
      </Show>

      <Show
        when={!gamesStore.isLoading()}
        fallback={<div class="empty-state">…</div>}
      >
        <Show when={filtered().length > 0} fallback={<div class="empty-state">{t("no_games_found")}</div>}>
          <div class="grid">
            <For each={filtered()}>
              {(game) => (
                <GameCard
                  game={game}
                  selectMode={selectMode()}
                  selected={selected().has(selectionKey(game.platform, game.id))}
                  onToggleSelect={() => toggleSelected(game.platform, game.id)}
                />
              )}
            </For>
          </div>
        </Show>
      </Show>

      {/* Pasek operacji zbiorczych — pojawia się tylko, gdy coś jest
          zaznaczone LUB gdy mamy wyniki ostatniej operacji do pokazania.
          Dwustopniowe potwierdzenie dla deinstalacji, tak samo jak
          pojedyncza deinstalacja w `GameDetail.tsx` — zbiorcza wersja
          jest RÓWNIE nieodwracalna, tylko dla większej liczby gier naraz,
          więc zasługuje na to samo zabezpieczenie, jeśli nie większe. */}
      <Show when={(selectMode() && selected().size > 0) || bulkResults()}>
        <div
          style={{
            position: "fixed", left: "50%", transform: "translateX(-50%)", bottom: "24px",
            background: "var(--bg-card)", border: "1px solid rgba(255,255,255,0.15)",
            "border-radius": "12px", padding: "14px 18px", display: "flex",
            "flex-direction": "column", gap: "10px", "max-width": "480px",
            "box-shadow": "0 8px 24px rgba(0,0,0,0.4)", "z-index": "50",
          }}
        >
          {/* Panel wyników — pokazuje się PO zakończeniu operacji, zamiast
              cichego powrotu do zwykłego widoku, właśnie po to, żeby
              częściowa porażka nie przeszła niezauważona (patrz komentarz
              przy `bulkResults` wyżej: to jest odpowiedź na brak
              prawdziwego "cofnij" — pełna przejrzystość + możliwość
              ponowienia TYLKO nieudanych pozycji). */}
          <Show when={bulkResults()}>
            {(results) => {
              const failed = () => results().filter((r) => !r.ok);
              const succeeded = () => results().filter((r) => r.ok);
              return (
                <>
                  <span style={{ "font-size": "13px" }}>
                    {t("bulk_completed")}: {succeeded().length}/{results().length}
                    {failed().length > 0 && ` (${failed().length})`}
                  </span>
                  <Show when={failed().length > 0}>
                    <div style={{ "max-height": "120px", "overflow-y": "auto", "font-size": "11px", color: "var(--danger)" }}>
                      <For each={failed()}>{(r) => <div>✕ {titleForKey(r.key)}</div>}</For>
                    </div>
                  </Show>
                  <div style={{ display: "flex", gap: "8px" }}>
                    <Show when={failed().length > 0}>
                      <button data-focusable tabIndex={0} class="ghost-btn" onClick={retryFailed} disabled={bulkRunning()}>
                        {t("bulk_retry_failed")} ({failed().length})
                      </button>
                    </Show>
                    <button data-focusable tabIndex={0} class="ghost-btn" onClick={exitSelectMode}>
                      {t("bulk_close")}
                    </button>
                  </div>
                </>
              );
            }}
          </Show>

          <Show when={!bulkResults() && !confirmingBulk()}>
            <Show
              when={!bulkRunning()}
              fallback={
                <span style={{ "font-size": "13px" }}>
                  {t("bulk_processing")} {bulkProgress()?.done ?? 0}/{bulkProgress()?.total ?? 0}
                </span>
              }
            >
              <div style={{ display: "flex", "align-items": "center", gap: "12px" }}>
                <span style={{ "font-size": "13px" }}>{t("bulk_selected_count")}: {selected().size}</span>
                <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => setConfirmingBulk("install")}>
                  {t("bulk_install_selected")}
                </button>
                <button data-focusable tabIndex={0} class="ghost-btn" style={{ color: "var(--danger)" }} onClick={() => setConfirmingBulk("uninstall")}>
                  {t("bulk_uninstall_selected")}
                </button>
                <button data-focusable tabIndex={0} class="ghost-btn" onClick={exitSelectMode}>
                  {t("bulk_cancel")}
                </button>
              </div>
            </Show>
          </Show>

          <Show when={!bulkResults() && confirmingBulk()}>
            <div style={{ display: "flex", "align-items": "center", gap: "12px" }}>
              <span style={{ "font-size": "13px", color: "var(--danger)" }}>
                {confirmingBulk() === "uninstall"
                  ? t("bulk_confirm_uninstall", { count: selected().size })
                  : t("bulk_confirm_install", { count: selected().size })}
              </span>
              <button data-focusable tabIndex={0} class="primary-btn" onClick={() => bulkAction(confirmingBulk()!)}>
                {t("bulk_confirm_yes")}
              </button>
              <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => setConfirmingBulk(null)}>
                {t("bulk_cancel")}
              </button>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
};

export default Library;
