use super::{Game, Platform, StoreError, StoreProvider};
use std::process::Command;

/// Ręczne nadpisanie prefiksu Wine z ustawień (patrz
/// `Settings::ea_wine_prefix`) — `None` = poleganie na autodetekcji
/// `ea_cli::find_prefix` (domyślne lokalizacje Lutrisa).
pub struct EaProvider {
    pub explicit_prefix: Option<std::path::PathBuf>,
}

impl Default for EaProvider {
    fn default() -> Self {
        // Tak jak inne providery (`XProvider::default()`) są tworzone bez
        // wstrzykiwania zależności w każdym miejscu użycia w `mod.rs`, ten
        // provider sam doczytuje swoje jedyne dodatkowe ustawienie wprost
        // z `config.json` — to tani odczyt małego pliku JSON, a pozwala
        // zachować jednolity wzorzec `Platform::Ea => EaProvider::default()`
        // używany we wszystkich funkcjach agregujących w `mod.rs`.
        Self {
            explicit_prefix: crate::settings::Settings::load_or_default()
                .ea_wine_prefix
                .map(std::path::PathBuf::from),
        }
    }
}

impl From<ea_cli::EaError> for StoreError {
    fn from(err: ea_cli::EaError) -> Self {
        match err {
            ea_cli::EaError::WineNotInstalled => StoreError::ToolNotInstalled("wine".into()),
            other => StoreError::ExecFailed("ea-app".into(), other.to_string()),
        }
    }
}

impl StoreProvider for EaProvider {
    fn platform(&self) -> Platform {
        Platform::Ea
    }

    fn is_available(&self) -> bool {
        // Warunek "dostępności" tej platformy to nie tylko obecność `wine`
        // (jak w większości innych sprawdzeń `is_available`) — bez
        // wykrytego prefiksu z faktycznie zainstalowanym EA app nie ma
        // czego pokazywać w bibliotece, więc pomijamy ją całkowicie
        // (patrz `list_all_games`), zamiast pokazywać pustą zakładkę.
        ea_cli::is_available() && ea_cli::find_prefix(self.explicit_prefix.as_deref()).is_ok()
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let prefix = ea_cli::find_prefix(self.explicit_prefix.as_deref())?;
        let games = ea_cli::list_installed_games(&prefix)?;

        // Zapasowe okładki przez SteamGridDB (patrz `steamgriddb.rs`) —
        // WYŁĄCZNIE gdy w Ustawieniach jest ustawiony klucz API; bez niego
        // to zwykłe `None`, dokładnie jak przed dodaniem tej integracji.
        // Wyszukiwanie idzie po TYTULE (jedyne, co tu mamy — patrz
        // moduł-dokumentacja `ea-cli`), więc, w przeciwieństwie do
        // Epic/GOG (ID produktu), może się czasem pomylić — stąd zawsze
        // nadpisywalne ręcznie przez `commands::set_game_cover`.
        let api_key = crate::settings::Settings::load_or_default().steamgriddb_api_key;
        let covers = fetch_covers_if_configured(&games, api_key.as_deref(), "ea");

        Ok(games
            .into_iter()
            .zip(covers)
            .map(|(g, cover_path)| Game {
                id: g.id,
                title: g.title,
                platform: Platform::Ea,
                installed: true,
                install_dir: Some(g.install_dir.to_string_lossy().into_owned()),
                cover_path,
                // Czas gry: brak własnego API EA — uzupełniane zapasowo z
                // lokalnego licznika Hacker Mode, patrz
                // `playtime::merge_into` (wywoływane centralnie w
                // `commands::list_games`, po zbudowaniu pełnej listy z
                // WSZYSTKICH platform — nie tutaj, żeby nie czytać pliku
                // `playtime.json` osobno dla każdego providera).
                playtime_minutes: None,
            })
            .collect())
    }

