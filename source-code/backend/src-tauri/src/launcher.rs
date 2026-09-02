use std::process::Command;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::stores::{self, Platform};
use crate::state::AppState;

const LAUNCH_COOLDOWN: Duration = Duration::from_secs(10);

/// Jak długo czekamy, aż okno nowo uruchomionego procesu (gry/launchera)
/// pojawi się w `ListWindows` comphwde, żeby je zmaksymalizować - patrz
/// [`hacker_mode_ipc::maximize_next_new_window`].
const NEW_WINDOW_TIMEOUT: Duration = Duration::from_secs(15);
const NEW_WINDOW_POLL_INTERVAL: Duration = Duration::from_millis(300);
const IPC_CALL_TIMEOUT: Duration = Duration::from_secs(2);

/// Uruchamia grę z danej platformy po jej identyfikatorze (patrz
/// `stores::Game::id`). Owija komendę zgodnie z ustawieniami
/// `Settings::gaming_tools` (MangoHud/vkBasalt jako zmienne env,
/// Gamescope jako zewnętrzny wrapper procesu) — patrz `apply_gaming_tools`.
pub fn launch_game(app: AppHandle, state: State<AppState>, platform: Platform, game_id: String) -> Result<(), String> {
    let mut cmd = stores::build_launch_command(platform, &game_id).map_err(|e| e.to_string())?;
    let settings = state.settings.lock().unwrap();
    let gaming_tools = settings.gaming_tools.clone();
    // Mapowanie kontrolera per gra (patrz moduł-dokumentacja
    // `controllers.rs`) — wstrzyknięte TYLKO gdy użytkownik jawnie
    // skonfigurował je dla TEJ gry (`commands::set_game_controller_config`);
    // bez wpisu zmienna po prostu nie jest ustawiana, więc gra dostaje
    // dokładnie to środowisko, co przed dodaniem tej funkcji.
    let controller_key = crate::commands::cover_override_key(platform, &game_id);
    if let Some(config) = settings.game_controller_configs.get(&controller_key) {
        cmd.env("SDL_GAMECONTROLLERCONFIG", config);
    }
    let custom_launch_prefix = settings.custom_launch_prefix.clone();
    drop(settings);
    let cmd = apply_gaming_tools(cmd, &gaming_tools);
    // Globalny prefiks uruchamiania (`Settings::custom_launch_prefix`,
    // np. `"gamemoderun"`) — nakładany jako NAJBARDZIEJ zewnętrzna
    // warstwa, na zewnątrz Gamescope (patrz `wrap_with_custom_prefix`).
    let cmd = wrap_with_custom_prefix(cmd, &custom_launch_prefix);
    // `Some(...)` tylko tutaj (nie w `launch_store_client` niżej) — patrz
    // `run_wrapped`/moduł-dokumentacja `playtime.rs`: liczymy czas gry
    // TYLKO dla uruchomień konkretnej gry, nie dla samego otwarcia klienta
    // sklepu (to nie jest "granie", tylko przeglądanie biblioteki).
    let playtime_key = Some((platform.slug().to_string(), game_id.clone()));
    run_wrapped(app, state, cmd, format!("{}:{}", platform.label(), game_id), playtime_key)
}

/// Uruchamia bezpośrednio klienta GUI danego sklepu/launchera (np. otwarcie
/// pełnego Steama zamiast konkretnej gry) — odpowiednik przycisków ze
/// starego widoku "Hacker Menu".
pub fn launch_store_client(app: AppHandle, state: State<AppState>, name: &str) -> Result<(), String> {
    let cmd = match name {
        "steam" => steam_client_command(),
        "heroic" => flatpak_command("com.heroicgameslauncher.hgl"),
        "hyperplay" => flatpak_command("xyz.hyperplay.HyperPlay"),
        "lutris" => native_command("lutris"),
        // EA app / Battle.net nie mają "klienta" wywoływanego jedną
        // komendą jak Steam/Lutris — trzeba najpierw znaleźć ich prefiks
        // Wine (patrz `ea_cli`/`bnet`, moduły-dokumentacja). Współdzielimy
        // tu tę samą logikę co `stores::ea::EaProvider::launch_command`/
        // `battlenet::BattleNetProvider::launch_command`, tylko bez
        // konkretnego `game_id` (bo tu uruchamiamy sam launcher, nie grę).
        "ea" => {
            let prefix = ea_cli::find_prefix(
                crate::settings::Settings::load_or_default().ea_wine_prefix.as_deref().map(std::path::Path::new),
            )
            .map_err(|e| e.to_string())?;
            ea_cli::build_launch_ea_app_command(&prefix).map_err(|e| e.to_string())?
        }
        "battlenet" => {
            let prefix = bnet::find_prefix(
                crate::settings::Settings::load_or_default().battlenet_wine_prefix.as_deref().map(std::path::Path::new),
            )
            .map_err(|e| e.to_string())?;
            bnet::build_launch_battlenet_command(&prefix).map_err(|e| e.to_string())?
        }
        other => return Err(format!("Nieznany klient sklepu: {other}")),
    };
    // `None` — patrz komentarz w `launch_game`: to uruchomienie samego
    // klienta/launchera, nie konkretnej gry, więc nie ma czego doliczyć do
    // lokalnego licznika czasu gry.
    run_wrapped(app, state, cmd, name.to_string(), None)
}

