use super::{steam_shortcuts, tool_available, Game, LoginFlow, Platform, StoreError, StoreProvider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Default)]
pub struct SteamProvider;

impl SteamProvider {
    /// Steam (natywny albo Flatpak) domyślnie trzyma dane w jednym z tych
    /// katalogów — sprawdzamy po kolei.
    fn root_candidates() -> Vec<PathBuf> {
        let home = dirs::home_dir().unwrap_or_default();
        vec![
            home.join(".steam/steam"),
            home.join(".steam/root"),
            home.join(".local/share/Steam"),
            home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"),
            home.join(".var/app/com.valvesoftware.Steam/.steam/steam"),
        ]
    }

    pub(crate) fn find_root() -> Option<PathBuf> {
        Self::root_candidates().into_iter().find(|p| p.exists())
    }

    /// Automatyczne wykrycie SteamID64 z lokalnego pliku Steam
    /// (`config/loginusers.vdf`) — zwykły tekstowy VDF (ten sam format co
    /// `appmanifest_*.acf`, w przeciwieństwie do binarnego
    /// `shortcuts.vdf`), w którym Steam trzyma listę kont, które
    /// kiedykolwiek zalogowały się na tej maszynie. Klucze najwyższego
    /// poziomu pod `"users"` to SAME SteamID64 (nie osobne pole) —
    /// struktura wygląda tak:
    /// ```text
    /// "users"
    /// {
    ///     "76561198012345678"
    ///     {
    ///         "AccountName"  "..."
    ///         "MostRecent"   "1"
    ///     }
    /// }
    /// ```
    /// Pozwala to Hacker Mode wypełnić `Settings::steam_id64` samodzielnie
    /// (patrz `commands::list_games`) — użytkownik musi ręcznie podać już
    /// tylko klucz Steam Web API (to prywatny sekret per-deweloper, nie da
    /// się go w żaden sposób wyczytać lokalnie), zamiast dwóch osobnych
    /// czynności (w tym szukania własnego SteamID64 przez zewnętrzne
    /// narzędzia typu steamid.io).
    ///
    /// Jeśli zalogowanych było więcej kont, wybieramy to oznaczone
    /// `"MostRecent" "1"` — a w braku takiego oznaczenia, po prostu
    /// pierwsze znalezione (lepsze to niż nic).
    pub fn find_local_steamid64() -> Option<String> {
        let root = Self::find_root()?;
        let content = std::fs::read_to_string(root.join("config/loginusers.vdf")).ok()?;
        // `VdfValue::parse` spłaszcza nazwę bloku najwyższego poziomu (tu:
        // "users") — `parsed` to już bezpośrednio mapa
        // { "<steamid64>": { "AccountName": ..., "MostRecent": ... }, ... },
        // BEZ dodatkowego zagnieżdżenia pod kluczem "users".
        let parsed = VdfValue::parse(&content);
        let users = parsed.as_map()?;

        let most_recent = users.iter().find(|(_, v)| {
            v.as_map()
                .and_then(|m| m.get("MostRecent"))
                .and_then(VdfValue::as_str)
                == Some("1")
        });

        most_recent
            .or_else(|| users.iter().next())
            .map(|(steamid, _)| steamid.clone())
    }

    fn library_folders(root: &Path) -> Vec<PathBuf> {
        let mut libraries = vec![root.join("steamapps")];
        let vdf_path = root.join("steamapps/libraryfolders.vdf");
        if let Ok(content) = std::fs::read_to_string(&vdf_path) {
            let parsed = VdfValue::parse(&content);
            if let Some(map) = parsed.as_map() {
                for (_key, value) in map {
                    // Struktura: "0" => { "path" => "...", ... }, "1" => {...}
                    if let Some(entry) = value.as_map() {
                        if let Some(path_str) = entry.get("path").and_then(VdfValue::as_str) {
                            libraries.push(PathBuf::from(path_str).join("steamapps"));
                        }
                    }
                }
            }
        }

        // BUGFIX: `libraryfolders.vdf` zwyczajowo wymienia RÓWNIEŻ domyślną
        // bibliotekę (`root/steamapps`), którą już dodaliśmy ręcznie wyżej
        // — bez deduplikacji ten sam katalog jest skanowany dwa razy,
        // efekt: każda gra/narzędzie z domyślnej biblioteki pojawia się w
        // Hacker Mode PODWÓJNIE (dokładnie to widać na zrzucie ekranu:
        // "Proton 10.0" duplikuje się). Kanonizujemy ścieżki (rozwiązuje
        // to też przypadek, gdy dwie ścieżki wskazują to samo miejsce przez
        // symlink) i odrzucamy powtórki, zachowując pierwsze wystąpienie.
        let mut seen = std::collections::HashSet::new();
        libraries
            .into_iter()
            .filter(|lib| {
                let canonical = std::fs::canonicalize(lib).unwrap_or_else(|_| lib.clone());
                seen.insert(canonical)
            })
            .collect()
    }
}

/// Steam trzyma lokalny cache okładek/artworku bibliotek w
/// `appcache/librarycache`. Format nazw plików zmieniał się na przestrzeni
/// lat (starszy: `<appid>_library_600x900.jpg` bezpośrednio w
/// `librarycache/`; nowszy: `librarycache/<appid>/library_600x900.jpg`),
/// więc sprawdzamy oba warianty i kilka rozszerzeń.
fn find_cover_art(root: &Path, appid: &str) -> Option<String> {
    let cache_dir = root.join("appcache").join("librarycache");
    let candidates = [
        cache_dir.join(format!("{appid}_library_600x900.jpg")),
        cache_dir.join(format!("{appid}_library_600x900.png")),
        cache_dir.join(appid).join("library_600x900.jpg"),
        cache_dir.join(appid).join("library_600x900.png"),
    ];
    candidates
        .into_iter()
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().into_owned())
}

