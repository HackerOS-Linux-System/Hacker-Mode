pub mod amazon;
pub mod battlenet;
pub mod ea;
pub mod epic;
pub mod gog;
pub mod lutris;
pub mod steam;
// Prywatny (nie `pub`) — szczegół implementacyjny wyłącznie `steam.rs`
// (odczyt `userdata/<id>/config/shortcuts.vdf`, skróty non-Steam typu
// "Add a Non-Steam Game to My Library"). Patrz jego moduł-dokumentacja.
mod steam_shortcuts;

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Steam,
    Epic,
    Gog,
    Amazon,
    Lutris,
    Ea,
    BattleNet,
}

impl Platform {
    pub fn label(&self) -> &'static str {
        match self {
            Platform::Steam => "Steam",
            Platform::Epic => "Epic Games",
            Platform::Gog => "GOG",
            Platform::Amazon => "Amazon Games",
            Platform::Lutris => "Lutris",
            Platform::Ea => "EA app",
            Platform::BattleNet => "Battle.net",
        }
    }

    /// Krótki identyfikator tekstowy platformy, dokładnie taki, jaki
    /// serde produkuje dla tego wariantu dzięki `#[serde(rename_all =
    /// "lowercase")]` na `Platform` — używany do porównań z
    /// `Settings::enabled_platforms` (lista stringów zapisywana przez
    /// frontend, patrz `Settings.tsx`/`settingsStore.ts`), bez potrzeby
    /// przechodzenia przez pełną (de)serializację JSON dla tak prostego
    /// porównania.
    pub fn slug(&self) -> &'static str {
        match self {
            Platform::Steam => "steam",
            Platform::Epic => "epic",
            Platform::Gog => "gog",
            Platform::Amazon => "amazon",
            Platform::Lutris => "lutris",
            Platform::Ea => "ea",
            Platform::BattleNet => "battlenet",
        }
    }

    /// Czy Hacker Mode potrafi zarządzać logowaniem do tej platformy z
    /// poziomu własnego UI (`StoreLogin.tsx`) — `true` dla Steam/Epic/GOG/
    /// Amazon (patrz `login_start`/`is_logged_in` nadpisane przez ich
    /// providerów). Lutris/EA app/Battle.net zarządzają logowaniem same,
    /// wewnątrz swojego okna — nie ma tu nic do "zalogowania" z perspektywy
    /// Hacker Mode, więc `StoreLogin.tsx` nie powinien pokazywać dla nich
    /// przycisku "Zaloguj"/statusu logowania (dawniej pokazywał, mylnie —
    /// patrz historia zmian).
    pub fn supports_managed_login(&self) -> bool {
        matches!(self, Platform::Steam | Platform::Epic | Platform::Gog | Platform::Amazon)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    /// Unikalny w obrębie danej platformy identyfikator (np. Steam AppID).
    pub id: String,
    pub title: String,
    pub platform: Platform,
    pub installed: bool,
    pub install_dir: Option<String>,
    /// Ścieżka do okładki/artworku, jeśli dostępna lokalnie w cache sklepu.
    pub cover_path: Option<String>,
    /// Czas gry w minutach, jeśli platforma to udostępnia.
    pub playtime_minutes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDetails {
    pub description: String,
    pub screenshots: Vec<String>,
}

/// Wspólny kształt "gra posiadana na koncie" dla platform, których
/// providerzy potrafią wylistować PEŁNĄ bibliotekę (nie tylko to, co
/// zainstalowane lokalnie) — dziś Epic (`epic::fetch_owned_games`) i
/// Amazon (`amazon::fetch_owned_games`), analogicznie do
/// `steam::OwnedSteamGame`, którego Steam używa od dłuższego czasu.
/// Celowo bez `platform`/`installed`/`playtime_minutes` — te pola
/// dokłada dopiero `merge_owned_but_not_installed` w `list_all_games`,
/// bo są takie same dla każdego wpisu z danej platformy.
///
/// BUGFIX: ta struktura przez kilka poprzednich tur w ogóle nie miała
/// `#[derive(...)]` — działało to tylko dlatego, że `OwnedGame` nigdzie
/// nie przekraczało granicy wymagającej `Serialize`/`Clone`/`Debug` (jest
/// tworzone i od razu konsumowane wewnątrz `merge_owned_but_not_installed`).
/// `Serialize`/`Deserialize` potrzebne teraz do cache'owania
/// `gog::fetch_owned_games` w `net_cache` (patrz `gog.rs`) — bez tego
/// derive'a ten cache w ogóle by się nie skompilował.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedGame {
    pub id: String,
    pub title: String,
    pub cover_path: Option<String>,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum StoreError {
    #[error("narzędzie `{0}` nie jest zainstalowane w systemie")]
    ToolNotInstalled(String),
    #[error("błąd uruchomienia narzędzia `{0}`: {1}")]
    ExecFailed(String, String),
    #[error("nie udało się sparsować danych z `{0}`: {1}")]
    ParseError(String, String),
    #[error("nie zalogowano do platformy {0}")]
    NotLoggedIn(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginFlow {
    /// URL do otwarcia w oknie webview osadzonym w Hacker Mode (ta sama
    /// sesja — bez zewnętrznej przeglądarki), na którym użytkownik loguje
    /// się do swojego konta.
    pub url: Option<String>,
    /// Instrukcja pokazywana użytkownikowi w UI (np. co zrobić po
    /// zalogowaniu, skąd skopiować kod).
    pub instructions: String,
    /// Czy po zalogowaniu trzeba wywołać `login_submit` z kodem/tokenem
    /// wklejonym przez użytkownika (flow "authorization code"), czy
    /// logowanie kończy się samo (np. Steam — obsługiwane przez sam
    /// klient Steam, bez udziału Hacker Mode).
    pub needs_code: bool,
}

/// Wspólny interfejs sklepu/launchera gier.
pub trait StoreProvider {
    fn platform(&self) -> Platform;

    /// Czy narzędzie/klient wymagany do obsługi tej platformy jest dostępny
    /// w systemie (np. `legendary`, `gogdl`, `nile`, `lutris`, `flatpak`
    /// ze Steamem). Jeśli `false`, sklep jest po prostu pomijany przy
    /// agregacji zamiast wywalać cały wynik.
    fn is_available(&self) -> bool;

    /// Czy użytkownik jest zalogowany do tej platformy (na tyle, żeby
    /// `list_games` miało sens). Domyślnie `true` — nadpisywane przez
    /// dostawców, którzy tego wymagają (Epic/GOG/Amazon).
    fn is_logged_in(&self) -> bool {
        true
    }

    /// Rozpoczyna proces logowania — zwraca URL do pokazania w osadzonym
    /// webview oraz instrukcję dla użytkownika.
    fn login_start(&self) -> Result<LoginFlow, StoreError> {
        Err(StoreError::ToolNotInstalled(
            "logowanie nie jest wymagane/obsługiwane dla tej platformy".into(),
        ))
    }

    /// Kończy proces logowania kodem autoryzacyjnym wklejonym przez
    /// użytkownika po zalogowaniu się na stronie z `login_start`.
    fn login_submit(&self, _code: &str) -> Result<(), StoreError> {
        Err(StoreError::ToolNotInstalled(
            "logowanie kodem nie jest wymagane/obsługiwane dla tej platformy".into(),
        ))
    }

    /// Zwraca listę zainstalowanych (i ewentualnie dostępnych do
    /// zainstalowania) gier. Implementacje NIE powinny panikować przy
    /// braku danych — zwracają pustą listę / błąd, który agregator
    /// zamienia na ostrzeżenie w logu.
    fn list_games(&self) -> Result<Vec<Game>, StoreError>;

    /// Buduje komendę instalującą grę o danym identyfikatorze. Domyślnie
    /// nieobsługiwane (np. Lutris, gdzie instalacja idzie przez własny
    /// system skryptów instalacyjnych, a nie prosty `install <id>`).
    fn install_command(&self, _game_id: &str) -> Result<Command, StoreError> {
        Err(StoreError::ToolNotInstalled(
            "instalacja z poziomu Hacker Mode nie jest obsługiwana dla tej platformy".into(),
        ))
    }

    /// Buduje komendę odinstalowującą grę. Domyślnie nieobsługiwane.
    fn uninstall_command(&self, _game_id: &str) -> Result<Command, StoreError> {
        Err(StoreError::ToolNotInstalled(
            "odinstalowanie z poziomu Hacker Mode nie jest obsługiwane dla tej platformy".into(),
        ))
    }

    /// Zbuduj komendę uruchamiającą daną grę. Zwracany `Command` jest
    /// dalej przekazywany do `commands::launcher`, które odpowiada za
    /// tryb wrapper / IPC z kompozytorem / logowanie.
    fn launch_command(&self, game_id: &str) -> Result<Command, StoreError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformWarning {
    pub platform: Platform,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryLoadResult {
    pub games: Vec<Game>,
    /// BUGFIX: dawniej błąd pojedynczego providera (np. `legendary`
    /// zwracające błąd przez zepsutą/wygasłą sesję) trafiał WYŁĄCZNIE do
    /// `tracing::warn!` w pliku logu — użytkownik widział po prostu pustą
    /// bibliotekę danego sklepu bez żadnej wskazówki dlaczego. Komenda
    /// `list_games` zwracała `Vec<Game>` zamiast `Result`/struktury z
    /// błędami, więc frontendowe `gamesStore.error` nie miało jak się
    /// kiedykolwiek ustawić. Teraz agregacja zwraca też listę ostrzeżeń —
    /// patrz `Library.tsx`, które je wyświetla.
    pub warnings: Vec<PlatformWarning>,
}

/// Agreguje biblioteki ze wszystkich dostępnych platform. Błąd pojedynczego
/// dostawcy nie przerywa całości — jest logowany, pomijany przy budowaniu
/// listy gier, ALE zwracany też w `warnings`, tak by np. zepsuta sesja
/// Epic nie blokowała widoku biblioteki Steam, a jednocześnie użytkownik
/// widział w UI, że coś jest nie tak z Epikiem, zamiast domyślać się tego
/// z pustej zakładki.
///
/// Uwaga wydajnościowa: pobieranie okładek dla Epic/GOG jest sieciowe —
/// przy dużej bibliotece bez lokalnego cache (pierwsze uruchomienie) samo
/// odpytywanie API okładka-po-okładce potrafiłoby zauważalnie zwolnić
/// `list_games`. Dlatego `epic::EpicProvider::list_games` i
/// `gog::GogProvider::list_games` pobierają swoje okładki równolegle, na
/// ograniczonej puli wątków (`fetch_covers_in_parallel` w każdym z tych
/// modułów, `std::thread::scope`, limit 8 wątków naraz, jeden współdzielony
/// `reqwest::blocking::Client` — patrz `cover_cache::build_client`) zamiast
/// pobierać jedna-po-drugiej.
pub fn list_all_games(steam_credentials: Option<(&str, &str)>) -> LibraryLoadResult {
    let providers: Vec<Box<dyn StoreProvider>> = vec![
        Box::new(steam::SteamProvider::default()),
        Box::new(epic::EpicProvider::default()),
        Box::new(gog::GogProvider::default()),
        Box::new(amazon::AmazonProvider::default()),
        Box::new(lutris::LutrisProvider::default()),
        Box::new(ea::EaProvider::default()),
        Box::new(battlenet::BattleNetProvider::default()),
    ];

    let mut all_games = Vec::new();
    let mut warnings = Vec::new();
    for provider in providers {
        if !provider.is_available() {
            tracing::debug!(platform = ?provider.platform(), "Sklep niedostępny (brak narzędzia), pomijam");
            continue;
        }
        match provider.list_games() {
            Ok(mut games) => {
                tracing::info!(platform = ?provider.platform(), count = games.len(), "Wczytano bibliotekę");
                all_games.append(&mut games);
            }
            Err(err) => {
                tracing::warn!(platform = ?provider.platform(), error = %err, "Nie udało się wczytać biblioteki");
                warnings.push(PlatformWarning {
                    platform: provider.platform(),
                    message: err.to_string(),
                });
            }
        }
    }

    if let Some((api_key, steam_id64)) = steam_credentials {
        // BUGFIX: dawniej ta gałąź dociągała WYŁĄCZNIE czas gry dla już
        // znalezionych (czyli tylko zainstalowanych) gier Steam. Teraz
        // ten sam jeden request (`fetch_owned_games`) robi dwie rzeczy:
        // wzbogaca zainstalowane gry o realny czas gry, i dokłada do
        // biblioteki gry POSIADANE, ale niezainstalowane na tej maszynie
        // (`installed: false`, z okładką z publicznego CDN Steam zamiast
        // lokalnego cache — patrz `steam::owned_game_cdn_cover_url`).
        // Wcześniej takie gry w ogóle się nie pojawiały w Hacker Mode.
        let owned = steam::fetch_owned_games(api_key, steam_id64);
        if !owned.is_empty() {
            // Uwaga o pożyczaniu: `installed_appids` musi trzymać
            // WŁASNE `String`, nie `&str` pożyczone z `all_games` — ten
            // sam blok dalej robi `all_games.iter_mut()`/`all_games.push()`
            // (pożyczenie mutowalne), więc jakiekolwiek pożyczenie
            // niemutowalne z `all_games` żyjące przez całą pętlę
            // (`HashSet<&str>` by tak robił) nie przeszłoby przez
            // borrow checker.
            let installed_appids: std::collections::HashSet<String> = all_games
                .iter()
                .filter(|g| g.platform == Platform::Steam)
                .map(|g| g.id.clone())
                .collect();

            for owned_game in &owned {
                if installed_appids.contains(&owned_game.appid) {
                    // Już mamy tę grę z lokalnego skanu — tylko
                    // uzupełniamy realny czas gry, resztę (okładkę z
                    // lokalnego cache, install_dir) zostawiamy bez zmian.
                    if let Some(game) = all_games
                        .iter_mut()
                        .find(|g| g.platform == Platform::Steam && g.id == owned_game.appid)
                    {
                        game.playtime_minutes = Some(owned_game.playtime_minutes);
                    }
                } else {
                    all_games.push(Game {
                        id: owned_game.appid.clone(),
                        title: owned_game.name.clone(),
                        platform: Platform::Steam,
                        installed: false,
                        install_dir: None,
                        cover_path: Some(steam::owned_game_cdn_cover_url(&owned_game.appid)),
                        playtime_minutes: Some(owned_game.playtime_minutes),
                    });
                }
            }
        }
    }

    // Ostrzeżenie widoczne w UI (patrz `Library.tsx`, ten sam mechanizm co
    // `warnings` wyżej), gdy Lutris JEST dostępny, ale `sqlite3` nie —
    // wtedy `lutris::LutrisProvider::uninstall_command` i czas gry w
    // `list_games` (patrz `lutris::fetch_playtime_minutes`) po cichu nic
    // nie robią, bo obie funkcje wymagają tego narzędzia (patrz
    // moduł-dokumentacja `lutris.rs`). Bez tego ostrzeżenia użytkownik nie
    // miałby żadnego sposobu, żeby się dowiedzieć, DLACZEGO przycisk
    // „Odinstaluj” przy grze z Lutrisa nic nie zrobił — w logu było to
    // widoczne od razu (`tracing::debug!`/`Err(StoreError::ToolNotInstalled)`),
    // ale nikt poza deweloperem nie zagląda do logów Hacker Mode.
    if lutris::LutrisProvider::default().is_available() && !tool_available("sqlite3") {
        warnings.push(PlatformWarning {
            platform: Platform::Lutris,
            message: "Brakuje narzędzia „sqlite3” w systemie — deinstalacja gier Lutrisa i ich realny czas gry (z bazy Lutrisa) nie działają. Reszta Lutrisa (uruchamianie, instalacja) działa normalnie.".to_string(),
        });
    }

    // Epic/Amazon/GOG: ten sam pomysł co dla Steam wyżej (gry POSIADANE,
    // ale niezainstalowane), tylko bez osobnych danych logowania —
    // providerzy te już wiedzą, czy są zalogowani (`is_logged_in`, oparte
    // o pliki konfiguracyjne `legendary`/`nile`/`gogdl`, patrz ich
    // moduły), więc warunkiem jest tylko dostępność narzędzia +
    // zalogowanie, tak jak przy `list_games` wyżej w tej samej funkcji.
    merge_owned_but_not_installed(&mut all_games, Platform::Epic, epic::EpicProvider::default().is_available() && epic::EpicProvider::default().is_logged_in(), epic::fetch_owned_games);
    merge_owned_but_not_installed(&mut all_games, Platform::Amazon, amazon::AmazonProvider::default().is_available() && amazon::AmazonProvider::default().is_logged_in(), amazon::fetch_owned_games);
    merge_owned_but_not_installed(&mut all_games, Platform::Gog, gog::GogProvider::default().is_available() && gog::GogProvider::default().is_logged_in(), gog::fetch_owned_games);

    LibraryLoadResult { games: all_games, warnings }
}

/// Dokłada do `all_games` gry z `fetch_owned` (pełna biblioteka
/// POSIADANA na koncie danej platformy), które nie są jeszcze na liście —
/// czyli te, których lokalny skan (`list_games`) nie znalazł, bo nie są
/// zainstalowane na tej maszynie. Gry już obecne na liście (bo są
/// zainstalowane) zostają BEZ ZMIAN — w przeciwieństwie do bloku Steam
/// wyżej, tu nie ma dodatkowego "prawdziwego" czasu gry do dociągnięcia
/// przy okazji (to już robi `epic::fetch_owned_games`/`amazon::fetch_owned_games`
/// samo przez `cover_path`, a czas gry i tak zostaje uzupełniony później,
/// centralnie, przez `playtime::merge_into` w `commands::list_games`).
///
/// `should_fetch` jest policzone PRZED wywołaniem (zamiast wołać
/// `is_available`/`is_logged_in` tutaj) czysto z wygody wywołania —
/// funkcja i tak nic nie robi, gdy `should_fetch` jest `false`, więc nie
/// marnuje to żadnej pracy.
///
/// `fetch_owned` dostaje `installed_ids` PRZED wywołaniem (nie po), żeby
/// providerzy, dla których dociąganie szczegółów każdej gry jest
/// kosztowne sieciowo (dziś: GOG — patrz `gog::fetch_owned_games`), mogli
/// pominąć zapytania dla gier, które i tak już są na liście, zamiast
/// dociągać wszystko i dopiero potem odfiltrowywać duplikaty. Epic/Amazon
/// dziś ten parametr ignorują (ich CLI zwraca tytuły/okładki dla całej
/// biblioteki jednym, tanim, lokalnym wywołaniem — filtrowanie po fakcie
/// nic tam nie kosztuje), ale sygnatura jest wspólna dla wszystkich
/// trzech, żeby dało się je trzymać w jednej zmiennej `fn`.
fn merge_owned_but_not_installed(
    all_games: &mut Vec<Game>,
    platform: Platform,
    should_fetch: bool,
    fetch_owned: fn(&std::collections::HashSet<String>) -> Vec<OwnedGame>,
) {
    if !should_fetch {
        return;
    }
    let installed_ids: std::collections::HashSet<String> =
        all_games.iter().filter(|g| g.platform == platform).map(|g| g.id.clone()).collect();

    let owned = fetch_owned(&installed_ids);
    if owned.is_empty() {
        return;
    }

    for owned_game in owned {
        if installed_ids.contains(&owned_game.id) {
            // `fetch_owned` MOŻE (ale nie musi, patrz Epic/Amazon wyżej)
            // sam już odfiltrować zainstalowane gry przed zwróceniem —
            // ta sprawdzka tu to tylko dodatkowe zabezpieczenie przeciw
            // duplikatom na liście, gdyby jednak tego nie zrobił.
            continue;
        }
        all_games.push(Game {
            id: owned_game.id,
            title: owned_game.title,
            platform,
            installed: false,
            install_dir: None,
            cover_path: owned_game.cover_path,
            playtime_minutes: None,
        });
    }
}

/// Buduje komendę uruchomienia gry, przeszukując odpowiedniego dostawcę wg
/// platformy przekazanej z frontendu (frontend zna platformę, bo to on
/// wyrenderował kartę gry z danych zwróconych przez `list_all_games`).
pub fn build_launch_command(platform: Platform, game_id: &str) -> Result<Command, StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().launch_command(game_id),
        Platform::Epic => epic::EpicProvider::default().launch_command(game_id),
        Platform::Gog => gog::GogProvider::default().launch_command(game_id),
        Platform::Amazon => amazon::AmazonProvider::default().launch_command(game_id),
        Platform::Lutris => lutris::LutrisProvider::default().launch_command(game_id),
        Platform::Ea => ea::EaProvider::default().launch_command(game_id),
        Platform::BattleNet => battlenet::BattleNetProvider::default().launch_command(game_id),
    }
}

pub fn build_install_command(platform: Platform, game_id: &str) -> Result<Command, StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().install_command(game_id),
        Platform::Epic => epic::EpicProvider::default().install_command(game_id),
        Platform::Gog => gog::GogProvider::default().install_command(game_id),
        Platform::Amazon => amazon::AmazonProvider::default().install_command(game_id),
        Platform::Lutris => lutris::LutrisProvider::default().install_command(game_id),
        Platform::Ea => ea::EaProvider::default().install_command(game_id),
        Platform::BattleNet => battlenet::BattleNetProvider::default().install_command(game_id),
    }
}

pub fn build_uninstall_command(platform: Platform, game_id: &str) -> Result<Command, StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().uninstall_command(game_id),
        Platform::Epic => epic::EpicProvider::default().uninstall_command(game_id),
        Platform::Gog => gog::GogProvider::default().uninstall_command(game_id),
        Platform::Amazon => amazon::AmazonProvider::default().uninstall_command(game_id),
        Platform::Lutris => lutris::LutrisProvider::default().uninstall_command(game_id),
        Platform::Ea => ea::EaProvider::default().uninstall_command(game_id),
        Platform::BattleNet => battlenet::BattleNetProvider::default().uninstall_command(game_id),
    }
}

