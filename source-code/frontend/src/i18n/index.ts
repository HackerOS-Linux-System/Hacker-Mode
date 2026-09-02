export type Lang = "pl" | "en" | "de" | "fr" | "es";

/** Rozbudowa o wiele języków (Ustawienia → Ogólne) — dawniej tylko
 * pl/en. Dodane de/fr/es to pełne, prawdziwe tłumaczenia (nie placeholder
 * kopiujący angielski) każdego klucza poniżej, łącznie z ekranem
 * powitalnym pierwszego uruchomienia (`onboarding_*`). */
export const LANGUAGES: { id: Lang; label: string }[] = [
  { id: "pl", label: "Polski" },
  { id: "en", label: "English" },
  { id: "de", label: "Deutsch" },
  { id: "fr", label: "Français" },
  { id: "es", label: "Español" },
];

/**
 * BUGFIX: wcześniej `TranslationKey` był wyprowadzany WYŁĄCZNIE z
 * `translations.pl` (`keyof typeof translations.pl`), a cały obiekt
 * `translations` był tylko `as const` — TypeScript nigdy nie sprawdzał, czy
 * `en`/`de`/`fr`/`es` mają te same klucze co `pl`. Brakujący albo
 * błędnie nazwany klucz w którymkolwiek z tych języków kompilowałby się
 * bez ostrzeżenia i po cichu spadał do angielskiego fallbacku w runtime
 * (patrz `getText`) — łatwo by to przeoczyć przy dodawaniu nowego klucza.
 *
 * Ten interfejs to jedno, kanoniczne źródło prawdy o tym, jakie klucze
 * muszą istnieć — `translations` niżej jest otypowany jako
 * `Record<Lang, Translation>` (nie `as const`), więc brakujący/nadmiarowy/
 * źle nazwany klucz w DOWOLNYM języku jest teraz błędem kompilacji `tsc`,
 * nie cichym fallbackiem odkrywanym dopiero w UI.
 */
export interface Translation {
  title: string;
  settings: string;
  hacker_menu: string;
  audio: string;
  display: string;
  network: string;
  power: string;
  general: string;
  gaming_tools: string;
  wifi_settings: string;
  bluetooth: string;
  increase_volume: string;
  decrease_volume: string;
  toggle_mute: string;
  increase_brightness: string;
  decrease_brightness: string;
  toggle_theme: string;
  toggle_wifi: string;
  connect: string;
  scan: string;
  pair: string;
  close: string;
  power_saving: string;
  balanced: string;
  performance: string;
  enable_gamescope: string;
  enable_mangohud: string;
  enable_vkbasalt: string;
  library: string;
  stats: string;
  store: string;
  home: string;
  app_not_installed: string;
  no_internet: string;
  launch_cooldown: string;
  wrapper_mode: string;
  shutdown: string;
  restart: string;
  sleep: string;
  switch_desktop: string;
  restart_apps: string;
  no_games_found: string;
  installed: string;
  visible_platforms: string;
  language: string;
  onboarding_welcome_title: string;
  onboarding_welcome_body: string;
  onboarding_language_step: string;
  onboarding_tools_title: string;
  onboarding_tools_body: string;
  onboarding_steam_note: string;
  onboarding_install: string;
  onboarding_skip: string;
  onboarding_finish: string;
  onboarding_installing: string;
  onboarding_already_available: string;
  // Panel SteamGridDB (Ustawienia → Sklepy) — patrz `steamgriddb.rs` i
  // `Settings.tsx`. Dodane w v0.7 razem z resztą kluczy niżej, żeby nowe
  // panele sklepów (dotąd hardkodowane po polsku, patrz README/CHANGELOG)
  // faktycznie przechodziły przez `getText` zamiast pomijać i18n.
  steamgriddb_panel_title: string;
  steamgriddb_panel_body: string;
  steamgriddb_api_key_label: string;
  // Panel wyszukiwania katalogu GOG (Sklep) — patrz `Store.tsx`.
  gog_search_title: string;
  gog_search_body: string;
  gog_search_placeholder: string;
  gog_search_button: string;
  gog_search_button_busy: string;
  gog_open_store_page: string;
  gog_no_store_page: string;
  // Dialog potwierdzenia przed deinstalacją (patrz `GameDetail.tsx`,
  // `uninstallWarning`) — treść ostrzeżenia różni się per platforma,
  // dlatego trzy osobne klucze zamiast jednego.
  uninstall_warning_destructive: string;
  uninstall_warning_lutris: string;
  uninstall_warning_generic: string;
  uninstall_confirm_button: string;
  uninstall_confirm_button_busy: string;
  uninstall_cancel_button: string;
  // Kopie zapasowe zapisów (Ustawienia) — patrz `cloud_saves.rs`.
  cloud_saves_panel_title: string;
  cloud_saves_panel_body: string;
  cloud_saves_backup_dir_label: string;
  sync_settings_panel_title: string;
  sync_settings_panel_body: string;
  sync_settings_export: string;
  sync_settings_import: string;
  external_apps_panel_title: string;
  external_apps_panel_body: string;
  advanced_launch_panel_title: string;
  advanced_launch_panel_body: string;
  custom_launch_prefix_label: string;
  crash_detection_threshold_label: string;
  notifications_panel_title: string;
  notifications_panel_body: string;
  notification_on_install: string;
  notification_on_game_exit: string;
  notification_on_backup_error: string;
  // Panel Proton/Wine (GameDetail.tsx)
  compat_tool_panel_title_steam: string;
  compat_tool_panel_title_lutris: string;
  compat_tool_empty_steam: string;
  compat_tool_empty_lutris: string;
  // Panel osiągnięć (GameDetail.tsx)
  achievements_loading: string;
  achievements_title: string;
  // Edytor tagów (GameDetail.tsx)
  tag_input_placeholder: string;
  // Pasek operacji zbiorczych (Library.tsx)
  bulk_select_mode: string;
  bulk_cancel_select_mode: string;
  bulk_search_placeholder: string;
  bulk_sort_name: string;
  bulk_sort_playtime: string;
  bulk_sort_installed: string;
  bulk_selected_count: string;
  bulk_install_selected: string;
  bulk_uninstall_selected: string;
  bulk_cancel: string;
  bulk_confirm_uninstall: string;
  bulk_confirm_install: string;
  bulk_confirm_yes: string;
  bulk_processing: string;
  bulk_completed: string;
  bulk_retry_failed: string;
  bulk_close: string;
}