/// BUGFIX: `appmanifest_*.acf` (skanowane w `list_games`) opisuje KAŻDĄ
/// zainstalowaną "aplikację" Steam tym samym formatem — łącznie z własnymi
/// narzędziami kompatybilności Valve (Proton, Steam Linux Runtime), które
/// instalują się automatycznie przy pierwszym uruchomieniu jakiejkolwiek
/// gry z Windows przez Proton. Prawdziwy klient Steam wie, że to nie gry,
/// bo sprawdza pole `common.type` w swoim lokalnym binarnym cache
/// `appinfo.vdf` (`"Tool"` zamiast `"Game"`) — ten plik ma udokumentowany,
/// ale nietrywialny format binarny, którego nie parsujemy (żeby nie
/// zgadywać jego układu bajtów bez pewnego źródła). Zamiast tego filtrujemy
/// po NAZWIE: Valve od lat konsekwentnie nazywa te narzędzia tymi samymi,
/// przewidywalnymi prefiksami (potwierdzone bezpośrednio na zrzucie ekranu
/// zgłoszonym przez użytkownika: "Proton 10.0", "Proton Experimental",
/// "Steam Linux Runtime 4.0", "Steam Linux Runtime 3.0 (sniper)") — dzięki
/// dopasowaniu po prefiksie działa to też dla przyszłych wersji (np.
/// "Proton 11.0"), bez potrzeby aktualizowania listy przy każdym wydaniu.
fn is_valve_compat_tool(name: &str) -> bool {
    const NON_GAME_PREFIXES: &[&str] = &[
        "Proton",
        "Steam Linux Runtime",
        "Steamworks Common Redistributables",
    ];
    NON_GAME_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

impl StoreProvider for SteamProvider {
    fn platform(&self) -> Platform {
        Platform::Steam
    }

    fn is_available(&self) -> bool {
        tool_available("steam") || tool_available("flatpak") || Self::find_root().is_some()
    }

    fn is_logged_in(&self) -> bool {
        // Steam zarządza logowaniem samodzielnie (własne GUI); tu jedynie
        // sprawdzamy, czy `loginusers.vdf` ma choć jeden wpis, żeby dać
        // UI sensowną wskazówkę ("otwórz Steam i zaloguj się").
        Self::find_root()
            .map(|root| {
                std::fs::read_to_string(root.join("config/loginusers.vdf"))
                    .map(|c| c.contains("\"AccountName\""))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        // Steam ma własne, natywne GUI logowania — nie ma tu żadnego kodu
        // do wklejenia. Po prostu odpalamy klienta.
        Ok(LoginFlow {
            url: None,
            instructions: "Otwórz klienta Steam i zaloguj się na swoje konto — Hacker Mode \
                automatycznie wykryje zainstalowane gry po zalogowaniu."
                .to_string(),
            needs_code: false,
        })
    }

    fn install_command(&self, game_id: &str) -> Result<Command, StoreError> {
        self.launch_command_with_action(game_id, "install")
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        self.launch_command_with_action(game_id, "uninstall")
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let root = Self::find_root().ok_or_else(|| {
            StoreError::ToolNotInstalled("katalog danych Steam (~/.steam/steam)".into())
        })?;

        let libraries = Self::library_folders(&root);
        tracing::debug!(root = %root.display(), libraries = ?libraries, "Steam: wykryte biblioteki");

        let mut games = Vec::new();
        let mut manifests_seen = 0usize;
        let mut filtered_as_tool = 0usize;
        for lib in &libraries {
            let entries = match std::fs::read_dir(lib) {
                Ok(e) => e,
                Err(err) => {
                    tracing::debug!(dir = %lib.display(), error = %err, "Steam: nie udało się odczytać katalogu biblioteki");
                    continue;
                }
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let is_manifest = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("appmanifest_") && n.ends_with(".acf"))
                    .unwrap_or(false);
                if !is_manifest {
                    continue;
                }
                manifests_seen += 1;
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let parsed = VdfValue::parse(&content);
                let Some(map) = parsed.as_map() else {
                    continue;
                };
                let Some(appid) = map.get("appid").and_then(VdfValue::as_str) else {
                    continue;
                };
                let name = map
                    .get("name")
                    .and_then(VdfValue::as_str)
                    .unwrap_or("Nieznana gra")
                    .to_string();

                if is_valve_compat_tool(&name) {
                    filtered_as_tool += 1;
                    continue;
                }

                let install_dir = map
                    .get("installdir")
                    .and_then(VdfValue::as_str)
                    .map(|d| lib.join("common").join(d).to_string_lossy().into_owned());

                games.push(Game {
                    id: appid.to_string(),
                    title: name,
                    platform: Platform::Steam,
                    installed: true,
                    install_dir,
                    cover_path: find_cover_art(&root, appid),
                    playtime_minutes: None, // Steam nie trzyma tego lokalnie w prosty sposób — wymaga Web API + klucza.
                });
            }
        }

        // Zabezpieczenie dodatkowe (poza deduplikacją folderów bibliotek w
        // `library_folders`) — gdyby ten sam appid trafił się z innego
        // powodu więcej niż raz (np. gra przeniesiona między dwiema
        // bibliotekami bez usunięcia starego manifestu), zachowujemy
        // pierwsze wystąpienie i odrzucamy resztę.
        let mut seen_appids = std::collections::HashSet::new();
        games.retain(|g| seen_appids.insert(g.id.clone()));

        // Diagnostyka: gdyby ktoś zgłosił "0 gier mimo zainstalowanych
        // gier w Steam", to jest pierwsze miejsce do sprawdzenia w logu —
        // pokazuje dokładnie, ile plików appmanifest w ogóle znaleziono,
        // ile odfiltrowano jako narzędzia Valve (Proton/Runtime) i ile
        // finalnie zostało uznane za gry. Jeśli `manifests_seen` jest 0,
        // problem jest w wykrywaniu bibliotek (`root`/`library_folders`,
        // patrz log wyżej), nie w samym filtrze.
        tracing::info!(
            manifests_seen,
            filtered_as_tool,
            games_kept = games.len(),
            "Steam: podsumowanie skanowania biblioteki (appmanifest)"
        );

        // Skróty "Add a Non-Steam Game to My Library" (np. Xenonauts.exe,
        // Carrion.exe) — CAŁKOWICIE inny mechanizm niż appmanifest,
        // wcześniej w ogóle nieobsługiwany przez Hacker Mode (stąd nie
        // pojawiały się w bibliotece, mimo że są widoczne w prawdziwym
        // kliencie Steam). Patrz moduł-dokumentacja `steam_shortcuts.rs`.
        let shortcuts = steam_shortcuts::find_all_shortcuts(&root);
        tracing::info!(shortcuts_found = shortcuts.len(), "Steam: podsumowanie skanowania shortcuts.vdf");
        for shortcut in shortcuts {
            games.push(Game {
                // 64-bitowa forma `rungameid` zamiast surowego appidu ze
                // skrótu — patrz `steam_shortcuts::rungameid_64` i jego
                // moduł-dokumentacja (`launch_command` niżej buduje
                // `steam://rungameid/<id>` z tego samego pola `id` dla
                // WSZYSTKICH gier Steam, więc to musi być już właściwa,
                // gotowa do użycia wartość).
                id: steam_shortcuts::rungameid_64(shortcut.appid).to_string(),
                title: shortcut.app_name,
                platform: Platform::Steam,
                // Plik wykonywalny już leży na dysku (dodany ręcznie przez
                // użytkownika) — z perspektywy Hacker Mode to zawsze
                // "zainstalowane", nie ma tu koncepcji instalacji/
                // odinstalowania przez Steam.
                installed: true,
                install_dir: shortcut.exe,
                // Steam trzyma niestandardowe okładki tych skrótów w
                // `userdata/<id>/config/grid/`, ale w innym, niestandardowym
                // formacie nazewnictwa niż `find_cover_art` niżej obsługuje
                // dla zwykłych appidów — świadomie zostawione na potem
                // zamiast zgadywane teraz.
                cover_path: None,
                playtime_minutes: None,
            });
        }

        Ok(games)
    }

    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError> {
        // BUGFIX (zgłoszone przez użytkownika): `steam://rungameid/<id>`
        // działa niezawodnie TYLKO gdy klient Steam już jest uruchomiony —
        // przy zimnym starcie (Steam jeszcze nie działa) to wywołanie
        // często tylko odpala sam klient Steam i NIE doprowadza do
        // uruchomienia konkretnej gry, więc użytkownik musiał ręcznie
        // odpalić Steam osobno, zanim cokolwiek dało się uruchomić z
        // Hacker Mode. `-applaunch <appid>` to udokumentowany, od dawna
        // istniejący przełącznik wiersza poleceń Steam PRZEZNACZONY
        // dokładnie do tego przypadku (uruchamianie gry z zewnętrznego
        // launchera) — działa identycznie niezależnie od tego, czy Steam
        // już działa (wtedy przekazuje żądanie przez IPC do działającej
        // instancji) czy nie (wtedy sam odpala Steam i, po jego
        // uruchomieniu, dołącza uruchomienie gry) — potwierdzone
        // wieloma niezależnymi źródłami społeczności (m.in. deweloperzy
        // narzędzi trzecich integrujących się ze Steam na Linuksie).
        self.launch_command_with_action_applaunch(game_id)
    }
}

/// Pobiera czas gry (w minutach) dla wszystkich gier na koncie przez
/// oficjalne Steam Web API (`IPlayerService/GetOwnedGames`). Wymaga klucza
/// API (https://steamcommunity.com/dev/apikey) i SteamID64 — Steam nie
/// udostępnia tej informacji lokalnie w plikach `appmanifest_*.acf` w
/// żaden prosty sposób (tylko globalny, niepowiązany z konkretną grą
/// znacznik czasu ostatniego uruchomienia). Błędy sieciowe/braku
/// uprawnień są ciche — zwracana jest po prostu pusta mapa, żeby biblioteka
/// nadal się wyświetliła (bez czasu gry) zamiast się wywalić.
/// Dociąga opis i zrzuty ekranu z publicznego (nieuwierzytelnionego) Steam
/// Store API (`store.steampowered.com/api/appdetails`) — używane wyłącznie
/// do wzbogacenia widoku szczegółów gry, nigdy do logiki
/// instalacji/uruchamiania.
pub fn fetch_store_details(appid: &str) -> Option<super::GameDetails> {
    let url = format!("https://store.steampowered.com/api/appdetails?appids={appid}&l=polish");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .ok()?;
    let response = client.get(&url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let parsed: serde_json::Value = response.json().ok()?;
    let entry = parsed.get(appid)?;
    if !entry.get("success")?.as_bool().unwrap_or(false) {
        return None;
    }
    let data = entry.get("data")?;

    let description = data
        .get("short_description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let screenshots = data
        .get("screenshots")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.get("path_thumbnail").and_then(|p| p.as_str()))
                .map(String::from)
                .take(6)
                .collect()
        })
        .unwrap_or_default();

    Some(super::GameDetails { description, screenshots })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSearchResult {
    pub appid: u64,
    pub name: String,
    pub cover_url: Option<String>,
    pub price: Option<String>,
}

/// Przeszukuje katalog Steam przez publiczne, nieuwierzytelnione API
/// `store.steampowered.com/api/storesearch` — to jest realne "natywne"
/// przeglądanie sklepu (dane, nie strona WWW w webview), używane w
/// `Store.tsx`. Wyniki prowadzą do `steam://store/<appid>`, które otwiera
/// stronę produktu we wbudowanej przeglądarce klienta Steam — Hacker Mode
/// nie odtwarza całego procesu zakupu, tylko wyszukiwanie i przejście do
/// niego.
pub fn search_store(query: &str) -> Vec<StoreSearchResult> {
    let url = format!(
        "https://store.steampowered.com/api/storesearch/?term={}&cc=pl&l=polish",
        urlencoding_minimal(query)
    );

    let result: Option<Vec<StoreSearchResult>> = (|| {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .ok()?;
        let response = client.get(&url).send().ok()?;
        if !response.status().is_success() {
            return None;
        }
        let parsed: serde_json::Value = response.json().ok()?;
        let items = parsed.get("items")?.as_array()?;
        Some(
            items
                .iter()
                .filter_map(|item| {
                    Some(StoreSearchResult {
                        appid: item.get("id")?.as_u64()?,
                        name: item.get("name")?.as_str()?.to_string(),
                        cover_url: item.get("tiny_image").and_then(|v| v.as_str()).map(String::from),
                        price: item
                            .get("price")
                            .and_then(|p| p.get("final_formatted"))
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    })
                })
                .collect(),
        )
    })();

    result.unwrap_or_default()
}

/// Minimalne url-encode dla zapytania (spacje/znaki specjalne) — bez
/// dodawania zależności `urlencoding`, bo potrzebujemy tylko obsłużyć
/// zwykły tekst wyszukiwania. `pub(crate)`, bo `steamgriddb.rs` (patrz
/// wyszukiwanie po tytule dla EA/Battle.net) i `gog.rs` (`search_store`)
/// współdzielą tę samą, bardzo prostą potrzebę zamiast każde utrzymywać
/// własną kopię.
pub(crate) fn urlencoding_minimal(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        let c = *byte as char;
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct OwnedSteamGame {
    pub appid: String,
    pub name: String,
    pub playtime_minutes: u64,
}

/// Pełna lista gier **posiadanych** na koncie Steam (nie tylko
/// zainstalowanych) przez oficjalne `IPlayerService/GetOwnedGames`.
///
/// BUGFIX: `SteamProvider::list_games` (skan lokalnych
/// `appmanifest_*.acf`) z natury rzeczy widzi WYŁĄCZNIE gry już
/// zainstalowane na tej maszynie — Steam nie trzyma nigdzie lokalnie
/// pełnej listy posiadanych gier. Dlatego biblioteka Steam w Hacker Mode
/// pokazywała tylko zainstalowane tytuły, mimo że użytkownik mógł mieć
/// dziesiątki innych gier na koncie. Ta funkcja (wywoływana z
/// `commands::stores::list_all_games`, gdy w ustawieniach jest klucz API +
/// SteamID64 — dokładnie te same dane co przy dociąganiu czasu gry) daje
/// pełną listę, którą `list_all_games` scala z wynikiem lokalnego skanu:
/// gry już obecne (zainstalowane) zostają jak są (z lokalną okładką z
/// cache Steam), a te tylko *posiadane* dochodzą jako `installed: false`
/// (patrz `Game` w tej samej funkcji wywołującej), więc dają się
/// zainstalować wprost z Hacker Mode.
///
/// `include_appinfo=true` (w przeciwieństwie do wcześniejszej wersji tej
/// funkcji, wywoływanej tylko po czas gry) — potrzebujemy teraz też nazwy
/// gry, żeby dało się ją wyświetlić bez lokalnych metadanych.
pub fn fetch_owned_games(api_key: &str, steam_id64: &str) -> Vec<OwnedSteamGame> {
    let url = format!(
        "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?key={api_key}&steamid={steam_id64}&format=json&include_appinfo=true&include_played_free_games=true"
    );

    let result: Option<Vec<OwnedSteamGame>> = (|| {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .ok()?;
        let response = client.get(&url).send().ok()?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "Steam Web API zwróciło błąd (sprawdź klucz API / SteamID64 / czy profil jest publiczny)");
            return None;
        }
        let parsed: serde_json::Value = response.json().ok()?;
        let games = parsed.get("response")?.get("games")?.as_array()?;
        let mut out = Vec::new();
        for game in games {
            let Some(appid) = game.get("appid").and_then(|v| v.as_u64()) else {
                continue;
            };
            let name = game
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Nieznana gra")
                .to_string();
            let minutes = game.get("playtime_forever").and_then(|v| v.as_u64()).unwrap_or(0);
            out.push(OwnedSteamGame { appid: appid.to_string(), name, playtime_minutes: minutes });
        }
        Some(out)
    })();

    result.unwrap_or_default()
}

/// Status "w trakcie gry" pobrany z oficjalnego Steam Web API
/// (`ISteamUser/GetPlayerSummaries/v2`) — jedyny wiarygodny sposób
/// śledzenia REALNEGO stanu gry Steam (patrz README, „Do zrobienia
/// dalej” i moduł-dokumentacja `launcher::watch_steam_web_api_session`):
/// `steam -applaunch <appid>` tylko przekazuje żądanie do już
/// działającego klienta Steam przez jego wewnętrzne IPC i kończy się
/// PRAWIE NATYCHMIAST, więc obserwowanie PID-u TEGO polecenia (jak dla
/// pozostałych platform, patrz `AppState::running_games`) nie mówi nic
/// wiarygodnego o tym, czy gra faktycznie wciąż działa.
///
/// Kształt odpowiedzi (`response.players[0].gameid`/`gameextrainfo`) to
/// dobrze udokumentowane, publiczne pole Steam Web API — w
/// przeciwieństwie do np. kształtu `catalog.gog.com` (patrz `gog.rs`)
/// nie jest to research po nieoficjalnych źródłach, tylko ten sam
/// endpoint, którego już używa `fetch_owned_games`/`fetch_achievements`
/// wyżej w tym pliku, z tymi samymi poświadczeniami
/// (`Settings::steam_api_key`/`steam_id64`).
///
/// Zwraca:
/// - `None` — zapytanie się nie powiodło (sieć/nieprawidłowy klucz/
///   nieoczekiwany kształt JSON-a) LUB profil gracza nie jest publiczny.
///   Wywołujący (patrz `launcher::watch_steam_web_api_session`) MUSI to
///   zignorować (potraktować jako "nie wiadomo", nie jako "gra się
///   zamknęła") — pojedynczy błąd zapytania nie oznacza, że sesja gry
///   faktycznie dobiegła końca.
/// - `Some(None)` — zapytanie się powiodło, ale gracz AKTUALNIE nie jest
///   w żadnej grze (pole `gameid` nieobecne albo puste).
/// - `Some(Some(appid))` — zapytanie się powiodło, gracz gra w grę o
///   danym Steam AppID.
pub fn fetch_player_current_appid(api_key: &str, steam_id64: &str) -> Option<Option<String>> {
    let url = format!(
        "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/?key={api_key}&steamids={steam_id64}"
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .ok()?;
    let response = client.get(&url).send().ok()?;
    if !response.status().is_success() {
        tracing::debug!(status = %response.status(), "Steam Web API (GetPlayerSummaries) zwróciło błąd — sprawdź klucz API/SteamID64/czy profil jest publiczny");
        return None;
    }
    let body = response.text().ok()?;
    parse_player_summary_gameid(&body)
}

/// Rdzeń `fetch_player_current_appid` — parsowanie wydzielone od
/// samego zapytania HTTP, żeby dało się je przetestować na przykładowym
/// JSON-ie (patrz `mod tests`) bez uruchamiania prawdziwego serwera.
fn parse_player_summary_gameid(body: &str) -> Option<Option<String>> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let players = parsed.get("response")?.get("players")?.as_array()?;
    let player = players.first()?;
    let gameid = player
        .get("gameid")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    Some(gameid.map(str::to_string))
}

/// Pojedyncze osiągnięcie, już scalone z danymi z `GetSchemaForGame`
/// (nazwa/opis/ikona — `GetPlayerAchievements` sam ma tylko techniczną
/// `apiname`, patrz `fetch_achievements`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub api_name: String,
    pub display_name: String,
    pub description: String,
    pub achieved: bool,
    /// Sekundy od epoki Unix, `0` jeśli nieodblokowane — dokładnie tak,
    /// jak zwraca to samo Steam Web API (`unlocktime`), bez konwersji na
    /// coś czytelniejszego tutaj; formatowanie daty to sprawa frontendu.
    pub unlock_time: u64,
    pub icon_url: String,
}