/// Instaluje grę (bez trybu wrapper — instalacja nie chowa powłoki, tylko
/// pokazuje postęp/logi w UI poprzez event `hacker-mode://install-progress`
/// i `hacker-mode://install-finished`).
pub fn install_game(app: AppHandle, platform: Platform, game_id: String) -> Result<(), String> {
    let mut cmd = stores::build_install_command(platform, &game_id).map_err(|e| e.to_string())?;
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Nie udało się uruchomić instalacji: {e}"))?;

    let label = format!("{}:{}", platform.label(), game_id);

    // Przekierowanie stdout instalatora do frontendu na żywo (paski
    // postępu `legendary`/`gogdl`/`nile` piszą procent ukończenia na
    // stdout) — proste, liniowe strumieniowanie, bez parsowania procentów
    // (to zależy od narzędzia; frontend może same linie wyświetlić jako
    // log, ewentualnie sparsować wzorce typu "NN.N%" po swojej stronie).
    if let Some(stdout) = child.stdout.take() {
        let app_stdout = app.clone();
        let label_stdout = label.clone();
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            // Dopasowuje wzorce w stylu "42%", "42.7%", "[42/100]" — różne
            // narzędzia (`legendary`, `gogdl`, `nile`) formatują pasek
            // postępu inaczej, więc próbujemy kilku wariantów zamiast
            // zakładać jeden konkretny format.
            let percent_re = regex::Regex::new(r"(\d{1,3}(?:\.\d+)?)\s*%").unwrap();
            for line in BufReader::new(stdout).lines().flatten() {
                let percent = percent_re
                    .captures(&line)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse::<f32>().ok())
                    .filter(|p| (0.0..=100.0).contains(p));
                let _ = app_stdout.emit(
                    "hacker-mode://install-progress",
                    serde_json::json!({ "label": label_stdout, "line": line, "percent": percent }),
                );
            }
        });
    }

    let app_for_wait = app.clone();
    let label_for_notify = label.clone();
    std::thread::spawn(move || {
        let status = child.wait();
        let ok = status.map(|s| s.success()).unwrap_or(false);
        tracing::info!(label = %label, ok, "Instalacja zakończona");

        // Patrz `stores::on_install_finished` — dziś potrzebne wyłącznie
        // dla GOG (patrz `gog::GogInstalledManifest`), no-op dla reszty
        // platform. Wywoływane TYLKO po sukcesie — nieudana instalacja
        // nie powinna zostać zapisana jako zainstalowana gra.
        if ok {
            stores::on_install_finished(platform, &game_id);
        }

        let _ = app_for_wait.emit(
            "hacker-mode://install-finished",
            serde_json::json!({ "label": label, "ok": ok }),
        );
        // Powiadomienie systemowe — patrz moduł-dokumentacja
        // `notifications.rs`: instalacja to typowa operacja W TLE,
        // użytkownik mógł się w międzyczasie przełączyć na co innego.
        // `Settings::notifications.on_install` pozwala to wyłączyć —
        // patrz `commands::set_notification_settings`.
        if notifications_enabled_for(&app_for_wait, |n| n.on_install) {
            if ok {
                crate::notifications::info("Instalacja zakończona", &label_for_notify);
            } else {
                crate::notifications::error("Instalacja nie powiodła się", &label_for_notify);
            }
        }
    });

    Ok(())
}

/// Sprawdza `Settings::notifications` przez `AppHandle::try_state`
/// (patrz `exit_wrapper_mode` po ten sam wzorzec) — używane wszędzie
/// tam, gdzie tylko `AppHandle` jest dostępny (np. wewnątrz wątków
/// czekających na zakończenie procesu), nie `State<AppState>`. Domyślnie
/// (gdyby z jakiegoś powodu stan nie był dostępny) zwraca `true` —
/// brak pewności co do ustawienia nie powinien po cichu wyciszać
/// powiadomień, tylko ostrożnie je pokazywać.
fn notifications_enabled_for(app: &AppHandle, get: impl Fn(&crate::settings::NotificationSettings) -> bool) -> bool {
    app.try_state::<AppState>()
        .map(|state| get(&state.settings.lock().unwrap().notifications))
        .unwrap_or(true)
}

pub fn uninstall_game(app: AppHandle, platform: Platform, game_id: String) -> Result<(), String> {
    let mut cmd = stores::build_uninstall_command(platform, &game_id).map_err(|e| e.to_string())?;
    let label = format!("{}:{}", platform.label(), game_id);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Nie udało się uruchomić odinstalowania: {e}"))?;
    std::thread::spawn(move || {
        let status = child.wait();
        let ok = status.map(|s| s.success()).unwrap_or(false);
        tracing::info!(label = %label, ok, "Odinstalowanie zakończone");

        // Patrz `install_game`/`stores::on_uninstall_finished` po ten
        // sam wzorzec — dziś potrzebne wyłącznie dla GOG.
        if ok {
            stores::on_uninstall_finished(platform, &game_id);
        }

        let _ = app.emit(
            "hacker-mode://uninstall-finished",
            serde_json::json!({ "label": label, "ok": ok }),
        );
        if notifications_enabled_for(&app, |n| n.on_install) {
            if ok {
                crate::notifications::info("Odinstalowano", &label);
            } else {
                crate::notifications::error("Odinstalowanie nie powiodło się", &label);
            }
        }
    });
    Ok(())
}

/// Uruchamia proces logowania do sklepu — zwraca `LoginFlow` z URL-em do
/// pokazania w osadzonym webview.
pub fn login_start(platform: Platform) -> Result<stores::LoginFlow, String> {
    stores::login_start(platform).map_err(|e| e.to_string())
}