export const translations: Record<Lang, Translation> = {
  pl: {
    title: "Hacker Mode",
    settings: "Ustawienia",
    hacker_menu: "Menu",
    audio: "Dźwięk",
    display: "Ekran",
    network: "Sieć",
    power: "Zasilanie",
    general: "Ogólne",
    gaming_tools: "Narzędzia do gier",
    wifi_settings: "Ustawienia Wi-Fi",
    bluetooth: "Bluetooth",
    increase_volume: "Zwiększ głośność",
    decrease_volume: "Zmniejsz głośność",
    toggle_mute: "Wycisz / wyłącz wyciszenie",
    increase_brightness: "Zwiększ jasność",
    decrease_brightness: "Zmniejsz jasność",
    toggle_theme: "Zmień motyw",
    toggle_wifi: "Włącz / wyłącz Wi-Fi",
    connect: "Połącz",
    scan: "Skanuj",
    pair: "Paruj",
    close: "Zamknij",
    power_saving: "Oszczędzanie energii",
    balanced: "Zrównoważony",
    performance: "Wydajność",
    enable_gamescope: "Włącz Gamescope",
    enable_mangohud: "Włącz MangoHud",
    enable_vkbasalt: "Włącz vkBasalt",
    library: "Biblioteka",
    stats: "Statystyki",
    store: "Sklepy",
    home: "Start",
    app_not_installed: "Ta aplikacja nie jest zainstalowana.",
    no_internet: "Brak połączenia z internetem.",
    launch_cooldown: "Poczekaj {seconds}s przed ponownym uruchomieniem „{app}”.",
    wrapper_mode: "Tryb wrapper (zamykaj Hacker Mode na czas gry)",
    shutdown: "Wyłącz",
    restart: "Uruchom ponownie",
    sleep: "Uśpij",
    switch_desktop: "Przełącz na pulpit",
    restart_apps: "Zrestartuj aplikacje",
    no_games_found: "Nie znaleziono żadnych gier. Zainstaluj i zaloguj się do sklepu, aby zobaczyć bibliotekę.",
    installed: "Zainstalowana",
    visible_platforms: "Widoczne platformy w bibliotece",
    language: "Język",
    onboarding_welcome_title: "Witaj w Hacker Mode",
    onboarding_welcome_body:
      "Hacker Mode to powłoka do gier w stylu Big Picture dla Linuksa — jedno miejsce na Twoją bibliotekę ze Steam, Epic, GOG, Amazon Games, Lutrisa, EA app i Battle.net, wygodne w obsłudze padem, na dużym ekranie.",
    onboarding_language_step: "Wybierz język",
    onboarding_tools_title: "Narzędzia do sklepów",
    onboarding_tools_body:
      "Żeby Hacker Mode widział gry z Epic Games, GOG i Amazon Games, potrzebne są trzy niewielkie narzędzia wiersza poleceń: legendary, gogdl i nile. Zainstalujemy je w osobnym, odizolowanym środowisku Pythona (~/.hackeros/Hacker-Mode/env/) — nie ruszając niczego innego w Twoim systemie.",
    onboarding_steam_note:
      "Steam nie jest tu obsługiwany — jeśli masz go zainstalowanego, Hacker Mode wykryje go automatycznie, bez żadnej dodatkowej instalacji.",
    onboarding_install: "Zainstaluj narzędzia",
    onboarding_skip: "Pomiń na razie",
    onboarding_finish: "Rozpocznij",
    onboarding_installing: "Instaluję…",
    onboarding_already_available: "już dostępne",
    steamgriddb_panel_title: "SteamGridDB — okładki dla EA app i Battle.net",
    steamgriddb_panel_body:
      "EA app i Battle.net nie udostępniają żadnego publicznego API artworku, więc Hacker Mode nie ma skąd wziąć dla nich okładek — poza tym polem. Podaj darmowy klucz API SteamGridDB (wygenerowany na steamgriddb.com/profile/preferences/api po zalogowaniu), a Hacker Mode spróbuje dopasować okładkę do każdej gry po tytule. Dopasowanie po tytule bywa niedokładne — w razie pomyłki popraw okładkę ręcznie przyciskiem „Zmień okładkę” w widoku danej gry, co zawsze ma pierwszeństwo.",
    steamgriddb_api_key_label: "Klucz API",
    gog_search_title: "GOG — wyszukaj w katalogu",
    gog_search_body:
      "Tak jak Steam wyżej — GOG też ma publiczne, nieuwierzytelnione API katalogu, więc wyszukiwanie idzie przez dane, nie przez stronę WWW. GOG nie ma na Linuksie działającego klienta z własnym schematem adresów — wynik otwiera się jako zwykła strona produktu w przeglądarce, skąd można kupić i (po zalogowaniu przez gogdl) zainstalować.",
    gog_search_placeholder: "Szukaj gry na GOG…",
    gog_search_button: "Szukaj",
    gog_search_button_busy: "…",
    gog_open_store_page: "Otwórz na GOG.com",
    gog_no_store_page: "Brak strony produktu",
    uninstall_warning_destructive:
      "Hacker Mode trwale usunie cały katalog instalacji tej gry z dysku (odpowiednik „rm -rf”) — {platform} nie udostępnia żadnego innego sposobu odinstalowania z linii poleceń. Tej operacji nie da się cofnąć.",
    uninstall_warning_lutris:
      "Gra zostanie oznaczona jako niezainstalowana bezpośrednio w bazie Lutrisa (bez kasowania plików z dysku — jak „Remove from library” w samym Lutrisie). Pliki gry zostaną na dysku; usuń je ręcznie, jeśli chcesz odzyskać miejsce.",
    uninstall_warning_generic:
      "{platform} odinstaluje tę grę ze swojego klienta. Tej operacji nie da się cofnąć z poziomu Hacker Mode.",
    uninstall_confirm_button: "Tak, odinstaluj",
    uninstall_confirm_button_busy: "Odinstalowuję…",
    uninstall_cancel_button: "Anuluj",
    cloud_saves_panel_title: "Kopie zapasowe zapisów",
    cloud_saves_panel_body:
      "Hacker Mode nie ma dostępu do prawdziwej chmury zapisów żadnej platformy (to prywatne API klientów) — zamiast tego trzyma własne, lokalne kopie zapasowe wskazanego katalogu zapisu każdej gry w miejscu, które wybierzesz tutaj. Jeśli wskażesz katalog synchronizowany przez Dropbox/Nextcloud/Syncthing, kopie faktycznie trafią „do chmury” tą drogą.",
    cloud_saves_backup_dir_label: "Katalog na kopie zapasowe",
    sync_settings_panel_title: "Eksport/import ustawień",
    sync_settings_panel_body: "Hacker Mode nie ma własnej synchronizacji ustawień w tle — możesz jednak wyeksportować wszystkie swoje ustawienia (tagi, ścieżki zapisów, mapowania kontrolerów, klucze API) do pliku i zaimportować go na innym urządzeniu.",
    sync_settings_export: "Eksportuj ustawienia",
    sync_settings_import: "Importuj ustawienia",
    external_apps_panel_title: "Wykryto poza Hacker Mode",
    external_apps_panel_body: "Aplikacje dodane ręcznie w Heroic (sideload) i prefiksy Wine zarządzane przez Bottles — wyłącznie informacyjnie, Hacker Mode nie zarządza nimi (uruchom/zainstaluj je z poziomu oryginalnego programu).",
    advanced_launch_panel_title: "Zaawansowane: uruchamianie",
    advanced_launch_panel_body: "Polecenie, którym owinięte zostanie KAŻDE uruchomienie gry — np. „gamemoderun” (Feral GameMode) albo „prime-run” (przełączanie na kartę graficzną NVIDIA Optimus). Nakładane na zewnątrz Gamescope, jeśli też jest włączony.",
    custom_launch_prefix_label: "Prefiks uruchamiania",
    crash_detection_threshold_label: "Próg wykrywania crasha (s)",
    notifications_panel_title: "Powiadomienia systemowe",
    notifications_panel_body: "Które zdarzenia mają wysyłać powiadomienie systemowe (notify-send) — przydatne, gdy Hacker Mode działa w tle podczas gry i mógłbyś nie zauważyć np. nieudanej instalacji.",
    notification_on_install: "Instalacja/deinstalacja zakończona",
    notification_on_game_exit: "Gra mogła się nie uruchomić poprawnie (szybkie zamknięcie/błąd)",
    notification_on_backup_error: "Automatyczna kopia zapasowa zapisu nie powiodła się",
    compat_tool_panel_title_steam: "Wersja Proton",
    compat_tool_panel_title_lutris: "Wersja Wine",
    compat_tool_empty_steam: "Brak niestandardowych wersji Protona (np. Proton-GE) w compatibilitytools.d — Hacker Mode pokazuje tu tylko te, ich oficjalnych wydań Protona nie da się bezpiecznie przypisać bez ryzyka błędnego zgadnięcia wewnętrznej nazwy.",
    compat_tool_empty_lutris: "Lutris nie ma jeszcze zainstalowanej żadnej własnej wersji Wine przez menedżer runnerów.",
    achievements_loading: "Wczytuję osiągnięcia…",
    achievements_title: "Osiągnięcia",
    tag_input_placeholder: "+ tag",
    bulk_select_mode: "Zaznacz kilka",
    bulk_cancel_select_mode: "Anuluj zaznaczanie",
    bulk_search_placeholder: "Szukaj w bibliotece…",
    bulk_sort_name: "Nazwa A-Z",
    bulk_sort_playtime: "Najwięcej godzin",
    bulk_sort_installed: "Zainstalowane najpierw",
    bulk_selected_count: "Zaznaczono",
    bulk_install_selected: "Zainstaluj zaznaczone",
    bulk_uninstall_selected: "Odinstaluj zaznaczone",
    bulk_cancel: "Anuluj",
    bulk_confirm_uninstall: "Na pewno odinstalować {count} gier? Tej operacji nie da się cofnąć.",
    bulk_confirm_install: "Zainstalować {count} gier?",
    bulk_confirm_yes: "Tak",
    bulk_processing: "Przetwarzanie…",
    bulk_completed: "Ukończono",
    bulk_retry_failed: "Ponów nieudane",
    bulk_close: "Zamknij",
  },
  en: {
    title: "Hacker Mode",
    settings: "Settings",
    hacker_menu: "Menu",
    audio: "Audio",
    display: "Display",
    network: "Network",
    power: "Power",
    general: "General",
    gaming_tools: "Gaming tools",
    wifi_settings: "Wi-Fi settings",
    bluetooth: "Bluetooth",
    increase_volume: "Increase volume",
    decrease_volume: "Decrease volume",
    toggle_mute: "Toggle mute",
    increase_brightness: "Increase brightness",
    decrease_brightness: "Decrease brightness",
    toggle_theme: "Toggle theme",
    toggle_wifi: "Toggle Wi-Fi",
    connect: "Connect",
    scan: "Scan",
    pair: "Pair",
    close: "Close",
    power_saving: "Power saving",
    balanced: "Balanced",
    performance: "Performance",
    enable_gamescope: "Enable Gamescope",
    enable_mangohud: "Enable MangoHud",
    enable_vkbasalt: "Enable vkBasalt",
    library: "Library",
    stats: "Stats",
    store: "Stores",
    home: "Home",
    app_not_installed: "This application is not installed.",
    no_internet: "No internet connection.",
    launch_cooldown: "Wait {seconds}s before launching \u201e{app}\u201c again.",
    wrapper_mode: "Wrapper mode (close Hacker Mode while playing)",
    shutdown: "Shut down",
    restart: "Restart",
    sleep: "Sleep",
    switch_desktop: "Switch to desktop",
    restart_apps: "Restart apps",
    no_games_found: "No games found. Install and log in to a store to see your library.",
    installed: "Installed",
    visible_platforms: "Visible platforms in library",
    language: "Language",
    onboarding_welcome_title: "Welcome to Hacker Mode",
    onboarding_welcome_body:
      "Hacker Mode is a Big Picture-style gaming shell for Linux — one place for your library from Steam, Epic, GOG, Amazon Games, Lutris, EA app, and Battle.net, built for controller navigation on a big screen.",
    onboarding_language_step: "Choose your language",
    onboarding_tools_title: "Store tools",
    onboarding_tools_body:
      "For Hacker Mode to see your Epic Games, GOG, and Amazon Games libraries, it needs three small command-line tools: legendary, gogdl, and nile. We'll install them into a separate, isolated Python environment (~/.hackeros/Hacker-Mode/env/) without touching anything else on your system.",
    onboarding_steam_note:
      "Steam isn't handled here — if it's installed, Hacker Mode will detect it automatically, no extra install needed.",
    onboarding_install: "Install tools",
    onboarding_skip: "Skip for now",
    onboarding_finish: "Get started",
    onboarding_installing: "Installing…",
    onboarding_already_available: "already available",
    steamgriddb_panel_title: "SteamGridDB — covers for EA app and Battle.net",
    steamgriddb_panel_body:
      "EA app and Battle.net don't expose any public artwork API, so Hacker Mode has no way to get covers for them except here. Provide a free SteamGridDB API key (generated at steamgriddb.com/profile/preferences/api after signing in) and Hacker Mode will try to match a cover to each game by its title. Title matching is sometimes off — fix it manually with \"Change cover\" in the game's detail view, which always takes priority.",
    steamgriddb_api_key_label: "API key",
    gog_search_title: "GOG — search the catalog",
    gog_search_body:
      "Same as Steam above — GOG also has a public, unauthenticated catalog API, so search goes through data, not a web page. GOG doesn't have a working client on Linux with its own URL scheme — the result opens as a regular product page in the browser, from which you can buy and (after logging in via gogdl) install.",
    gog_search_placeholder: "Search for a game on GOG…",
    gog_search_button: "Search",
    gog_search_button_busy: "…",
    gog_open_store_page: "Open on GOG.com",
    gog_no_store_page: "No product page",
    uninstall_warning_destructive:
      "Hacker Mode will permanently delete this game's entire install directory from disk (equivalent to \"rm -rf\") — {platform} offers no other way to uninstall from the command line. This cannot be undone.",
    uninstall_warning_lutris:
      "The game will be marked as not installed directly in Lutris's own database (without deleting files from disk — like \"Remove from library\" in Lutris itself). The game's files will remain on disk; delete them manually if you want to reclaim the space.",
    uninstall_warning_generic:
      "{platform} will uninstall this game from its own client. This cannot be undone from Hacker Mode.",
    uninstall_confirm_button: "Yes, uninstall",
    uninstall_confirm_button_busy: "Uninstalling…",
    uninstall_cancel_button: "Cancel",
    cloud_saves_panel_title: "Save backups",
    cloud_saves_panel_body:
      "Hacker Mode doesn't have access to any platform's real save cloud (that's a private API of each client) — instead it keeps its own local backups of each game's save directory in a location you choose here. If you point it at a folder synced by Dropbox/Nextcloud/Syncthing, backups will actually reach \"the cloud\" that way.",
    cloud_saves_backup_dir_label: "Backup folder",
    sync_settings_panel_title: "Export/import settings",
    sync_settings_panel_body: "Hacker Mode doesn't have its own background settings sync — but you can export all your settings (tags, save paths, controller mappings, API keys) to a file and import it on another device.",
    sync_settings_export: "Export settings",
    sync_settings_import: "Import settings",
    external_apps_panel_title: "Detected outside Hacker Mode",
    external_apps_panel_body: "Apps added manually in Heroic (sideload) and Wine prefixes managed by Bottles — informational only, Hacker Mode doesn't manage them (launch/install them from the original program).",
    advanced_launch_panel_title: "Advanced: launching",
    advanced_launch_panel_body: "A command every game launch gets wrapped in — e.g. \"gamemoderun\" (Feral GameMode) or \"prime-run\" (NVIDIA Optimus GPU switching). Applied outside Gamescope, if that's also enabled.",
    custom_launch_prefix_label: "Launch prefix",
    crash_detection_threshold_label: "Crash detection threshold (s)",
    notifications_panel_title: "System notifications",
    notifications_panel_body: "Which events send a system notification (notify-send) — useful when Hacker Mode is running in the background during a game and you might miss, say, a failed install.",
    notification_on_install: "Install/uninstall finished",
    notification_on_game_exit: "Game may have failed to start (quick exit/error)",
    notification_on_backup_error: "Automatic save backup failed",
    compat_tool_panel_title_steam: "Proton version",
    compat_tool_panel_title_lutris: "Wine version",
    compat_tool_empty_steam: "No custom Proton versions (e.g. Proton-GE) in compatibilitytools.d — Hacker Mode only shows these, official Proton releases can't be safely assigned without risking guessing the internal name wrong.",
    compat_tool_empty_lutris: "Lutris doesn't have any of its own Wine versions installed via the runner manager yet.",
    achievements_loading: "Loading achievements…",
    achievements_title: "Achievements",
    tag_input_placeholder: "+ tag",
    bulk_select_mode: "Select multiple",
    bulk_cancel_select_mode: "Cancel selection",
    bulk_search_placeholder: "Search your library…",
    bulk_sort_name: "Name A-Z",
    bulk_sort_playtime: "Most hours",
    bulk_sort_installed: "Installed first",
    bulk_selected_count: "Selected",
    bulk_install_selected: "Install selected",
    bulk_uninstall_selected: "Uninstall selected",
    bulk_cancel: "Cancel",
    bulk_confirm_uninstall: "Uninstall {count} games? This can't be undone.",
    bulk_confirm_install: "Install {count} games?",
    bulk_confirm_yes: "Yes",
    bulk_processing: "Processing…",
    bulk_completed: "Completed",
    bulk_retry_failed: "Retry failed",
    bulk_close: "Close",
  },
  de: {
    title: "Hacker Mode",
    settings: "Einstellungen",
    hacker_menu: "Menü",
    audio: "Ton",
    display: "Anzeige",
    network: "Netzwerk",
    power: "Energie",
    general: "Allgemein",
    gaming_tools: "Gaming-Tools",
    wifi_settings: "WLAN-Einstellungen",
    bluetooth: "Bluetooth",
    increase_volume: "Lautstärke erhöhen",
    decrease_volume: "Lautstärke verringern",
    toggle_mute: "Stummschaltung umschalten",
    increase_brightness: "Helligkeit erhöhen",
    decrease_brightness: "Helligkeit verringern",
    toggle_theme: "Design wechseln",
    toggle_wifi: "WLAN umschalten",
    connect: "Verbinden",
    scan: "Scannen",
    pair: "Koppeln",
    close: "Schließen",
    power_saving: "Energiesparen",
    balanced: "Ausgewogen",
    performance: "Leistung",
    enable_gamescope: "Gamescope aktivieren",
    enable_mangohud: "MangoHud aktivieren",
    enable_vkbasalt: "vkBasalt aktivieren",
    library: "Bibliothek",
    stats: "Statistiken",
    store: "Stores",
    home: "Start",
    app_not_installed: "Diese Anwendung ist nicht installiert.",
    no_internet: "Keine Internetverbindung.",
    launch_cooldown: "Warte {seconds}s, bevor du „{app}“ erneut startest.",
    wrapper_mode: "Wrapper-Modus (Hacker Mode während des Spielens schließen)",
    shutdown: "Herunterfahren",
    restart: "Neu starten",
    sleep: "Ruhezustand",
    switch_desktop: "Zum Desktop wechseln",
    restart_apps: "Anwendungen neu starten",
    no_games_found: "Keine Spiele gefunden. Installiere einen Store und melde dich an, um deine Bibliothek zu sehen.",
    installed: "Installiert",
    visible_platforms: "Sichtbare Plattformen in der Bibliothek",
    language: "Sprache",
    onboarding_welcome_title: "Willkommen bei Hacker Mode",
    onboarding_welcome_body:
      "Hacker Mode ist eine Big-Picture-Gaming-Oberfläche für Linux — ein Ort für deine Bibliothek von Steam, Epic, GOG, Amazon Games, Lutris, EA app und Battle.net, gestaltet für die Steuerung mit dem Controller auf einem großen Bildschirm.",
    onboarding_language_step: "Sprache wählen",
    onboarding_tools_title: "Store-Tools",
    onboarding_tools_body:
      "Damit Hacker Mode deine Epic-Games-, GOG- und Amazon-Games-Bibliothek sehen kann, werden drei kleine Kommandozeilen-Tools benötigt: legendary, gogdl und nile. Wir installieren sie in einer separaten, isolierten Python-Umgebung (~/.hackeros/Hacker-Mode/env/), ohne den Rest deines Systems zu verändern.",
    onboarding_steam_note:
      "Steam wird hier nicht behandelt — falls installiert, erkennt Hacker Mode es automatisch, ohne zusätzliche Installation.",
    onboarding_install: "Tools installieren",
    onboarding_skip: "Vorerst überspringen",
    onboarding_finish: "Loslegen",
    onboarding_installing: "Installiere…",
    onboarding_already_available: "bereits verfügbar",
    steamgriddb_panel_title: "SteamGridDB — Cover für EA app und Battle.net",
    steamgriddb_panel_body:
      "EA app und Battle.net bieten keine öffentliche Artwork-API, daher hat Hacker Mode außer hier keine Möglichkeit, Cover für sie zu bekommen. Gib einen kostenlosen SteamGridDB-API-Schlüssel an (erstellt auf steamgriddb.com/profile/preferences/api nach dem Login), und Hacker Mode versucht, für jedes Spiel anhand des Titels ein Cover zu finden. Der Titel-Abgleich ist manchmal ungenau — korrigiere ihn bei Bedarf manuell über „Cover ändern“ in der Spielansicht, das immer Vorrang hat.",
    steamgriddb_api_key_label: "API-Schlüssel",
    gog_search_title: "GOG — Katalog durchsuchen",
    gog_search_body:
      "Genau wie Steam oben — auch GOG hat eine öffentliche, nicht authentifizierte Katalog-API, daher läuft die Suche über Daten statt über eine Webseite. GOG hat unter Linux keinen funktionierenden Client mit eigenem URL-Schema — das Ergebnis öffnet sich als normale Produktseite im Browser, von der aus man kaufen und (nach Anmeldung über gogdl) installieren kann.",
    gog_search_placeholder: "Spiel auf GOG suchen…",
    gog_search_button: "Suchen",
    gog_search_button_busy: "…",
    gog_open_store_page: "Auf GOG.com öffnen",
    gog_no_store_page: "Keine Produktseite",
    uninstall_warning_destructive:
      "Hacker Mode löscht das gesamte Installationsverzeichnis dieses Spiels dauerhaft von der Festplatte (entspricht „rm -rf“) — {platform} bietet keine andere Möglichkeit zur Deinstallation über die Kommandozeile. Dieser Vorgang lässt sich nicht rückgängig machen.",
    uninstall_warning_lutris:
      "Das Spiel wird direkt in der Lutris-Datenbank als nicht installiert markiert (ohne Dateien von der Festplatte zu löschen — wie „Remove from library“ in Lutris selbst). Die Spieldateien bleiben auf der Festplatte; lösche sie manuell, um Speicherplatz zurückzugewinnen.",
    uninstall_warning_generic:
      "{platform} deinstalliert dieses Spiel über den eigenen Client. Dieser Vorgang lässt sich von Hacker Mode aus nicht rückgängig machen.",
    uninstall_confirm_button: "Ja, deinstallieren",
    uninstall_confirm_button_busy: "Deinstalliere…",
    uninstall_cancel_button: "Abbrechen",
    cloud_saves_panel_title: "Spielstand-Backups",
    cloud_saves_panel_body:
      "Hacker Mode hat keinen Zugriff auf die echte Save-Cloud irgendeiner Plattform (das ist eine private API des jeweiligen Clients) — stattdessen legt es eigene, lokale Backups des Spielstand-Ordners jedes Spiels an dem hier gewählten Ort an. Wenn du einen von Dropbox/Nextcloud/Syncthing synchronisierten Ordner angibst, gelangen die Backups auf diesem Weg tatsächlich \"in die Cloud\".",
    cloud_saves_backup_dir_label: "Backup-Ordner",
    sync_settings_panel_title: "Einstellungen exportieren/importieren",
    sync_settings_panel_body: "Hacker Mode hat keine eigene Hintergrund-Synchronisierung der Einstellungen — du kannst aber alle deine Einstellungen (Tags, Speicherpfade, Controller-Zuordnungen, API-Schlüssel) in eine Datei exportieren und auf einem anderen Gerät importieren.",
    sync_settings_export: "Einstellungen exportieren",
    sync_settings_import: "Einstellungen importieren",
    external_apps_panel_title: "Außerhalb von Hacker Mode erkannt",
    external_apps_panel_body: "Manuell in Heroic hinzugefügte Apps (Sideload) und von Bottles verwaltete Wine-Prefixe — nur informativ, Hacker Mode verwaltet sie nicht (starte/installiere sie über das ursprüngliche Programm).",
    advanced_launch_panel_title: "Erweitert: Starten",
    advanced_launch_panel_body: "Ein Befehl, in den JEDER Spielstart eingebettet wird — z. B. „gamemoderun“ (Feral GameMode) oder „prime-run“ (NVIDIA-Optimus-GPU-Umschaltung). Wird außerhalb von Gamescope angewendet, falls dieses ebenfalls aktiviert ist.",
    custom_launch_prefix_label: "Start-Präfix",
    crash_detection_threshold_label: "Absturzerkennungsschwelle (s)",
    notifications_panel_title: "Systembenachrichtigungen",
    notifications_panel_body: "Welche Ereignisse eine Systembenachrichtigung (notify-send) senden — nützlich, wenn Hacker Mode während des Spiels im Hintergrund läuft und du z. B. eine fehlgeschlagene Installation verpassen könntest.",
    notification_on_install: "Installation/Deinstallation abgeschlossen",
    notification_on_game_exit: "Spiel konnte möglicherweise nicht starten (schnelles Beenden/Fehler)",
    notification_on_backup_error: "Automatisches Spielstand-Backup fehlgeschlagen",
    compat_tool_panel_title_steam: "Proton-Version",
    compat_tool_panel_title_lutris: "Wine-Version",
    compat_tool_empty_steam: "Keine benutzerdefinierten Proton-Versionen (z. B. Proton-GE) in compatibilitytools.d — Hacker Mode zeigt nur diese, offizielle Proton-Versionen können nicht sicher zugewiesen werden, ohne den internen Namen falsch zu erraten.",
    compat_tool_empty_lutris: "Lutris hat noch keine eigene Wine-Version über den Runner-Manager installiert.",
    achievements_loading: "Erfolge werden geladen…",
    achievements_title: "Erfolge",
    tag_input_placeholder: "+ Tag",
    bulk_select_mode: "Mehrere auswählen",
    bulk_cancel_select_mode: "Auswahl abbrechen",
    bulk_search_placeholder: "Bibliothek durchsuchen…",
    bulk_sort_name: "Name A-Z",
    bulk_sort_playtime: "Meiste Stunden",
    bulk_sort_installed: "Installierte zuerst",
    bulk_selected_count: "Ausgewählt",
    bulk_install_selected: "Ausgewählte installieren",
    bulk_uninstall_selected: "Ausgewählte deinstallieren",
    bulk_cancel: "Abbrechen",
    bulk_confirm_uninstall: "{count} Spiele deinstallieren? Dies kann nicht rückgängig gemacht werden.",
    bulk_confirm_install: "{count} Spiele installieren?",
    bulk_confirm_yes: "Ja",
    bulk_processing: "Verarbeitung…",
    bulk_completed: "Abgeschlossen",
    bulk_retry_failed: "Fehlgeschlagene wiederholen",
    bulk_close: "Schließen",
  },
  fr: {
    title: "Hacker Mode",
    settings: "Paramètres",
    hacker_menu: "Menu",
    audio: "Audio",
    display: "Écran",
    network: "Réseau",
    power: "Alimentation",
    general: "Général",
    gaming_tools: "Outils de jeu",
    wifi_settings: "Paramètres Wi-Fi",
    bluetooth: "Bluetooth",
    increase_volume: "Augmenter le volume",
    decrease_volume: "Diminuer le volume",
    toggle_mute: "Activer/désactiver le son",
    increase_brightness: "Augmenter la luminosité",
    decrease_brightness: "Diminuer la luminosité",
    toggle_theme: "Changer de thème",
    toggle_wifi: "Activer/désactiver le Wi-Fi",
    connect: "Connecter",
    scan: "Scanner",
    pair: "Associer",
    close: "Fermer",
    power_saving: "Économie d'énergie",
    balanced: "Équilibré",
    performance: "Performance",
    enable_gamescope: "Activer Gamescope",
    enable_mangohud: "Activer MangoHud",
    enable_vkbasalt: "Activer vkBasalt",
    library: "Bibliothèque",
    stats: "Statistiques",
    store: "Boutiques",
    home: "Accueil",
    app_not_installed: "Cette application n'est pas installée.",
    no_internet: "Aucune connexion internet.",
    launch_cooldown: "Attendez {seconds}s avant de relancer « {app} ».",
    wrapper_mode: "Mode wrapper (fermer Hacker Mode pendant le jeu)",
    shutdown: "Éteindre",
    restart: "Redémarrer",
    sleep: "Veille",
    switch_desktop: "Passer au bureau",
    restart_apps: "Redémarrer les applications",
    no_games_found: "Aucun jeu trouvé. Installez une boutique et connectez-vous pour voir votre bibliothèque.",
    installed: "Installé",
    visible_platforms: "Plateformes visibles dans la bibliothèque",
    language: "Langue",
    onboarding_welcome_title: "Bienvenue dans Hacker Mode",
    onboarding_welcome_body:
      "Hacker Mode est une interface de jeu façon Big Picture pour Linux — un seul endroit pour votre bibliothèque Steam, Epic, GOG, Amazon Games, Lutris, EA app et Battle.net, pensée pour être utilisée à la manette sur grand écran.",
    onboarding_language_step: "Choisissez votre langue",
    onboarding_tools_title: "Outils des boutiques",
    onboarding_tools_body:
      "Pour que Hacker Mode voie vos bibliothèques Epic Games, GOG et Amazon Games, trois petits outils en ligne de commande sont nécessaires : legendary, gogdl et nile. Nous les installerons dans un environnement Python isolé (~/.hackeros/Hacker-Mode/env/), sans toucher au reste de votre système.",
    onboarding_steam_note:
      "Steam n'est pas géré ici — s'il est installé, Hacker Mode le détectera automatiquement, sans installation supplémentaire.",
    onboarding_install: "Installer les outils",
    onboarding_skip: "Ignorer pour l'instant",
    onboarding_finish: "Commencer",
    onboarding_installing: "Installation…",
    onboarding_already_available: "déjà disponible",
    steamgriddb_panel_title: "SteamGridDB — jaquettes pour EA app et Battle.net",
    steamgriddb_panel_body:
      "EA app et Battle.net n'offrent aucune API publique d'illustrations, donc Hacker Mode n'a aucun moyen d'obtenir des jaquettes pour eux, sauf ici. Indiquez une clé API SteamGridDB gratuite (générée sur steamgriddb.com/profile/preferences/api après connexion) et Hacker Mode essaiera de trouver une jaquette pour chaque jeu par son titre. La correspondance par titre est parfois imprécise — corrigez-la manuellement via « Changer la jaquette » dans la fiche du jeu, qui est toujours prioritaire.",
    steamgriddb_api_key_label: "Clé API",
    gog_search_title: "GOG — rechercher dans le catalogue",
    gog_search_body:
      "Comme Steam ci-dessus — GOG dispose aussi d'une API de catalogue publique et non authentifiée, donc la recherche passe par les données, pas par une page web. GOG n'a pas sous Linux de client fonctionnel avec son propre schéma d'adresses — le résultat s'ouvre comme une page produit classique dans le navigateur, depuis laquelle on peut acheter et installer (après connexion via gogdl).",
    gog_search_placeholder: "Rechercher un jeu sur GOG…",
    gog_search_button: "Rechercher",
    gog_search_button_busy: "…",
    gog_open_store_page: "Ouvrir sur GOG.com",
    gog_no_store_page: "Pas de page produit",
    uninstall_warning_destructive:
      "Hacker Mode supprimera définitivement tout le répertoire d'installation de ce jeu du disque (équivalent à « rm -rf ») — {platform} n'offre aucun autre moyen de désinstaller en ligne de commande. Cette opération est irréversible.",
    uninstall_warning_lutris:
      "Le jeu sera marqué comme non installé directement dans la base de données de Lutris (sans supprimer les fichiers du disque — comme « Remove from library » dans Lutris lui-même). Les fichiers du jeu resteront sur le disque ; supprimez-les manuellement pour récupérer de l'espace.",
    uninstall_warning_generic:
      "{platform} désinstallera ce jeu depuis son propre client. Cette opération est irréversible depuis Hacker Mode.",
    uninstall_confirm_button: "Oui, désinstaller",
    uninstall_confirm_button_busy: "Désinstallation…",
    uninstall_cancel_button: "Annuler",
    cloud_saves_panel_title: "Sauvegardes des parties",
    cloud_saves_panel_body:
      "Hacker Mode n'a pas accès au vrai cloud de sauvegarde d'une plateforme (c'est une API privée de chaque client) — il conserve à la place ses propres sauvegardes locales du dossier de sauvegarde de chaque jeu, à l'emplacement choisi ici. Si vous indiquez un dossier synchronisé par Dropbox/Nextcloud/Syncthing, les sauvegardes atteindront réellement \"le cloud\" par ce biais.",
    cloud_saves_backup_dir_label: "Dossier de sauvegarde",
    sync_settings_panel_title: "Exporter/importer les paramètres",
    sync_settings_panel_body: "Hacker Mode n'a pas de synchronisation des paramètres en arrière-plan — mais vous pouvez exporter tous vos paramètres (tags, chemins de sauvegarde, mappages de manette, clés API) vers un fichier et l'importer sur un autre appareil.",
    sync_settings_export: "Exporter les paramètres",
    sync_settings_import: "Importer les paramètres",
    external_apps_panel_title: "Détecté en dehors de Hacker Mode",
    external_apps_panel_body: "Applications ajoutées manuellement dans Heroic (sideload) et préfixes Wine gérés par Bottles — à titre informatif uniquement, Hacker Mode ne les gère pas (lancez/installez-les depuis le programme d'origine).",
    advanced_launch_panel_title: "Avancé : lancement",
    advanced_launch_panel_body: "Une commande dans laquelle CHAQUE lancement de jeu est enveloppé — p. ex. « gamemoderun » (Feral GameMode) ou « prime-run » (bascule GPU NVIDIA Optimus). Appliquée à l'extérieur de Gamescope, si celui-ci est aussi activé.",
    custom_launch_prefix_label: "Préfixe de lancement",
    crash_detection_threshold_label: "Seuil de détection de crash (s)",
    notifications_panel_title: "Notifications système",
    notifications_panel_body: "Quels événements envoient une notification système (notify-send) — utile quand Hacker Mode tourne en arrière-plan pendant une partie et que vous pourriez manquer, par exemple, une installation échouée.",
    notification_on_install: "Installation/désinstallation terminée",
    notification_on_game_exit: "Le jeu ne s'est peut-être pas lancé correctement (fermeture rapide/erreur)",
    notification_on_backup_error: "Échec de la sauvegarde automatique de la partie",
    compat_tool_panel_title_steam: "Version de Proton",
    compat_tool_panel_title_lutris: "Version de Wine",
    compat_tool_empty_steam: "Aucune version personnalisée de Proton (ex. Proton-GE) dans compatibilitytools.d — Hacker Mode n'affiche que celles-ci, les versions officielles de Proton ne peuvent pas être assignées en toute sécurité sans risquer de deviner incorrectement le nom interne.",
    compat_tool_empty_lutris: "Lutris n'a encore aucune version de Wine installée via son gestionnaire de runners.",
    achievements_loading: "Chargement des succès…",
    achievements_title: "Succès",
    tag_input_placeholder: "+ tag",
    bulk_select_mode: "Sélectionner plusieurs",
    bulk_cancel_select_mode: "Annuler la sélection",
    bulk_search_placeholder: "Rechercher dans la bibliothèque…",
    bulk_sort_name: "Nom A-Z",
    bulk_sort_playtime: "Le plus d'heures",
    bulk_sort_installed: "Installés d'abord",
    bulk_selected_count: "Sélectionnés",
    bulk_install_selected: "Installer la sélection",
    bulk_uninstall_selected: "Désinstaller la sélection",
    bulk_cancel: "Annuler",
    bulk_confirm_uninstall: "Désinstaller {count} jeux ? Cette opération est irréversible.",
    bulk_confirm_install: "Installer {count} jeux ?",
    bulk_confirm_yes: "Oui",
    bulk_processing: "Traitement…",
    bulk_completed: "Terminé",
    bulk_retry_failed: "Réessayer les échecs",
    bulk_close: "Fermer",
  },
  es: {
    title: "Hacker Mode",
    settings: "Ajustes",
    hacker_menu: "Menú",
    audio: "Audio",
    display: "Pantalla",
    network: "Red",
    power: "Energía",
    general: "General",
    gaming_tools: "Herramientas de juego",
    wifi_settings: "Ajustes de Wi-Fi",
    bluetooth: "Bluetooth",
    increase_volume: "Subir volumen",
    decrease_volume: "Bajar volumen",
    toggle_mute: "Silenciar / activar sonido",
    increase_brightness: "Subir brillo",
    decrease_brightness: "Bajar brillo",
    toggle_theme: "Cambiar tema",
    toggle_wifi: "Activar/desactivar Wi-Fi",
    connect: "Conectar",
    scan: "Buscar",
    pair: "Emparejar",
    close: "Cerrar",
    power_saving: "Ahorro de energía",
    balanced: "Equilibrado",
    performance: "Rendimiento",
    enable_gamescope: "Activar Gamescope",
    enable_mangohud: "Activar MangoHud",
    enable_vkbasalt: "Activar vkBasalt",
    library: "Biblioteca",
    stats: "Estadísticas",
    store: "Tiendas",
    home: "Inicio",
    app_not_installed: "Esta aplicación no está instalada.",
    no_internet: "Sin conexión a internet.",
    launch_cooldown: "Espera {seconds}s antes de volver a abrir «{app}».",
    wrapper_mode: "Modo wrapper (cerrar Hacker Mode mientras juegas)",
    shutdown: "Apagar",
    restart: "Reiniciar",
    sleep: "Suspender",
    switch_desktop: "Cambiar al escritorio",
    restart_apps: "Reiniciar aplicaciones",
    no_games_found: "No se encontraron juegos. Instala una tienda e inicia sesión para ver tu biblioteca.",
    installed: "Instalado",
    visible_platforms: "Plataformas visibles en la biblioteca",
    language: "Idioma",
    onboarding_welcome_title: "Bienvenido a Hacker Mode",
    onboarding_welcome_body:
      "Hacker Mode es una interfaz de juego estilo Big Picture para Linux — un solo lugar para tu biblioteca de Steam, Epic, GOG, Amazon Games, Lutris, EA app y Battle.net, pensada para usarse con mando en una pantalla grande.",
    onboarding_language_step: "Elige tu idioma",
    onboarding_tools_title: "Herramientas de las tiendas",
    onboarding_tools_body:
      "Para que Hacker Mode vea tu biblioteca de Epic Games, GOG y Amazon Games, necesita tres pequeñas herramientas de línea de comandos: legendary, gogdl y nile. Las instalaremos en un entorno de Python aislado (~/.hackeros/Hacker-Mode/env/), sin tocar nada más de tu sistema.",
    onboarding_steam_note:
      "Steam no se gestiona aquí — si está instalado, Hacker Mode lo detectará automáticamente, sin instalación adicional.",
    onboarding_install: "Instalar herramientas",
    onboarding_skip: "Omitir por ahora",
    onboarding_finish: "Empezar",
    onboarding_installing: "Instalando…",
    onboarding_already_available: "ya disponible",
    steamgriddb_panel_title: "SteamGridDB — carátulas para EA app y Battle.net",
    steamgriddb_panel_body:
      "EA app y Battle.net no ofrecen ninguna API pública de artwork, así que Hacker Mode no tiene de dónde sacar carátulas para ellas, salvo por esto. Indica una clave API gratuita de SteamGridDB (generada en steamgriddb.com/profile/preferences/api tras iniciar sesión) y Hacker Mode intentará encontrar una carátula para cada juego por su título. La coincidencia por título a veces falla — corrígela manualmente con «Cambiar carátula» en la vista del juego, que siempre tiene prioridad.",
    steamgriddb_api_key_label: "Clave API",
    gog_search_title: "GOG — buscar en el catálogo",
    gog_search_body:
      "Igual que Steam arriba — GOG también tiene una API de catálogo pública y sin autenticación, así que la búsqueda pasa por datos, no por una página web. GOG no tiene en Linux un cliente funcional con su propio esquema de enlaces — el resultado se abre como una página de producto normal en el navegador, desde donde se puede comprar e instalar (tras iniciar sesión con gogdl).",
    gog_search_placeholder: "Buscar un juego en GOG…",
    gog_search_button: "Buscar",
    gog_search_button_busy: "…",
    gog_open_store_page: "Abrir en GOG.com",
    gog_no_store_page: "Sin página de producto",
    uninstall_warning_destructive:
      "Hacker Mode eliminará permanentemente todo el directorio de instalación de este juego del disco (equivalente a «rm -rf») — {platform} no ofrece ninguna otra forma de desinstalar desde la línea de comandos. Esta operación no se puede deshacer.",
    uninstall_warning_lutris:
      "El juego se marcará como no instalado directamente en la base de datos de Lutris (sin borrar archivos del disco — como «Remove from library» en el propio Lutris). Los archivos del juego permanecerán en el disco; bórralos manualmente si quieres recuperar espacio.",
    uninstall_warning_generic:
      "{platform} desinstalará este juego desde su propio cliente. Esta operación no se puede deshacer desde Hacker Mode.",
    uninstall_confirm_button: "Sí, desinstalar",
    uninstall_confirm_button_busy: "Desinstalando…",
    uninstall_cancel_button: "Cancelar",
    cloud_saves_panel_title: "Copias de seguridad de partidas",
    cloud_saves_panel_body:
      "Hacker Mode no tiene acceso a la nube de guardado real de ninguna plataforma (es una API privada de cada cliente) — en su lugar guarda sus propias copias de seguridad locales de la carpeta de guardado de cada juego en la ubicación que elijas aquí. Si indicas una carpeta sincronizada por Dropbox/Nextcloud/Syncthing, las copias llegarán realmente \"a la nube\" por esa vía.",
    cloud_saves_backup_dir_label: "Carpeta de copias de seguridad",
    sync_settings_panel_title: "Exportar/importar ajustes",
    sync_settings_panel_body: "Hacker Mode no tiene sincronización de ajustes en segundo plano — pero puedes exportar todos tus ajustes (etiquetas, rutas de guardado, mapeos de mando, claves API) a un archivo e importarlo en otro dispositivo.",
    sync_settings_export: "Exportar ajustes",
    sync_settings_import: "Importar ajustes",
    external_apps_panel_title: "Detectado fuera de Hacker Mode",
    external_apps_panel_body: "Aplicaciones añadidas manualmente en Heroic (sideload) y prefijos de Wine gestionados por Bottles — solo informativo, Hacker Mode no las gestiona (inícialas/instálalas desde el programa original).",
    advanced_launch_panel_title: "Avanzado: inicio",
    advanced_launch_panel_body: "Un comando en el que se envuelve CADA inicio de juego — p. ej. «gamemoderun» (Feral GameMode) o «prime-run» (cambio de GPU NVIDIA Optimus). Se aplica fuera de Gamescope, si también está activado.",
    custom_launch_prefix_label: "Prefijo de inicio",
    crash_detection_threshold_label: "Umbral de detección de fallos (s)",
    notifications_panel_title: "Notificaciones del sistema",
    notifications_panel_body: "Qué eventos envían una notificación del sistema (notify-send) — útil cuando Hacker Mode se ejecuta en segundo plano durante una partida y podrías no notar, por ejemplo, una instalación fallida.",
    notification_on_install: "Instalación/desinstalación completada",
    notification_on_game_exit: "El juego pudo no iniciarse correctamente (cierre rápido/error)",
    notification_on_backup_error: "Falló la copia de seguridad automática de la partida",
    compat_tool_panel_title_steam: "Versión de Proton",
    compat_tool_panel_title_lutris: "Versión de Wine",
    compat_tool_empty_steam: "No hay versiones personalizadas de Proton (p. ej. Proton-GE) en compatibilitytools.d — Hacker Mode solo muestra estas, las versiones oficiales de Proton no se pueden asignar con seguridad sin arriesgarse a adivinar mal el nombre interno.",
    compat_tool_empty_lutris: "Lutris aún no tiene ninguna versión propia de Wine instalada mediante el gestor de runners.",
    achievements_loading: "Cargando logros…",
    achievements_title: "Logros",
    tag_input_placeholder: "+ etiqueta",
    bulk_select_mode: "Seleccionar varios",
    bulk_cancel_select_mode: "Cancelar selección",
    bulk_search_placeholder: "Buscar en la biblioteca…",
    bulk_sort_name: "Nombre A-Z",
    bulk_sort_playtime: "Más horas",
    bulk_sort_installed: "Instalados primero",
    bulk_selected_count: "Seleccionados",
    bulk_install_selected: "Instalar seleccionados",
    bulk_uninstall_selected: "Desinstalar seleccionados",
    bulk_cancel: "Cancelar",
    bulk_confirm_uninstall: "¿Desinstalar {count} juegos? Esto no se puede deshacer.",
    bulk_confirm_install: "¿Instalar {count} juegos?",
    bulk_confirm_yes: "Sí",
    bulk_processing: "Procesando…",
    bulk_completed: "Completado",
    bulk_retry_failed: "Reintentar fallidos",
    bulk_close: "Cerrar",
  },
};

export type TranslationKey = keyof Translation;

export function getText(
  lang: Lang,
  key: TranslationKey,
  vars?: Record<string, string | number>,
): string {
  let text: string = translations[lang]?.[key] ?? translations.en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      text = text.replace(`{${k}}`, String(v));
    }
  }
  return text;
}

export function detectLang(): Lang {
  const nav = typeof navigator !== "undefined" ? navigator.language : "en";
  const known = LANGUAGES.map((l) => l.id);
  const short = (nav?.split("-")[0] ?? "en") as Lang;
  return known.includes(short) ? short : "en";
}