/// Osiągnięcia gracza dla danej gry Steam — łączy DWA wywołania Steam Web
/// API, bo żadne pojedyncze nie ma wszystkiego, czego potrzebuje widok
/// szczegółów gry (`GameDetail.tsx`):
/// - `ISteamUserStats/GetPlayerAchievements` — czy dane osiągnięcie jest
///   odblokowane i kiedy, ale identyfikuje je tylko techniczną `apiname`
///   (np. `"ACH_WIN_ONE_GAME"`), bez nazwy/opisu/ikony do pokazania.
/// - `ISteamUserStats/GetSchemaForGame` — pełny SCHEMAT gry: dla każdej
///   `apiname` ma `displayName`/`description`/`icon` (odblokowana)/
///   `icongray` (zablokowana), ale nic o TYM konkretnym graczu.
///
/// Wymaga tych samych dwóch ustawień co czas gry Steam
/// (`Settings::steam_api_key`/`steam_id64`) — jeśli profil gracza nie jest
/// publiczny, `GetPlayerAchievements` zwróci błąd (Steam Web API to
/// sygnalizuje polem `"success": false` w odpowiedzi, nie kodem HTTP),
/// więc sprawdzamy oba.
pub fn fetch_achievements(api_key: &str, steam_id64: &str, appid: &str) -> Vec<Achievement> {
    let Some(client) = crate::cover_cache::build_client() else {
        return Vec::new();
    };

    let Some(schema) = fetch_achievement_schema(appid, api_key, &client) else {
        // Brak schematu = albo gra nie ma osiągnięć w ogóle, albo błąd
        // API — w obu przypadkach nie mamy jak pokazać czegokolwiek
        // sensownego (same `apiname` bez nazw byłyby bezużyteczne w UI).
        return Vec::new();
    };
    let unlocked = fetch_player_unlocks(appid, api_key, steam_id64, &client).unwrap_or_default();

    schema
        .into_iter()
        .map(|entry| {
            let progress = unlocked.get(&entry.api_name);
            Achievement {
                api_name: entry.api_name,
                display_name: entry.display_name,
                description: entry.description,
                achieved: progress.map(|p| p.achieved).unwrap_or(false),
                unlock_time: progress.map(|p| p.unlock_time).unwrap_or(0),
                icon_url: if progress.map(|p| p.achieved).unwrap_or(false) {
                    entry.icon_unlocked
                } else {
                    entry.icon_locked
                },
            }
        })
        .collect()
}