pub fn login_submit(platform: Platform, code: String) -> Result<(), String> {
    stores::login_submit(platform, &code).map_err(|e| e.to_string())
}

pub fn is_logged_in(platform: Platform) -> bool {
    stores::is_logged_in(platform)
}

fn steam_client_command() -> Command {
    if which::which("steam").is_ok() {
        Command::new("steam")
    } else {
        flatpak_command("com.valvesoftware.Steam")
    }
}

fn flatpak_command(app_id: &str) -> Command {
    let mut cmd = Command::new("flatpak");
    cmd.args(["run", app_id]);
    cmd
}

fn native_command(bin: &str) -> Command {
    Command::new(bin)
}

/// Owija komendę uruchamiającą grę zgodnie z ustawieniami
/// `Settings::gaming_tools` (checkboxy w `Settings.tsx`). Wcześniej te
/// ustawienia były zapisywane do `config.json`, ale nigdzie nie
/// wpływały na faktyczne uruchomienie gry — ta funkcja to naprawia:
///
/// - **MangoHud**: ustawia zmienną env `MANGOHUD=1`, którą overlay
///   MangoHud odczytuje, żeby się aktywować (standardowy sposób jego
///   włączania bez modyfikowania samej komendy gry).
/// - **vkBasalt**: analogicznie, `ENABLE_VKBASALT=1`.
/// - **Gamescope**: to nie zmienna env, tylko osobny proces, który musi
///   opakować (`exec`) oryginalną komendę — stąd `wrap_with_gamescope`,
///   które przenosi program/argumenty/env oryginalnej komendy pod
///   `gamescope -- <oryginalny program> <argumenty>`.
fn apply_gaming_tools(cmd: Command, tools: &crate::settings::GamingTools) -> Command {
    let mut cmd = cmd;
    if tools.mangohud {
        cmd.env("MANGOHUD", "1");
    }
    if tools.vkbasalt {
        cmd.env("ENABLE_VKBASALT", "1");
    }
    if tools.gamescope {
        cmd = wrap_with_gamescope(cmd);
    }
    cmd
}

/// Owija `original` globalnym prefiksem uruchamiania
/// (`Settings::custom_launch_prefix`) — ten sam mechanizm przenoszenia
/// programu/argumentów/env co `wrap_with_gamescope` (patrz jej
/// dokumentacja), tylko z konfigurowalnym poleceniem zamiast na sztywno
/// `"gamescope"`. Zwraca `original` bez zmian, jeśli prefiks jest pusty/
/// nieustawiony — więc bezpiecznie wywoływać zawsze, bez osobnego `if`
/// w miejscu wywołania.
fn wrap_with_custom_prefix(original: Command, prefix: &Option<String>) -> Command {
    let Some(prefix) = prefix else { return original };
    let mut words = prefix.split_whitespace();
    let Some(first) = words.next() else { return original };
    let rest: Vec<&str> = words.collect();

    let program = original.get_program().to_owned();
    let args: Vec<std::ffi::OsString> = original.get_args().map(|a| a.to_owned()).collect();
    let envs: Vec<(std::ffi::OsString, Option<std::ffi::OsString>)> = original
        .get_envs()
        .map(|(k, v)| (k.to_owned(), v.map(|v| v.to_owned())))
        .collect();

    let mut wrapped = Command::new(first);
    wrapped.args(&rest);
    wrapped.arg(&program);
    wrapped.args(&args);
    for (key, value) in envs {
        match value {
            Some(value) => {
                wrapped.env(key, value);
            }
            None => {
                wrapped.env_remove(key);
            }
        }
    }
    wrapped
}

/// Przenosi program/argumenty/zmienne env z `original` pod `gamescope --
/// <original>`, tak by cała gra (włącznie z ewentualnymi env-ami
/// ustawionymi przez `apply_gaming_tools` wcześniej, np. `MANGOHUD=1`)
/// wystartowała wewnątrz sesji Gamescope zamiast bezpośrednio.
///
/// Korzysta z `Command::get_program`/`get_args`/`get_envs` (stabilne w
/// std od dawna) zamiast przebudowywać komendę od zera z osobnych pól —
/// dzięki temu nie trzeba duplikować logiki budowania argumentów z
/// `commands/stores/*.rs` w tym module.
fn wrap_with_gamescope(original: Command) -> Command {
    let program = original.get_program().to_owned();
    let args: Vec<std::ffi::OsString> = original.get_args().map(|a| a.to_owned()).collect();
    let envs: Vec<(std::ffi::OsString, Option<std::ffi::OsString>)> = original
        .get_envs()
        .map(|(k, v)| (k.to_owned(), v.map(|v| v.to_owned())))
        .collect();

    let mut wrapped = Command::new("gamescope");
    wrapped.arg("--");
    wrapped.arg(&program);
    wrapped.args(&args);
    for (key, value) in envs {
        match value {
            Some(value) => {
                wrapped.env(key, value);
            }
            None => {
                wrapped.env_remove(key);
            }
        }
    }
    wrapped
}

