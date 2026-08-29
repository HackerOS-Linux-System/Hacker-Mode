import { Component, createSignal, For } from "solid-js";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";
import StoreLogin from "@/components/StoreLogin";
import { api, type SteamSearchResult, type GogSearchResult } from "@/lib/tauri";

interface StoreFront {
  id: string;
  label: string;
  url: string;
  hint: string;
}

// Otwieranie tych stron dzieje się we własnym oknie webview należącym do
// TEJ SAMEJ sesji/procesu Hacker Mode (a nie w zewnętrznej przeglądarce) —
// dokładnie tak jak przeglądanie sklepu w Heroic Games Launcher.
const STOREFRONTS: StoreFront[] = [
  {
    id: "steam",
    label: "Steam Store",
    url: "https://store.steampowered.com",
    hint: "Po zainstalowaniu gry przez Steam pojawi się automatycznie w Bibliotece.",
  },
  {
    id: "epic",
    label: "Epic Games Store",
    url: "https://store.epicgames.com",
    hint: "Instalacja/logowanie odbywa się przez `legendary` — patrz Ustawienia → Sklepy.",
  },
  {
    id: "gog",
    label: "GOG.com",
    url: "https://www.gog.com/games",
    hint: "Instalacja/logowanie odbywa się przez `gogdl` — patrz Ustawienia → Sklepy.",
  },
  {
    id: "amazon",
    label: "Amazon Games",
    url: "https://gaming.amazon.com",
    hint: "Instalacja/logowanie odbywa się przez `nile` — patrz Ustawienia → Sklepy.",
  },
  {
    id: "ea",
    label: "EA app",
    url: "https://www.ea.com/games/library",
    hint: "Hacker Mode uruchamia EA app pod Wine (Ustawienia → Sklepy) — kup/zainstaluj z jego poziomu.",
  },
  {
    id: "battlenet",
    label: "Battle.net",
    url: "https://shop.battle.net",
    hint: "Hacker Mode uruchamia Battle.net pod Wine (Ustawienia → Sklepy) — kup/zainstaluj z jego poziomu.",
  },
];

function openStorefront(front: StoreFront) {
  // Nowe okno webview, ale wciąż część tego samego procesu/sesji Hacker
  // Mode — zamykane automatycznie, gdy sesja się kończy.
  const webview = new WebviewWindow(`storefront-${front.id}`, {
    url: front.url,
    title: front.label,
    width: 1200,
    height: 800,
    decorations: true,
  });
  webview.once("tauri://error", (e) => {
    console.error("Nie udało się otworzyć sklepu:", e);
  });
}