struct SchemaEntry {
    api_name: String,
    display_name: String,
    description: String,
    icon_unlocked: String,
    icon_locked: String,
}

fn fetch_achievement_schema(appid: &str, api_key: &str, client: &reqwest::blocking::Client) -> Option<Vec<SchemaEntry>> {
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?key={api_key}&appid={appid}"
    );
    let response = client.get(&url).send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().ok()?;
    parse_achievement_schema(&body)
}

/// Rdzeń `fetch_achievement_schema` — wydzielone parsowanie, testowalne
/// na przykładowym JSON-ie bez wykonywania zapytania HTTP (patrz `mod
/// tests`), ten sam wzorzec co `gog::parse_store_details`/`parse_owned_ids`.
fn parse_achievement_schema(body: &str) -> Option<Vec<SchemaEntry>> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let entries = parsed
        .get("game")?
        .get("availableGameStats")?
        .get("achievements")?
        .as_array()?;

    let out = entries
        .iter()
        .filter_map(|e| {
            Some(SchemaEntry {
                api_name: e.get("name")?.as_str()?.to_string(),
                display_name: e.get("displayName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                description: e.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                icon_unlocked: e.get("icon").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                icon_locked: e.get("icongray").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
        })
        .collect();
    Some(out)
}

struct PlayerProgress {
    achieved: bool,
    unlock_time: u64,
}

fn fetch_player_unlocks(
    appid: &str,
    api_key: &str,
    steam_id64: &str,
    client: &reqwest::blocking::Client,
) -> Option<std::collections::HashMap<String, PlayerProgress>> {
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?key={api_key}&steamid={steam_id64}&appid={appid}"
    );
    let response = client.get(&url).send().ok()?;
    // Steam Web API zgłasza "profil prywatny"/"gra bez osiągnięć" przez
    // `"success": false` w TREŚCI odpowiedzi (HTTP 200), a nie kodem
    // statusu — stąd sprawdzamy oba, żeby nie przeoczyć tego drugiego
    // przypadku i nie zwrócić po cichu pustej mapy tam, gdzie w ogóle nie
    // dało się odczytać danych gracza.
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().ok()?;
    parse_player_unlocks(appid, &body)
}

/// Rdzeń `fetch_player_unlocks` — patrz `parse_achievement_schema` po
/// wyjaśnienie, dlaczego parsowanie jest osobną, testowalną funkcją.
/// `appid` tu jest potrzebny WYŁĄCZNIE do logu debug przy `success: false`
/// — sam parsing go nie używa.
fn parse_player_unlocks(appid: &str, body: &str) -> Option<std::collections::HashMap<String, PlayerProgress>> {
    let parsed: serde_json::Value = serde_json::from_str(body).ok()?;
    let stats = parsed.get("playerstats")?;
    if stats.get("success").and_then(|v| v.as_bool()) != Some(true) {
        tracing::debug!(appid, "Steam: GetPlayerAchievements success=false (profil prywatny lub gra bez osiągnięć)");
        return None;
    }
    let achievements = stats.get("achievements")?.as_array()?;

    let mut out = std::collections::HashMap::new();
    for a in achievements {
        let Some(api_name) = a.get("apiname").and_then(|v| v.as_str()) else {
            continue;
        };
        let achieved = a.get("achieved").and_then(|v| v.as_u64()).unwrap_or(0) == 1;
        let unlock_time = a.get("unlocktime").and_then(|v| v.as_u64()).unwrap_or(0);
        out.insert(api_name.to_string(), PlayerProgress { achieved, unlock_time });
    }
    Some(out)
}

#[cfg(test)]
mod achievement_tests {
    use super::*;

    const SAMPLE_SCHEMA: &str = r#"{
        "game": {
            "gameName": "Half-Life 2",
            "availableGameStats": {
                "achievements": [
                    { "name": "ACH_WIN_ONE_GAME", "displayName": "Wygraj grę", "description": "Wygraj jedną grę", "icon": "https://example.com/unlocked.jpg", "icongray": "https://example.com/locked.jpg" },
                    { "name": "ACH_NO_DESC", "displayName": "Bez opisu" }
                ]
            }
        }
    }"#;

    #[test]
    fn parses_achievement_schema() {
        let entries = parse_achievement_schema(SAMPLE_SCHEMA).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].api_name, "ACH_WIN_ONE_GAME");
        assert_eq!(entries[0].display_name, "Wygraj grę");
        assert_eq!(entries[0].icon_unlocked, "https://example.com/unlocked.jpg");
        // Brakujące pola opcjonalne (`description` tu) mają zostać pustym
        // stringiem, nie wywalić parsowania całego wpisu.
        assert_eq!(entries[1].description, "");
    }

    #[test]
    fn parse_achievement_schema_missing_achievements_field_returns_none() {
        assert!(parse_achievement_schema(r#"{"game": {"gameName": "X"}}"#).is_none());
    }

    #[test]
    fn parse_achievement_schema_garbage_returns_none() {
        assert!(parse_achievement_schema("nie json").is_none());
    }

    const SAMPLE_PLAYER_ACHIEVEMENTS: &str = r#"{
        "playerstats": {
            "steamID": "12345",
            "gameName": "Half-Life 2",
            "achievements": [
                { "apiname": "ACH_WIN_ONE_GAME", "achieved": 1, "unlocktime": 1700000000 },
                { "apiname": "ACH_NOT_YET", "achieved": 0, "unlocktime": 0 }
            ],
            "success": true
        }
    }"#;

    const SAMPLE_PLAYER_ACHIEVEMENTS_PRIVATE: &str = r#"{
        "playerstats": {
            "success": false,
            "error": "Profile is not public"
        }
    }"#;

    #[test]
    fn parses_player_unlocks() {
        let unlocks = parse_player_unlocks("440", SAMPLE_PLAYER_ACHIEVEMENTS).unwrap();
        assert_eq!(unlocks.len(), 2);
        assert!(unlocks["ACH_WIN_ONE_GAME"].achieved);
        assert_eq!(unlocks["ACH_WIN_ONE_GAME"].unlock_time, 1700000000);
        assert!(!unlocks["ACH_NOT_YET"].achieved);
    }

    #[test]
    fn parse_player_unlocks_private_profile_returns_none() {
        assert!(parse_player_unlocks("440", SAMPLE_PLAYER_ACHIEVEMENTS_PRIVATE).is_none());
    }

    #[test]
    fn parse_player_unlocks_garbage_returns_none() {
        assert!(parse_player_unlocks("440", "nie json").is_none());
    }
}