fn run_wrapped(
    app: AppHandle,
    state: State<AppState>,
    mut cmd: Command,
    label: String,
    playtime_key: Option<(String, String)>,
) -> Result<(), String> {
    // Debounce: unikamy przypadkowego wielokrotnego odpalenia tej samej
    // aplikacji przy podwójnym kliknięciu / powtórzonym evencie z pada.
    static LAST_LAUNCH: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, Instant>>> =
        std::sync::OnceLock::new();
    let map = LAST_LAUNCH.get_or_init(|| std::sync::Mutex::new(Default::default()));
    {
        let mut guard = map.lock().unwrap();
        if let Some(last) = guard.get(&label) {
            if last.elapsed() < LAUNCH_COOLDOWN {
                return Err(format!(
                    "Poczekaj chwilę przed ponownym uruchomieniem „{label}”."
                ));
            }
        }
        guard.insert(label.clone(), Instant::now());
    }

    let wrapper_enabled = state.settings.lock().unwrap().wrapper_mode_enabled;
    let is_dev = state.is_dev();

    // Poświadczenia Steam Web API — pobrane TERAZ (razem z resztą ustawień
    // niżej), z tego samego powodu co `gaming_tools`/`auto_backup_config`:
    // `State<AppState>` nie przenosi się do `std::thread::spawn`. `Some(...)`
    // TYLKO dla uruchomień konkretnej gry Steam (`platform_slug == "steam"`)
    // Z ustawionym kluczem API i SteamID64 — patrz
    // `watch_steam_web_api_session` po pełne wyjaśnienie, dlaczego Steam
    // wymaga zupełnie innego mechanizmu śledzenia niż pozostałe platformy.
    let steam_web_api_creds: Option<(String, String, String)> = playtime_key
        .as_ref()
        .filter(|(platform_slug, _)| platform_slug == "steam")
        .and_then(|(_, game_id)| {
            let settings = state.settings.lock().unwrap();
            let api_key = settings.steam_api_key.clone().filter(|k| !k.trim().is_empty())?;
            let steam_id64 = settings.steam_id64.clone().filter(|k| !k.trim().is_empty())?;
            Some((game_id.clone(), api_key, steam_id64))
        });

    // Migawka okien SPRZED odpalenia procesu — comphwde nie ma
    // dedykowanego "WrapperModeEnter/Exit" jak dawny własny kompozytor;
    // zamiast tego chowamy własne okno powłoki i po prostu wypatrujemy,
    // które okno jest NOWE względem tej migawki, żeby je zmaksymalizować.
    // Patrz `hacker_mode_ipc::maximize_next_new_window`.
    let known_windows = if wrapper_enabled && !is_dev {
        hacker_mode_ipc::known_window_ids(IPC_CALL_TIMEOUT).unwrap_or_default()
    } else {
        Vec::new()
    };

    if wrapper_enabled && !is_dev {
        enter_wrapper_mode(&app, &state);
    }

    cmd.env("XDG_SESSION_TYPE", "wayland");
    // Kopia zapasowa zapisu po zamknięciu gry (patrz moduł-dokumentacja
    // `cloud_saves.rs`) — dane potrzebne do niej (katalog kopii
    // zapasowych + ścieżka zapisu TEJ gry, jeśli skonfigurowana)
    // wyciągamy z ustawień TERAZ, przed odpaleniem wątku czekającego na
    // zakończenie procesu, bo `State<AppState>` nie przenosi się do
    // `std::thread::spawn` (jego czas życia jest związany z wywołaniem
    // komendy Tauri) — ten sam wzorzec co `gaming_tools`/
    // `game_controller_configs` wyżej w tej funkcji.
    let auto_backup_config = playtime_key.as_ref().and_then(|(platform_slug, game_id)| {
        let settings = state.settings.lock().unwrap();
        let backup_root = settings.cloud_saves_backup_dir.clone()?;
        let save_path = settings.game_save_paths.get(&format!("{platform_slug}:{game_id}"))?.clone();
        Some((backup_root, save_path))
    });
    // Znacznik czasu startu — jedyny sposób, w jaki Hacker Mode może
    // zmierzyć czas gry dla platform bez własnego API (patrz
    // moduł-dokumentacja `playtime.rs`): różnica między tym momentem a
    // momentem, w którym proces faktycznie się zakończy (`child.wait()`
    // niżej), niezależnie od tego, czy `playtime_key` jest w ogóle
    // ustawiony — licznik czasu i tak jest tani (jeden `Instant::now()`),
    // więc prościej zawsze go mieć niż rozgałęziać kod na dwie wersje.
    let started_at = Instant::now();
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Nie udało się uruchomić „{label}”: {e}"))?;

    // Rejestrujemy PID w `AppState::running_games` i powiadamiamy frontend
    // (`hacker-mode://game-launched`) — TYLKO dla realnego uruchomienia
    // gry (`playtime_key.is_some()`), nie dla samego otwarcia klienta
    // sklepu (`launch_store_client`, gdzie `playtime_key` jest `None`) —
    // patrz moduł-dokumentacja `AppState::running_games` po ważne
    // zastrzeżenie dotyczące Steam (PID to proces polecenia uruchomienia,
    // nie samej gry).
    if let Some((platform_slug, game_id)) = &playtime_key {
        let key = format!("{platform_slug}:{game_id}");
        state.running_games.lock().unwrap().insert(key, child.id());
        let _ = app.emit(
            "hacker-mode://game-launched",
            serde_json::json!({ "platform": platform_slug, "gameId": game_id, "label": label }),
        );
    }

    // Watcher Steam Web API — patrz `steam_web_api_creds` wyżej i
    // moduł-dokumentacja `watch_steam_web_api_session`. Uruchomiony
    // NIEZALEŻNIE od wątku niżej czekającego na `child.wait()` (który dla
    // Steam obserwuje tylko krótkotrwały proces `steam -applaunch`, nie
    // samą grę) — to ten watcher, nie ten wątek, doliczy czas gry i
    // wyemituje zdarzenia `game-exited`/powiadomienie o możliwym crashu
    // dla uruchomień Steam z skonfigurowanymi poświadczeniami API.
    if let Some((appid, api_key, steam_id64)) = steam_web_api_creds.clone() {
        let app_for_watcher = app.clone();
        let label_for_watcher = label.clone();
        std::thread::spawn(move || {
            watch_steam_web_api_session(app_for_watcher, appid, label_for_watcher, api_key, steam_id64);
        });
    }

    let app_for_thread = app.clone();
    let wrapper_enabled_for_thread = wrapper_enabled && !is_dev;

    if wrapper_enabled_for_thread {
        // Osobny wątek: czeka aż nowe okno gry/launchera pojawi się w
        // comphwde i maksymalizuje je. Nie blokuje ani UI, ani wątku
        // czekającego na zakończenie procesu poniżej.
        std::thread::spawn(move || {
            match hacker_mode_ipc::maximize_next_new_window(&known_windows, NEW_WINDOW_TIMEOUT, NEW_WINDOW_POLL_INTERVAL) {
                Ok(id) => tracing::info!(window_id = id, "Nowe okno zmaksymalizowane (tryb wrapper)"),
                Err(e) => tracing::debug!(error = %e, "Nie doczekano się nowego okna do zmaksymalizowania"),
            }
        });
    }

    std::thread::spawn(move || {
        let status = child.wait();
        let seconds_ran = started_at.elapsed().as_secs();
        tracing::info!(label = %label, ?status, "Proces zakończony");
        let ok = status.map(|s| s.success()).unwrap_or(false);

        if let Some((platform_slug, game_id)) = &playtime_key {
            // Usuwamy z `running_games` NIEZALEŻNIE od tego, czy watcher
            // Steam Web API przejął śledzenie — ten wpis to i tak tylko
            // PID krótkotrwałego procesu uruchamiającego (patrz
            // `AppState::running_games`), więc jego usunięcie jest
            // zawsze poprawne, niezależnie od gałęzi niżej.
            if let Some(app_state) = app_for_thread.try_state::<AppState>() {
                let key = format!("{platform_slug}:{game_id}");
                app_state.running_games.lock().unwrap().remove(&key);
            }

            if steam_web_api_creds.is_none() {
                // Ścieżka dla WSZYSTKICH platform poza Steam-z-kluczem-API:
                // tu `child.wait()` obserwuje realny proces gry (albo,
                // dla samego Steam bez skonfigurowanych poświadczeń API,
                // to najlepsze dostępne przybliżenie — patrz README, „Do
                // zrobienia dalej”), więc czas trwania TEGO procesu to
                // wiarygodna miara długości sesji.
                //
                // Zaokrąglenie w dół następuje RAZ, tutaj, na sumie sekund
                // całej sesji — patrz komentarz w `playtime::add_minutes` o
                // tym, dlaczego to ważne (żeby wielokrotne krótkie sesje nie
                // gubiły czasu przez zaokrąglanie przy każdym pojedynczym
                // zapisie).
                let minutes = seconds_ran / 60;
                crate::playtime::add_minutes(platform_slug, game_id, minutes);
                crate::playtime::record_session(platform_slug, game_id, minutes);

                let _ = app_for_thread.emit(
                    "hacker-mode://game-exited",
                    serde_json::json!({
                        "platform": platform_slug,
                        "gameId": game_id,
                        "label": &label,
                        "ok": ok,
                        "secondsRan": seconds_ran,
                    }),
                );

                // Powiadomienie systemowe TYLKO gdy coś wyglądało na crash
                // (błąd LUB bardzo krótka sesja) — normalne zamknięcie gry po
                // rozsądnym czasie NIE generuje powiadomienia (użytkownik i
                // tak w tym momencie prawdopodobnie patrzy na ekran, skoro
                // właśnie zamknął grę — powiadomienie o czymś oczywistym
                // byłoby tylko szumem, patrz `notifications.rs` moduł-dokumentacja
                // o zasadzie "tylko operacje w tle, których zakończenia można
                // nie zauważyć"). Próg konfigurowalny przez
                // `Settings::crash_detection_threshold_seconds` (patrz jej
                // dokumentacja — dawniej stała `8` wpisana na sztywno
                // niezależnie w TRZECH miejscach, dziś jedno, dzielone
                // źródło prawdy, które czyta też `GameCard.tsx`/
                // `GameDetail.tsx` z tego samego obiektu `Settings`).
                let crash_threshold = app_for_thread
                    .try_state::<AppState>()
                    .map(|s| s.settings.lock().unwrap().crash_detection_threshold_seconds)
                    .unwrap_or(8);
                if (!ok || seconds_ran < crash_threshold) && notifications_enabled_for(&app_for_thread, |n| n.on_game_exit) {
                    crate::notifications::error(
                        "Gra mogła się nie uruchomić poprawnie",
                        &format!("{label} zamknęła się po {seconds_ran}s{}", if !ok { " z błędem" } else { "" }),
                    );
                }
            } else {
                // Steam z skonfigurowanym Steam Web API: `child.wait()`
                // tutaj obserwuje WYŁĄCZNIE krótkotrwały proces
                // `steam -applaunch`, który zakończy się w ułamku
                // sekundy, gdy Steam już działał — to OCZEKIWANE, nie
                // dowód awarii. Właściwe śledzenie (doliczenie czasu gry,
                // zdarzenie `game-exited`, ewentualne powiadomienie o
                // crashu) przejmuje `watch_steam_web_api_session`
                // (uruchomiony wcześniej w tej funkcji), które mierzy
                // REALNY czas trwania sesji przez Steam Web API zamiast
                // czasu życia tego procesu-pośrednika.
                tracing::debug!(label = %label, "Steam -applaunch zakończone — śledzenie sesji przejęte przez watcher Steam Web API");
            }
        }

        // Automatyczna kopia zapasowa zapisu — TYLKO gdy użytkownik
        // skonfigurował zarówno katalog kopii zapasowych (Ustawienia →
        // Kopie zapasowe), jak i ścieżkę zapisu TEJ gry
        // (`GameDetail.tsx`, patrz `auto_backup_config` wyżej). Cicha przy
        // sukcesie w logu (tylko `debug!`), ale zawsze z powiadomieniem
        // systemowym o BŁĘDZIE — to jedyny moment, w którym użytkownik
        // mógłby nie zauważyć, że automatyczna kopia akurat zawiodła
        // (np. bo dysk się zapełnił), skoro dzieje się to całkowicie w
        // tle po zamknięciu gry.
        if let (Some((platform_slug, game_id)), Some((backup_root, save_path))) = (&playtime_key, &auto_backup_config) {
            match crate::cloud_saves::backup_now(std::path::Path::new(backup_root), platform_slug, game_id, save_path) {
                Ok(entry) => {
                    tracing::debug!(file = %entry.file_name, "Automatyczna kopia zapasowa zapisu utworzona");
                }
                Err(err) => {
                    tracing::warn!(%err, "Automatyczna kopia zapasowa zapisu nie powiodła się");
                    if notifications_enabled_for(&app_for_thread, |n| n.on_backup_error) {
                        crate::notifications::error("Kopia zapasowa zapisu nie powiodła się", &err);
                    }
                }
            }
        }

        if wrapper_enabled_for_thread {
            exit_wrapper_mode(&app_for_thread);
        }

        let _ = app_for_thread.emit(
            "hacker-mode://app-closed",
            serde_json::json!({ "label": label }),
        );
    });

    Ok(())
}