    fn install_command(&self, _game_id: &str) -> Result<Command, StoreError> {
        // Bez dostępu do prywatnego API EA nie da się zainicjować
        // pobierania konkretnej gry z pominięciem samego EA Desktop
        // (patrz moduł-dokumentacja `ea-cli`) — jedyna uczciwa opcja to
        // otworzyć klienta, z poziomu którego użytkownik wybiera grę do
        // zainstalowania sam, tak jak zrobiłby to bez Hacker Mode.
        // Wcześniej ta metoda w ogóle nie była nadpisana (błąd
        // "nieobsługiwane"); teraz przynajmniej faktycznie coś robi,
        // kosztem tego, że event `hacker-mode://install-finished`
        // (patrz `launcher::install_game`) odpali się dopiero gdy
        // użytkownik ZAMKNIE cały EA app, nie gdy skończy się pobieranie
        // tej jednej gry — UI pokazujący pasek postępu instalacji
        // (`Library.tsx`/`GameDetail.tsx`) nie dostanie tu żadnych
        // procentów, tylko finalne "zakończono" po zamknięciu klienta.
        self.launch_command(_game_id)
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        // Analogicznie brak prywatnego API = brak "prawdziwego"
        // odinstalowania przez EA Desktop z pominięciem jego UI. Jedyna
        // uczciwa opcja, jaką Hacker Mode MOŻE zrobić samodzielnie: usunąć
        // katalog, w którym EA Desktop trzyma pliki tej gry (dokładnie ten
        // katalog, który `list_games` już zgłasza jako `install_dir`) —
        // `ea_cli::resolve_game_dir` odtwarza tę samą ścieżkę z walidacją
        // przeciw path traversal, więc nie polegamy tu na `install_dir`
        // przekazanym z frontendu. To NIE usuwa wpisów w rejestrze Wine
        // ani zapisów gry poza tym katalogiem (EA Desktop sam nie robi
        // nic lepszego bez GUI — to i tak najbliższe realnemu
        // "odinstalowaniu", jakie da się zrobić z linii poleceń).
        let prefix = ea_cli::find_prefix(self.explicit_prefix.as_deref())?;
        let dir = ea_cli::resolve_game_dir(&prefix, game_id)?;
        let mut cmd = Command::new("rm");
        cmd.args(["-rf", "--"]);
        cmd.arg(&dir);
        Ok(cmd)
    }

    fn launch_command(&self, _game_id: &str) -> Result<Command, StoreError> {
        // Celowo ignorujemy `_game_id` — uruchamiamy sam EA app, nie
        // konkretną grę z jego pominięciem (patrz moduł-dokumentacja
        // `ea-cli::build_launch_ea_app_command`). Frontend informuje o tym
        // użytkownika przy karcie gry (patrz `GameDetail.tsx`).
        let prefix = ea_cli::find_prefix(self.explicit_prefix.as_deref())?;
        Ok(ea_cli::build_launch_ea_app_command(&prefix)?)
    }
}

/// Pobiera okładki przez SteamGridDB dla listy gier EA tylko-po-tytule, o
/// ile klucz API jest skonfigurowany — w przeciwnym razie zwraca od razu
/// wektor samych `None`, bez budowania klienta HTTP ani wykonywania
/// jakiegokolwiek zapytania sieciowego. `battlenet.rs` ma swoją, prawie
/// identyczną kopię tej funkcji (nad `bnet::InstalledGame` zamiast
/// `ea_cli::InstalledGame` — dwa różne typy z siostrzanych crate'ów, stąd
/// brak współdzielenia jednej generycznej wersji za cenę dodatkowej
/// abstrakcji dla dwóch krótkich funkcji).
fn fetch_covers_if_configured(
    games: &[ea_cli::InstalledGame],
    api_key: Option<&str>,
    cache_prefix: &str,
) -> Vec<Option<String>> {
    let mut covers: Vec<Option<String>> = vec![None; games.len()];
    let Some(api_key) = api_key.filter(|k| !k.trim().is_empty()) else {
        return covers;
    };
    let Some(client) = crate::cover_cache::build_client() else {
        return covers;
    };
    for (game, slot) in games.iter().zip(covers.iter_mut()) {
        let cache_key = format!("{cache_prefix}-{}", game.id);
        *slot = crate::steamgriddb::fetch_cover_by_title(&game.title, &cache_key, api_key, &client);
    }
    covers
}