/// Publiczny, znany od lat adres CDN Steam dla okładek biblioteki
/// (`library_600x900`) — dokładnie ten sam wzorzec, którego bez
/// uwierzytelniania używają np. SteamDB i inne narzędzia community. Nie
/// wymaga pobierania/cache'owania lokalnie (w przeciwieństwie do
/// `cover_cache` dla Epic/GOG/Amazon) — działa jako bezpośredni URL w
/// `<img src>`, więc używany tylko dla gier POSIADANYCH, ale
/// NIEZAINSTALOWANYCH (te zainstalowane mają już lepszą, lokalnie
/// pobraną okładkę z `find_cover_art`, patrz `list_games`).
pub fn owned_game_cdn_cover_url(appid: &str) -> String {
    format!("https://cdn.cloudflare.steamstatic.com/steam/apps/{appid}/library_600x900.jpg")
}

impl SteamProvider {
    /// Steam obsługuje kilka akcji przez protokół `steam://`:
    /// `rungameid/<id>` (uruchom), `install/<id>` (zainstaluj). Wspólna
    /// funkcja, żeby nie duplikować logiki wyboru `steam` vs Flatpak.
    fn launch_command_with_action(&self, game_id: &str, action: &str) -> Result<Command, StoreError> {
        if tool_available("steam") {
            let mut cmd = Command::new("steam");
            cmd.arg(format!("steam://{action}/{game_id}"));
            Ok(cmd)
        } else if tool_available("flatpak") {
            let mut cmd = Command::new("flatpak");
            cmd.args([
                "run",
                "com.valvesoftware.Steam",
                &format!("steam://{action}/{game_id}"),
            ]);
            Ok(cmd)
        } else {
            Err(StoreError::ToolNotInstalled("steam".into()))
        }
    }