/// Śledzi REALNY stan sesji gry Steam przez oficjalne Steam Web API
/// (`ISteamUser/GetPlayerSummaries`, patrz `commands::stores::steam::fetch_player_current_appid`)
/// zamiast przez PID procesu `steam -applaunch` — patrz README, „Do
/// zrobienia dalej”: ten proces-pośrednik przekazuje uruchomienie do już
/// działającego klienta Steam przez jego wewnętrzne IPC i kończy się w
/// ułamku sekundy, więc obserwowanie JEGO czasu życia (jak dla
/// pozostałych platform, patrz `AppState::running_games`) nie mówi nic
/// wiarygodnego o tym, czy gra faktycznie działa.
///
/// Dwie fazy:
/// 1. **Oczekiwanie na start** (do `LAUNCH_GRACE_PERIOD`): odpytujemy
///    Steam Web API co `POLL_INTERVAL_WAITING`, aż zobaczymy `appid` w
///    `gameid` gracza. Jeśli to się nie stanie w tym oknie — uznajemy,
///    że uruchomienie się nie powiodło (np. gra od razu się wywaliła,
///    zanim Steam w ogóle zdążyłby zgłosić ją jako aktywną), wysyłamy
///    powiadomienie o możliwym crashu i kończymy, BEZ doliczania czasu
///    gry (sesja nigdy nie została potwierdzona jako rozpoczęta).
/// 2. **Obserwacja działającej sesji**: po potwierdzeniu startu wstawiamy
///    klucz do `AppState::steam_watched_games` (żeby `is_game_running`
///    zaczęło zwracać `true`) i odpytujemy rzadziej
///    (`POLL_INTERVAL_RUNNING`, żeby niepotrzebnie nie zużywać limitu
///    zapytań Steam Web API), aż `gameid` przestanie być równe `appid` —
///    wtedy sesja się skończyła, patrz `finish_steam_web_api_session`.
///
/// Pojedynczy NIEUDANY poll (błąd sieci/API, patrz `Option<Option<_>>`
/// w `fetch_player_current_appid`) jest CELOWO ignorowany, nie
/// traktowany jako "gra się zamknęła" — dopiero UDANE zapytanie, które
/// wyraźnie pokazuje INNY (albo żaden) `gameid`, kończy obserwację.
/// `MAX_SESSION_DURATION` to czysty bezpiecznik na wypadek, gdyby z
/// jakiegoś powodu Steam Web API w kółko zwracało ten sam `appid` mimo
/// realnego zamknięcia gry (np. przez wiele godzin bezczynności klienta
/// Steam) — zapobiega wątkowi działającemu w nieskończoność.
fn watch_steam_web_api_session(app: AppHandle, appid: String, label: String, api_key: String, steam_id64: String) {
    const LAUNCH_GRACE_PERIOD: Duration = Duration::from_secs(90);
    const POLL_INTERVAL_WAITING: Duration = Duration::from_secs(5);
    const POLL_INTERVAL_RUNNING: Duration = Duration::from_secs(20);
    const MAX_SESSION_DURATION: Duration = Duration::from_secs(60 * 60 * 24);

    let key = format!("steam:{appid}");
    let launch_attempt_started = Instant::now();
    let mut confirmed_started_at: Option<Instant> = None;

    loop {
        if let Some(started_at) = confirmed_started_at {
            if started_at.elapsed() > MAX_SESSION_DURATION {
                tracing::warn!(appid = %appid, "Przekroczono maksymalny czas obserwacji sesji Steam przez Web API — kończę jako zabezpieczenie");
                finish_steam_web_api_session(&app, &appid, &label, &key, started_at.elapsed().as_secs());
                return;
            }
        } else if launch_attempt_started.elapsed() > LAUNCH_GRACE_PERIOD {
            tracing::debug!(appid = %appid, "Steam Web API nie potwierdziło uruchomienia gry w oknie startowym");
            if notifications_enabled_for(&app, |n| n.on_game_exit) {
                crate::notifications::error(
                    "Gra mogła się nie uruchomić poprawnie",
                    &format!("{label}: Steam Web API nie potwierdziło uruchomienia gry w ciągu {}s", LAUNCH_GRACE_PERIOD.as_secs()),
                );
            }
            let _ = app.emit("hacker-mode://app-closed", serde_json::json!({ "label": label }));
            return;
        }

        match crate::commands::stores::steam::fetch_player_current_appid(&api_key, &steam_id64) {
            Some(Some(current_appid)) if current_appid == appid => {
                if confirmed_started_at.is_none() {
                    confirmed_started_at = Some(Instant::now());
                    if let Some(app_state) = app.try_state::<AppState>() {
                        app_state.steam_watched_games.lock().unwrap().insert(key.clone());
                    }
                    tracing::info!(appid = %appid, "Steam Web API potwierdziło uruchomienie gry");
                }
            }
            Some(_) => {
                // Zapytanie się powiodło, ale to NIE nasza gra (zamknięta
                // albo gracz przełączył się na coś innego) — jeśli
                // wcześniej potwierdziliśmy start, to KONIEC sesji.
                // Jeśli jeszcze nie potwierdziliśmy — wciąż jesteśmy w
                // oknie startowym, próbujemy dalej aż do `LAUNCH_GRACE_PERIOD`.
                if let Some(started_at) = confirmed_started_at {
                    finish_steam_web_api_session(&app, &appid, &label, &key, started_at.elapsed().as_secs());
                    return;
                }
            }
            None => {
                // Błąd pojedynczego zapytania (sieć/klucz) — ignorujemy,
                // patrz moduł-dokumentacja tej funkcji.
                tracing::debug!(appid = %appid, "Zapytanie do Steam Web API nie powiodło się (ignoruję pojedynczy błąd)");
            }
        }

        std::thread::sleep(if confirmed_started_at.is_some() { POLL_INTERVAL_RUNNING } else { POLL_INTERVAL_WAITING });
    }
}