/// Hak wywoływany PO potwierdzonym sukcesie instalacji (patrz
/// `launcher::install_game`) — dziś potrzebny WYŁĄCZNIE dla GOG, które
/// (w przeciwieństwie do Epic/Amazon, gdzie `legendary`/`nile` same
/// utrzymują swój stan zainstalowanych gier) nie ma żadnej podkomendy do
/// wylistowania zainstalowanych gier, więc Hacker Mode musi sam
/// zapisać, że instalacja się powiodła — patrz moduł-dokumentacja
/// `gog::GogInstalledManifest`. Dla pozostałych platform to zwyczajnie
/// no-op.
pub fn on_install_finished(platform: Platform, game_id: &str) {
    if platform == Platform::Gog {
        gog::record_installed(game_id);
    }
}

/// Odpowiednik `on_install_finished`, wywoływane po potwierdzonym
/// sukcesie odinstalowania — patrz `gog::forget_installed`.
pub fn on_uninstall_finished(platform: Platform, game_id: &str) {
    if platform == Platform::Gog {
        gog::forget_installed(game_id);
    }
}

pub fn is_logged_in(platform: Platform) -> bool {
    match platform {
        Platform::Steam => steam::SteamProvider::default().is_logged_in(),
        Platform::Epic => epic::EpicProvider::default().is_logged_in(),
        Platform::Gog => gog::GogProvider::default().is_logged_in(),
        Platform::Amazon => amazon::AmazonProvider::default().is_logged_in(),
        Platform::Lutris => lutris::LutrisProvider::default().is_logged_in(),
        Platform::Ea => ea::EaProvider::default().is_logged_in(),
        Platform::BattleNet => battlenet::BattleNetProvider::default().is_logged_in(),
    }
}