    /// Patrz `launch_command` po pełne uzasadnienie, dlaczego to NIE jest
    /// zwykłe `launch_command_with_action(id, "rungameid")` — `-applaunch`
    /// to osobny przełącznik wiersza poleceń (nie URI `steam://`), więc
    /// wymaga osobnej budowy argumentów zamiast współdzielenia
    /// `launch_command_with_action`.
    fn launch_command_with_action_applaunch(&self, game_id: &str) -> Result<Command, StoreError> {
        if tool_available("steam") {
            let mut cmd = Command::new("steam");
            cmd.args(["-applaunch", game_id]);
            Ok(cmd)
        } else if tool_available("flatpak") {
            let mut cmd = Command::new("flatpak");
            cmd.args(["run", "com.valvesoftware.Steam", "-applaunch", game_id]);
            Ok(cmd)
        } else {
            Err(StoreError::ToolNotInstalled("steam".into()))
        }
    }
}

/// Bardzo mały parser formatu VDF (Valve KeyValue) — wystarczający do
/// odczytu `libraryfolders.vdf` i `appmanifest_*.acf`. Format:
///
/// ```text
/// "AppState"
/// {
///     "appid"     "123"
///     "name"      "Przykładowa Gra"
///     "nested"
///     {
///         "a" "1"
///     }
/// }
/// ```
#[derive(Debug, Clone)]
enum VdfValue {
    Str(String),
    Map(HashMap<String, VdfValue>),
}