/// Kończy sesję śledzoną przez `watch_steam_web_api_session`: dolicza
/// zmierzony (przez Steam Web API, nie przez czas życia procesu
/// `steam -applaunch`) czas gry, usuwa grę z
/// `AppState::steam_watched_games` i emituje te same zdarzenia
/// (`hacker-mode://game-exited`, `hacker-mode://app-closed`) co
/// standardowa ścieżka PID-owa w `run_wrapped` — więc z punktu widzenia
/// frontendu (`GameDetail.tsx`/`GameCard.tsx`) obie ścieżki wyglądają
/// identycznie, tylko źródło `secondsRan` jest inne (i dokładniejsze).
fn finish_steam_web_api_session(app: &AppHandle, appid: &str, label: &str, key: &str, seconds_ran: u64) {
    let minutes = seconds_ran / 60;
    crate::playtime::add_minutes("steam", appid, minutes);
    crate::playtime::record_session("steam", appid, minutes);

    if let Some(app_state) = app.try_state::<AppState>() {
        app_state.steam_watched_games.lock().unwrap().remove(key);
    }

    let _ = app.emit(
        "hacker-mode://game-exited",
        serde_json::json!({
            "platform": "steam",
            "gameId": appid,
            "label": label,
            "ok": true,
            "secondsRan": seconds_ran,
        }),
    );
    let _ = app.emit("hacker-mode://app-closed", serde_json::json!({ "label": label }));

    let crash_threshold = app
        .try_state::<AppState>()
        .map(|s| s.settings.lock().unwrap().crash_detection_threshold_seconds)
        .unwrap_or(8);
    if seconds_ran < crash_threshold && notifications_enabled_for(app, |n| n.on_game_exit) {
        crate::notifications::error(
            "Gra mogła się nie uruchomić poprawnie",
            &format!("{label} zamknęła się po {seconds_ran}s (wg Steam Web API)"),
        );
    }
}