const Store: Component = () => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);
  const [query, setQuery] = createSignal("");
  const [results, setResults] = createSignal<SteamSearchResult[]>([]);
  const [searching, setSearching] = createSignal(false);

  const [gogQuery, setGogQuery] = createSignal("");
  const [gogResults, setGogResults] = createSignal<GogSearchResult[]>([]);
  const [gogSearching, setGogSearching] = createSignal(false);

  async function search() {
    if (!query().trim()) {
      setResults([]);
      return;
    }
    setSearching(true);
    try {
      setResults(await api.searchSteamStore(query().trim()));
    } catch (err) {
      console.warn("Wyszukiwanie w katalogu Steam nie powiodło się:", err);
      setResults([]);
    } finally {
      setSearching(false);
    }
  }

  // Odpowiednik `search()` wyżej, ale dla katalogu GOG
  // (`catalog.gog.com`, patrz `stores::gog::search_store`) — GOG, tak jak
  // Steam, ma publiczne API katalogu, więc nie musi być przeglądany jako
  // zwykła strona WWW (patrz `STOREFRONTS` niżej, gdzie zostaje jako
  // zapasowa opcja).
  async function searchGog() {
    if (!gogQuery().trim()) {
      setGogResults([]);
      return;
    }
    setGogSearching(true);
    try {
      setGogResults(await api.searchGogStore(gogQuery().trim()));
    } catch (err) {
      console.warn("Wyszukiwanie w katalogu GOG nie powiodło się:", err);
      setGogResults([]);
    } finally {
      setGogSearching(false);
    }
  }

  return (
    <div class="content">
      <div class="page-title">{t("store")}</div>

      <StoreLogin />

      <div class="settings-panel">
        <h3>Steam — wyszukaj w katalogu</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 12px" }}>
          To realne przeglądanie katalogu Steam przez dane (publiczne API
          wyszukiwania), a nie strona WWW w oknie — wyniki prowadzą wprost do
          strony produktu w kliencie Steam. Dla Epic/Amazon nie ma
          publicznego, nieuwierzytelnionego API katalogu (GOG ma swoje —
          patrz sekcja niżej), więc te dwa sklepy są nadal przeglądane jako
          strony WWW dalej na tej stronie.
        </p>
        <div style={{ display: "flex", gap: "8px", "margin-bottom": "16px" }}>
          <input data-focusable tabIndex={0}
            type="text"
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && search()}
            placeholder="Szukaj gry na Steam…"
            style={{ flex: "1", padding: "10px", "border-radius": "8px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
          />
          <button data-focusable tabIndex={0} class="primary-btn" onClick={search} disabled={searching()}>
            {searching() ? "…" : "Szukaj"}
          </button>
        </div>

        <div class="grid" style={{ "grid-template-columns": "repeat(auto-fill, minmax(220px, 1fr))" }}>
          <For each={results()}>
            {(item) => (
              <div class="game-card" style={{ "flex-direction": "row", "align-items": "center", padding: "10px" }}>
                {item.cover_url && (
                  <img src={item.cover_url} alt="" style={{ width: "80px", "border-radius": "6px" }} />
                )}
                <div style={{ flex: "1" }}>
                  <div class="title">{item.name}</div>
                  {item.price && (
                    <div style={{ "font-size": "12px", color: "var(--text-muted)" }}>{item.price}</div>
                  )}
                  <button data-focusable tabIndex={0} class="ghost-btn" style={{ "margin-top": "6px" }} onClick={() => api.openSteamStorePage(item.appid)}>
                    Otwórz w Steam
                  </button>
                </div>
              </div>
            )}
          </For>
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("gog_search_title")}</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 12px" }}>
          {t("gog_search_body")}
        </p>
        <div style={{ display: "flex", gap: "8px", "margin-bottom": "16px" }}>
          <input data-focusable tabIndex={0}
            type="text"
            value={gogQuery()}
            onInput={(e) => setGogQuery(e.currentTarget.value)}
            onKeyDown={(e) => e.key === "Enter" && searchGog()}
            placeholder={t("gog_search_placeholder")}
            style={{ flex: "1", padding: "10px", "border-radius": "8px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
          />
          <button data-focusable tabIndex={0} class="primary-btn" onClick={searchGog} disabled={gogSearching()}>
            {gogSearching() ? t("gog_search_button_busy") : t("gog_search_button")}
          </button>
        </div>

        <div class="grid" style={{ "grid-template-columns": "repeat(auto-fill, minmax(220px, 1fr))" }}>
          <For each={gogResults()}>
            {(item) => (
              <div class="game-card" style={{ "flex-direction": "row", "align-items": "center", padding: "10px" }}>
                {item.cover_url && (
                  <img src={item.cover_url} alt="" style={{ width: "80px", "border-radius": "6px" }} />
                )}
                <div style={{ flex: "1" }}>
                  <div class="title">{item.title}</div>
                  <button
                    data-focusable tabIndex={0}
                    class="ghost-btn"
                    style={{ "margin-top": "6px" }}
                    disabled={!item.slug}
                    onClick={() => item.slug && api.openGogStorePage(item.slug)}
                  >
                    {item.slug ? t("gog_open_store_page") : t("gog_no_store_page")}
                  </button>
                </div>
              </div>
            )}
          </For>
        </div>
      </div>

      <p style={{ color: "var(--text-muted)", "max-width": "640px", "margin-bottom": "24px" }}>
        Przeglądanie sklepów odbywa się w tej samej sesji Hacker Mode — bez
        wychodzenia do osobnej przeglądarki. Zakupione/zainstalowane gry
        pojawią się automatycznie w zakładce „Biblioteka”.
      </p>

      <div class="grid" style={{ "grid-template-columns": "repeat(auto-fill, minmax(260px, 1fr))" }}>
        <For each={STOREFRONTS}>
          {(front) => (
            <div class="game-card">
              <div class="title">{front.label}</div>
              <div style={{ color: "var(--text-muted)", "font-size": "12px" }}>{front.hint}</div>
              <button data-focusable tabIndex={0} class="primary-btn" onClick={() => openStorefront(front)}>
                Otwórz
              </button>
            </div>
          )}
        </For>
      </div>
    </div>
  );
};

export default Store;