impl VdfValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            VdfValue::Str(s) => Some(s),
            _ => None,
        }
    }

    fn as_map(&self) -> Option<&HashMap<String, VdfValue>> {
        match self {
            VdfValue::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Parsuje cały dokument VDF i zwraca korzeń jako `Map` (spłaszczając
    /// nazwę bloku najwyższego poziomu, np. `"AppState" { ... }` ->
    /// mapę z zawartością `{ ... }`).
    fn parse(input: &str) -> VdfValue {
        let tokens = tokenize(input);
        let mut pos = 0usize;
        // Pierwszy token to nazwa bloku najwyższego poziomu — pomijamy go
        // i parsujemy sam blok `{ ... }`.
        if tokens.len() >= 2 {
            pos = 1;
        }
        parse_block(&tokens, &mut pos)
    }
}

#[derive(Debug, Clone)]
enum Token {
    Str(String),
    Open,
    Close,
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                let mut s = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        s.push(chars[i + 1]);
                        i += 2;
                    } else {
                        s.push(chars[i]);
                        i += 1;
                    }
                }
                i += 1; // zamykający cudzysłów
                tokens.push(Token::Str(s));
            }
            '{' => {
                tokens.push(Token::Open);
                i += 1;
            }
            '}' => {
                tokens.push(Token::Close);
                i += 1;
            }
            c if c.is_whitespace() => {
                i += 1;
            }
            _ => {
                i += 1; // ignorujemy komentarze / nieobsługiwane znaki
            }
        }
    }
    tokens
}