fn enter_wrapper_mode(app: &AppHandle, state: &State<AppState>) {
    state.wrapper_active.store(true, Ordering::Relaxed);
    if let Some(win) = app.get_webview_window("shell") {
        let _ = win.hide();
    }
    // Chowamy też po stronie comphwde (nie tylko po stronie Tauri) —
    // patrz `enter_wrapper`'s doc comment: to `MinimizeWindow` na naszym
    // własnym oknie znalezionym po `SHELL_APP_ID`, ten sam prymityw,
    // którego comphwde już używa dla SDE.
    if let Err(e) = hacker_mode_ipc::enter_wrapper(IPC_CALL_TIMEOUT) {
        tracing::debug!(error = %e, "comphwde niedostępne przez IPC lub nie znaleziono własnego okna (pomijam)");
    }
}

fn exit_wrapper_mode(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        state.wrapper_active.store(false, Ordering::Relaxed);
    }
    if let Some(win) = app.get_webview_window("shell") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    if let Err(e) = hacker_mode_ipc::exit_wrapper(IPC_CALL_TIMEOUT) {
        tracing::debug!(error = %e, "comphwde niedostępne przez IPC lub nie znaleziono własnego okna (pomijam)");
    }
}

/// Zamyka wszystkie znane procesy launcherów/sklepów (odpowiednik
/// `restartApps` z wersji 0.1) i przywraca powłokę.
pub fn restart_apps(app: &AppHandle) -> ActionSummary {
    let targets = ["steam", "heroic", "hyperplay", "lutris", "legendary", "gogdl", "nile"];
    let mut killed = Vec::new();
    for target in targets {
        // BUGFIX: `Command::status()` zwraca `Ok(ExitStatus)` nawet gdy
        // `pkill` nie znalazł żadnego pasującego procesu (`pkill` kończy
        // się wtedy kodem 1 — to wciąż `Ok` z punktu widzenia Rusta,
        // `Err` byłby tylko, gdyby samego `pkill` nie dało się
        // uruchomić). Poprzednia wersja sprawdzała `.is_ok()`, więc
        // `killed` zawierało WSZYSTKIE targety niezależnie od tego, czy
        // cokolwiek faktycznie zabito — trzeba sprawdzić
        // `status.success()`.
        match Command::new("pkill").args(["-f", target]).status() {
            Ok(status) if status.success() => killed.push(target.to_string()),
            Ok(_) => {
                // `pkill` uruchomił się poprawnie, ale nic nie pasowało —
                // ten target po prostu nie był uruchomiony, to nie błąd.
            }
            Err(e) => {
                tracing::debug!(target, error = %e, "Nie udało się uruchomić `pkill`");
            }
        }
    }
    exit_wrapper_mode(app);
    ActionSummary { killed }
}