pub fn login_start(platform: Platform) -> Result<LoginFlow, StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().login_start(),
        Platform::Epic => epic::EpicProvider::default().login_start(),
        Platform::Gog => gog::GogProvider::default().login_start(),
        Platform::Amazon => amazon::AmazonProvider::default().login_start(),
        Platform::Lutris => lutris::LutrisProvider::default().login_start(),
        Platform::Ea => ea::EaProvider::default().login_start(),
        Platform::BattleNet => battlenet::BattleNetProvider::default().login_start(),
    }
}

pub fn login_submit(platform: Platform, code: &str) -> Result<(), StoreError> {
    match platform {
        Platform::Steam => steam::SteamProvider::default().login_submit(code),
        Platform::Epic => epic::EpicProvider::default().login_submit(code),
        Platform::Gog => gog::GogProvider::default().login_submit(code),
        Platform::Amazon => amazon::AmazonProvider::default().login_submit(code),
        Platform::Lutris => lutris::LutrisProvider::default().login_submit(code),
        Platform::Ea => ea::EaProvider::default().login_submit(code),
        Platform::BattleNet => battlenet::BattleNetProvider::default().login_submit(code),
    }
}

/// Dociąga opis i zrzuty ekranu do widoku szczegółów gry
/// (`GameDetail.tsx`). Zaimplementowane tam, gdzie istnieje sensowne,
/// PUBLICZNE źródło danych katalogowych bez potrzeby zgadywania
/// prywatnego API:
/// - Steam: publiczne, nieuwierzytelnione Store API (`steam.rs`).
/// - GOG: publiczne, nieuwierzytelnione `api.gog.com` (`gog.rs`) —
///   dokładnie ten sam endpoint co dla okładek, tylko z innymi polami
///   `expand`.
/// - Epic: BEZ żadnego zapytania sieciowego — `legendary` i tak już
///   zapisuje pełne metadane katalogowe (opis, `keyImages`) lokalnie przy
///   każdym `legendary list`, patrz `epic::fetch_store_details`.
///
/// Amazon/Lutris/EA/Battle.net: brak jakiegokolwiek udokumentowanego,
/// publicznego API katalogowego (Amazon — `nile` go nie udostępnia;
/// Lutris — publiczne `lutris.net/api/games/<slug>` nie zwraca opisu ani
/// zrzutów ekranu, tylko podstawowe metadane typu rok/gatunek; EA/
/// Battle.net — prywatne API, patrz moduły `ea-cli`/`bnet`) — pozostaje
/// `None`, zamiast zgadywać nieistniejący endpoint.
pub fn fetch_game_details(platform: Platform, game_id: &str) -> Option<GameDetails> {
    match platform {
        Platform::Steam => steam::fetch_store_details(game_id),
        Platform::Gog => gog::fetch_store_details(game_id),
        Platform::Epic => epic::fetch_store_details(game_id),
        _ => None,
    }
}

/// Pomocnicza funkcja: sprawdza obecność narzędzia w `PATH`.
pub(crate) fn tool_available(name: &str) -> bool {
    which::which(name).is_ok()
}
