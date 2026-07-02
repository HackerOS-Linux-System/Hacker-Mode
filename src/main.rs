mod cli;
mod commands;
mod compositor;
mod platforms;
mod system;

use cli::Mode;
use std::process::{Command, Stdio};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = cli::parse();

    match args.mode {
        // Tryb 1: okienkowy, w istniejącym środowisku graficznym.
        Mode::Ui => run_ui(),

        // Tryb 2: samodzielny — my JESTEŚMY kompozytorem (jak gamescope),
        // a UI odpalamy jako naszego własnego klienta Wayland.
        Mode::Default => run_standalone(),
    }
}

/// Odpala samą warstwę UI (Tauri + WebView) — używane zarówno bezpośrednio
/// (tryb "ui"), jak i jako proces potomny uruchamiany przez kompozytor
/// w trybie "default".
fn run_ui() -> anyhow::Result<()> {
    check_display_available()?;
    warn_and_patch_for_root();

    // GTK/WebKitGTK potrafi panikować (SIGABRT) zamiast zwrócić błąd, jeśli
    // inicjalizacja się nie powiedzie mimo obecności zmiennych środowiskowych
    // (np. socket Wayland/X istnieje, ale jest martwy). Łapiemy to i zamieniamy
    // na czytelny komunikat zamiast surowego backtrace'u z crate'a tao/gtk.
    let result = std::panic::catch_unwind(|| {
        tauri::Builder::default()
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_fs::init())
            .manage(commands::AppState::default())
            .invoke_handler(commands::handler())
            .run(tauri::generate_context!())
    });

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => anyhow::bail!("Błąd uruchamiania Tauri: {e}"),
        Err(_) => anyhow::bail!(
            "GTK/WebKit nie zainicjalizowały się poprawnie (SIGABRT). \
             Zwykle oznacza to, że proces nie ma dostępu do żadnego \
             środowiska graficznego (X11/Wayland) — patrz komunikat powyżej."
        ),
    }
}

/// WebKitGTK domyślnie uruchamia swój proces renderujący w piaskownicy
/// (bubblewrap), która odmawia startu, gdy cały proces działa jako root —
/// typowe w testach wewnątrz kontenerów Dockera (tak jak w Twoim przypadku).
/// Docelowo `hacker-mode` powinien działać jako zwykły użytkownik (tak jak
/// każda sesja graficzna spod SDDM), ale żeby test w kontenerze-jako-root
/// w ogóle wystartował, wyłączamy piaskownicę i jasno o tym informujemy —
/// zamiast cichego, mylącego błędu.
fn warn_and_patch_for_root() {
    let is_root = std::env::var("USER").map(|u| u == "root").unwrap_or(false)
        || euid_is_zero();

    if is_root && std::env::var("WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS").is_err() {
        tracing::warn!(
            "Uruchamiasz hacker-mode jako root — WebKitGTK domyślnie odmawia \
             wtedy startu piaskownicy renderera. Wyłączam piaskownicę WebKit \
             na potrzeby tego procesu (WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1). \
             W produkcji uruchamiaj hacker-mode jako zwykły użytkownik, nie root."
        );
        std::env::set_var("WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS", "1");
    }
}

/// Prosty test euid==0 bez dodatkowej zależności od crate `libc` w całym
/// projekcie — czytamy /proc/self/status, co działa identycznie na każdym
/// Linuksie.
fn euid_is_zero() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(2)?.parse::<u32>().ok())
        })
        .map(|euid| euid == 0)
        .unwrap_or(false)
}
/// podłączyć. Bez tego GTK potrafi zamiast czytelnego błędu wywalić SIGABRT
/// (`Failed to initialize GTK`) — dokładnie to, co widać w logu.
fn check_display_available() -> anyhow::Result<()> {
    let has_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    let has_x11 = std::env::var("DISPLAY").is_ok();

    if !has_wayland && !has_x11 {
        anyhow::bail!(
            "\n\
            Brak dostępnego środowiska graficznego — nie ustawiono ani \
            WAYLAND_DISPLAY, ani DISPLAY.\n\n\
            Najczęstsza przyczyna: `hacker-mode` jest uruchamiany w gołym \
            kontenerze Dockera / przez SSH bez X forwarding / na czystym \
            TTY bez sesji graficznej. GTK/WebKit (którego Tauri używa na \
            Linuksie) nie ma wtedy do czego się podłączyć.\n\n\
            Co zrobić:\n\
            • Na zwykłym pulpicie (X11/Wayland): po prostu uruchom `hacker-mode ui` \
              wewnątrz swojej sesji graficznej (np. w terminalu w KDE/GNOME), nie w kontenerze.\n\
            • Testowanie w kontenerze Docker: dodaj do `docker run` przekazanie X11:\n\
            \x20   -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix\n\
              (i wcześniej `xhost +local:docker` na hoście), albo doinstaluj Xvfb\n\
            \x20   i uruchom pod: `xvfb-run -a ./target/release/hacker-mode ui`.\n\
            • Tryb `hacker-mode default` (własny kompozytor) nie potrzebuje \
              DISPLAY/WAYLAND_DISPLAY z zewnątrz — sam je ustawia dla procesu \
              potomnego — ale wymaga uprawnień do DRM/seatd, patrz README.\n"
        );
    }
    Ok(())
}

/// Tryb "default": uruchamiany z SDDM na czystym TTY.
/// 1. Startujemy własny kompozytor Wayland (Smithay, backend DRM/udev).
/// 2. Gdy socket kompozytora jest gotowy, odpalamy SAMYCH SIEBIE ponownie,
///    tym razem z argumentem "ui", ustawiając WAYLAND_DISPLAY na nasz socket
///    — dzięki temu WebView (GTK/WebKit) łączy się jak zwykły klient Wayland
///    do kompozytora, który sami hostujemy. Dokładnie tak działa gamescope
///    względem Steama.
fn run_standalone() -> anyhow::Result<()> {
    let socket_name = compositor::start()?; // blokuje aż socket nasłuchuje, patrz compositor/mod.rs

    let exe = std::env::current_exe()?;
    let mut child = Command::new(exe)
        .arg("ui")
        .env("WAYLAND_DISPLAY", &socket_name)
        // wymuszamy backend Wayland dla WebKitGTK/Tauri, zamiast X11:
        .env("GDK_BACKEND", "wayland")
        .env("HACKER_MODE_STANDALONE", "1")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    // Główna pętla zdarzeń kompozytora (input, renderowanie, zarządzanie
    // oknami klientów — UI oraz uruchomionych gier) żyje w compositor::run().
    compositor::run(child.id())?;

    let _ = child.wait();
    Ok(())
}