#[derive(serde::Serialize)]
pub struct ActionSummary {
    pub killed: Vec<String>,
}

/// Czy Hacker Mode ma zarejestrowany PID dla danej gry — patrz
/// `AppState::running_games`. Odpytywane przez frontend przy wejściu na
/// widok (`GameDetail.tsx`/`GameCard.tsx`), żeby od razu pokazać stan
/// "w trakcie gry" nawet bez odebrania zdarzenia `game-launched` (np. po
/// przejściu na inny widok i powrocie — zdarzenia Tauri nie mają
/// historii, nowy listener nie widzi zdarzeń sprzed jego rejestracji).
pub fn is_game_running(state: &State<AppState>, platform: Platform, game_id: &str) -> bool {
    let key = format!("{}:{game_id}", platform.slug());
    state.running_games.lock().unwrap().contains_key(&key)
        || state.steam_watched_games.lock().unwrap().contains(&key)
}

/// Zatrzymuje uruchomioną grę, wysyłając sygnał do zarejestrowanego PID-u
/// (patrz `AppState::running_games`) — `force: false` wysyła SIGTERM
/// (grzeczna prośba o zamknięcie, gra może dokończyć zapis stanu),
/// `force: true` wysyła SIGKILL (natychmiastowe zabicie, gdy SIGTERM nie
/// podziałał — patrz przycisk "Wymuś zamknięcie" w `GameDetail.tsx`,
/// pokazywany dopiero gdy zwykłe "Zatrzymaj" nie pomogło po kilku
/// sekundach). Nie usuwa PID-u z `running_games` samodzielnie — to i tak
/// zrobi wątek w `run_wrapped` czekający na `child.wait()`, gdy proces
/// faktycznie się zakończy (jedno źródło prawdy o tym, czy gra
/// naprawdę już nie działa, zamiast dwóch niezależnych miejsc, które
/// mogłyby się rozjechać).
///
/// UWAGA dla Steam (patrz `AppState::running_games`): jeśli Steam już
/// działał w momencie uruchomienia, zarejestrowany PID to proces
/// polecenia `steam -applaunch`, które kończy się niemal natychmiast po
/// przekazaniu żądania — w praktyce nie ma go już w `running_games` do
/// zabicia. Z tego powodu `GameDetail.tsx`/`GameCard.tsx` w ogóle nie
/// pokazują przycisku "Zatrzymaj" dla Steam, żeby nie sugerować
/// funkcjonalności, która i tak nic by nie zrobiła.
pub fn stop_game(state: &State<AppState>, platform: Platform, game_id: &str, force: bool) -> Result<(), String> {
    let key = format!("{}:{game_id}", platform.slug());
    let pid = state
        .running_games
        .lock()
        .unwrap()
        .get(&key)
        .copied()
        .ok_or("Ta gra nie jest aktualnie uruchomiona (albo Hacker Mode stracił jej ślad — patrz ograniczenie dla Steam)")?;

    let signal = if force { "-KILL" } else { "-TERM" };
    let status = Command::new("kill")
        .args([signal, &pid.to_string()])
        .status()
        .map_err(|e| format!("Nie udało się uruchomić `kill`: {e}"))?;
    if !status.success() {
        // Najczęstsza przyczyna: proces już się zakończył sam
        // (wyścig między kliknięciem "Zatrzymaj" a naturalnym końcem gry)
        // — `kill` na nieistniejącym PID-zie kończy się błędem, co tutaj
        // NIE jest traktowane jako poważny błąd, tylko zgłaszane wprost.
        return Err(format!("`kill` nie zdołał zatrzymać procesu {pid} (mógł się już zakończyć)"));
    }
    Ok(())
}