fn parse_block(tokens: &[Token], pos: &mut usize) -> VdfValue {
    // Oczekujemy `Open` na start bloku; jeśli go nie ma (np. pusty plik),
    // zwracamy pustą mapę.
    if matches!(tokens.get(*pos), Some(Token::Open)) {
        *pos += 1;
    }
    let mut map = HashMap::new();
    while let Some(tok) = tokens.get(*pos) {
        match tok {
            Token::Close => {
                *pos += 1;
                break;
            }
            Token::Str(key) => {
                let key = key.clone();
                *pos += 1;
                match tokens.get(*pos) {
                    Some(Token::Str(value)) => {
                        map.insert(key, VdfValue::Str(value.clone()));
                        *pos += 1;
                    }
                    Some(Token::Open) => {
                        let nested = parse_block(tokens, pos);
                        map.insert(key, nested);
                    }
                    _ => {}
                }
            }
            Token::Open => {
                // Blok bez klucza — pomijamy (nieprawidłowy VDF).
                let _ = parse_block(tokens, pos);
            }
        }
    }
    VdfValue::Map(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Realny kształt `ISteamUser/GetPlayerSummaries/v2` dla gracza W
    /// TRAKCIE gry — pole `gameid` to Steam AppID jako string (dobrze
    /// udokumentowane publiczne API, patrz komentarz przy
    /// `fetch_player_current_appid`).
    #[test]
    fn parses_player_summary_when_in_game() {
        let body = r#"{
            "response": {
                "players": [
                    {
                        "steamid": "76561197960287930",
                        "personaname": "Gracz",
                        "gameid": "440",
                        "gameextrainfo": "Team Fortress 2"
                    }
                ]
            }
        }"#;
        assert_eq!(parse_player_summary_gameid(body), Some(Some("440".to_string())));
    }

    #[test]
    fn parses_player_summary_when_not_in_game() {
        // Poza grą Steam po prostu NIE wysyła pól `gameid`/`gameextrainfo`
        // w ogóle (nie puste stringi) — obie sytuacje muszą dać ten sam
        // wynik `Some(None)`, patrz też test niżej.
        let body = r#"{
            "response": {
                "players": [
                    { "steamid": "76561197960287930", "personaname": "Gracz" }
                ]
            }
        }"#;
        assert_eq!(parse_player_summary_gameid(body), Some(None));
    }

    #[test]
    fn parses_player_summary_treats_empty_gameid_as_not_in_game() {
        let body = r#"{"response": {"players": [{"gameid": ""}]}}"#;
        assert_eq!(parse_player_summary_gameid(body), Some(None));
    }

    #[test]
    fn parses_player_summary_returns_none_for_empty_players_list() {
        // Profil prywatny/nieprawidłowe SteamID64 — API odpowiada
        // sukcesem, ale z pustą listą graczy. Wywołujący (patrz
        // `launcher::watch_steam_web_api_session`) musi to potraktować
        // jak błąd zapytania (`None`), NIE jak "gra się zamknęła".
        let body = r#"{"response": {"players": []}}"#;
        assert_eq!(parse_player_summary_gameid(body), None);
    }

    #[test]
    fn parses_player_summary_returns_none_for_malformed_json() {
        assert_eq!(parse_player_summary_gameid("to nie jest JSON"), None);
    }

    #[test]
    fn finds_most_recent_steamid64_from_loginusers_vdf() {
        let sample = r#"
"users"
{
    "76561197960287930"
    {
        "AccountName"       "oldaccount"
        "PersonaName"       "Old"
        "RememberPassword"  "1"
        "MostRecent"        "0"
    }
    "76561198012345678"
    {
        "AccountName"       "mainaccount"
        "PersonaName"       "Main"
        "RememberPassword"  "1"
        "MostRecent"        "1"
    }
}
"#;
        let parsed = VdfValue::parse(sample);
        let users = parsed.as_map().expect("root powinien być mapą");
        assert_eq!(users.len(), 2, "obie konta powinny się sparsować");

        let most_recent = users
            .iter()
            .find(|(_, v)| {
                v.as_map()
                    .and_then(|m| m.get("MostRecent"))
                    .and_then(VdfValue::as_str)
                    == Some("1")
            })
            .map(|(steamid, _)| steamid.clone());

        assert_eq!(most_recent.as_deref(), Some("76561198012345678"));
    }

    #[test]
    fn parses_simple_acf() {
        let sample = r#"
        "AppState"
        {
            "appid"     "440"
            "name"      "Team Fortress 2"
            "installdir" "Team Fortress 2"
        }
        "#;
        let parsed = VdfValue::parse(sample);
        let map = parsed.as_map().unwrap();
        assert_eq!(map.get("appid").unwrap().as_str().unwrap(), "440");
        assert_eq!(
            map.get("name").unwrap().as_str().unwrap(),
            "Team Fortress 2"
        );
    }

    #[test]
    fn filters_out_valve_compat_tools_by_name() {
        assert!(is_valve_compat_tool("Proton 10.0"));
        assert!(is_valve_compat_tool("Proton Experimental"));
        assert!(is_valve_compat_tool("Steam Linux Runtime 4.0"));
        assert!(is_valve_compat_tool("Steam Linux Runtime 3.0 (sniper)"));
        assert!(is_valve_compat_tool("Steamworks Common Redistributables"));
        assert!(!is_valve_compat_tool("Dying Light"));
        assert!(!is_valve_compat_tool("No Man's Sky"));
    }

    #[test]
    fn parses_nested_library_folders() {
        let sample = r#"
        "libraryfolders"
        {
            "0"
            {
                "path" "/mnt/games"
                "apps"
                {
                    "440" "12345"
                }
            }
        }
        "#;
        let parsed = VdfValue::parse(sample);
        let map = parsed.as_map().unwrap();
        let entry0 = map.get("0").unwrap().as_map().unwrap();
        assert_eq!(entry0.get("path").unwrap().as_str().unwrap(), "/mnt/games");
    }
}
