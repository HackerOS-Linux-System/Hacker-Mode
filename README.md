# Hacker Mode

Sesja do gier dla HackerOS — odpowiednik Steam Big Picture / Gamescope
Session, ale jako **jedna binarka** obsługująca Steam, Epic Games, GOG,
Amazon Games, Lutris, EA app i Battle.net bez ręcznego wrapperowania
osobnych aplikacji.

> **Wersja 1.1.0.** Stos: TypeScript + Solid.js (frontend) + Rust + Tauri
> (backend). Domyślnym kompozytorem sesji jest teraz **`labwc`** (dojrzały
> kompozytor wlroots z natywnym backendem DRM/KMS — sesja działa także z
> gołego TTY). `comphwde` (projekt HWDE) pozostaje w pełni obsługiwany jako
> opcja: `"compositor": "comphwde"` w `~/.config/Hacker-Mode/config.json`.
> Szczegóły zmian: [CHANGELOG](CHANGELOG.md).

## Struktura repozytorium

```
source-code/backend/     # Backend Tauri (Rust) — logika gier, sklepów, ustawień
source-code/frontend/    # Frontend Solid.js + TypeScript — UI powłoki
ipc/                       # Klient protokołu IPC do comphwde + hacker-mode-ipcctl
launchers-cli/             # Pomocnicze CLI: ea-cli, bnet
scripts/                   # Skrypt sesji, labwc/rc.xml, .desktop, testy sesji
packaging/arch/            # PKGBUILD (niezweryfikowany w czystym chroocie)
docs/                      # TESTING_SESSION.md, VERIFICATION.md
.github/workflows/         # CI (frontend, backend, sesja labwc headless)
Makefile, build.hl         # Te same targety w dwóch zapisach
Cargo.toml                 # Workspace
```

## Kompozytor sesji

Kompozytor wybiera się w `~/.config/Hacker-Mode/config.json` (ten sam plik,
w którym Hacker Mode trzyma resztę ustawień; tworzony przy pierwszym
starcie sesji z uprawnieniami `0600`):

```json
{ "compositor": "labwc" }
```

| Wartość | Opis |
|---|---|
| `labwc` (domyślna) | `labwc -C ~/.config/Hacker-Mode/labwc -s <klient>`; klient uruchamia `hacker-mode` i po jego wyjściu woła `labwc --exit`. Okna: bez ramek, zmaksymalizowane regułą z `rc.xml`. Wymaga pakietu `labwc` (+ `Xwayland` dla gier X11/Wine). |
| `comphwde` | `comphwde --extern-hacker-mode`, powłoka startowana przez IPC (`hacker-mode-ipcctl launch`). Wymaga osobnej instalacji `comphwde` z projektu [HWDE](https://github.com/HackerOS-Linux-System/HWDE). |

Kolejność rozstrzygania: zmienna `HACKER_MODE_COMPOSITOR` → `config.json` →
stary `~/.config/hacker-mode/config.json` (migrowany automatycznie) →
`labwc`. Zmiana działa po ponownym zalogowaniu (Ustawienia pokazują też,
który kompozytor działa teraz). Nieznana wartość = `labwc` z ostrzeżeniem.

`~/.config/Hacker-Mode/labwc/rc.xml` jest nadpisywany szablonem przy
każdym starcie sesji, **dopóki zawiera znacznik** `hacker-mode:generated`;
usuń znacznik, żeby zachować własne zmiany.

`make check-labwc` / `make check-comphwde` sprawdzają obecność kompozytora,
a `hacker-mode doctor` robi pełną diagnostykę (patrz niżej).

## Filozofia produktu

Zamiast (jak w wersji 0.1) *wrapperować* już istniejące aplikacje sklepowe,
Hacker Mode **sam** zarządza bibliotekami gier — Steam (bezpośredni odczyt
plików + okładki z lokalnego cache + opcjonalny czas gry przez Steam Web
API), Epic (`legendary`, z okładkami z lokalnego cache metadanych), GOG
(`gogdl`, z okładkami z publicznego `api.gog.com`), Amazon Games (`nile`),
Lutris (natywne JSON CLI). Wspierane jest **uruchamianie, instalowanie
(z paskiem postępu, gdy narzędzie raportuje procent) i odinstalowywanie**
gier, a logowanie do sklepów odbywa się w osadzonym oknie webview — dla GOG
kod autoryzacyjny jest przechwytywany automatycznie z URL-a przekierowania,
dla Amazon proces logowania jest w pełni doczekiwany w tle, dla Epic jest
pole do wklejenia kodu jako fallback. Katalog Steam można też przeszukiwać
natywnie (przez publiczne API, nie stronę WWW) wprost z zakładki „Sklepy”.

Tryb **wrapper** z wersji 0.1 został zachowany jako opcjonalny przełącznik:
uruchomienie gry/launchera chowa powłokę i automatycznie przełącza nowe okno
na pełny ekran — dotyczy to teraz również okien XWayland (gry spod
Proton/Wine), nie tylko natywnych klientów Wayland. Realizacja zależy od
kompozytora (patrz [Kompozytor sesji](#kompozytor-sesji)): w **labwc**
robią to statyczne reguły okien w `rc.xml` (każde okno bez ramki i
zmaksymalizowane w chwili otwarcia — bez wyścigów); w **comphwde** —
wywołania IPC (`hacker-mode-ipc::enter_wrapper`/`exit_wrapper`/
`maximize_next_new_window`).

Nawigacja działa myszką/klawiaturą **i padem** — również wieloma padami
jednocześnie (D-pad/lewy analog do poruszania się po siatce gier, A do
zatwierdzenia, B do cofnięcia, zderzaki do przełączania zakładek).

## Komendy binarki

## Komendy binarki

```bash
hacker-mode          # Normalny start — zakłada sesję z menedżera logowania/TTY
hacker-mode dev      # Tryb deweloperski: okno zamiast pełnego ekranu,
                     # bez integracji z kompozytorem i akcjami zasilania
hacker-mode doctor   # Diagnostyka (tylko odczyt): zależności, kompozytor oraz
                     # zgodność parserów z PRAWDZIWYMI plikami Steam/Lutris/
                     # Heroic/Bottles na tej maszynie. Kod wyjścia 1 = problem.
```

## Status komponentów

Poziomy pewności są rozdzielone świadomie — „przeszło w CI/na tej
maszynie" to co innego niż „sprawdzone na prawdziwym systemie":

| Komponent | Status |
|---|---|
| `ipc/` | ✅ Testy jednostkowe + testy integracyjne z **mockiem** serwera comphwde. ⚠️ Zgodność z prawdziwą binarką `comphwde` niepotwierdzona (kod HWDE jest niedostępny); klient jest tolerancyjny na kształt odpowiedzi, a `cargo test -p hacker-mode-ipc -- --ignored` z `COMPHWDE_BIN` sprawdza ją na żywo. |
| `source-code/frontend/` | ✅ `tsc`, `vite build`, parytet i18n (7 języków × 138 kluczy) i testy vitest przechodzą. ⚠️ Zachowanie w działającym oknie Tauri sprawdzane ręcznie. |
| `source-code/backend/src-tauri/` | ✅ `cargo check --workspace --all-targets` i `cargo test --workspace` przechodzą (Rust 1.91). ⚠️ Integracja z prawdziwymi klientami (Steam/Lutris/GOG/Epic/Amazon) wymaga weryfikacji na żywych instalacjach — `hacker-mode doctor` + [docs/VERIFICATION.md](docs/VERIFICATION.md). |
| Sesja labwc (`scripts/hacker-mode-session`) | ✅ Cykl życia sprawdzony z prawdziwym labwc 0.7.1 w trybie headless (`scripts/test-session-headless.sh`, także w CI). ⚠️ Nie testowano: menedżer logowania (SDDM/GDM/LightDM) i goły TTY na fizycznym GPU — checklista w [docs/TESTING_SESSION.md](docs/TESTING_SESSION.md). |
| Sesja comphwde | ⚠️ Logika skryptu bez zmian względem v0.3; nietestowana (brak binarki comphwde). Backend DRM/KMS w comphwde to sprawa projektu HWDE. |

### Zakres wsparcia platform

Rdzeń (Steam) ma dziś najpełniejsze wsparcie — reszta platform to różne
podzbiory, ograniczone tym, co ich CLI/API w ogóle udostępnia:

| Platforma | Instalacja | Deinstalacja | Okładki | Czas gry | Opis/zrzuty ekranu | Posiadane-niezainstalowane |
|---|---|---|---|---|---|---|
| Steam | ✅ | ✅ | ✅ (lokalny cache) | ✅ (Web API) | ✅ (Store API) | ✅ (Web API) |
| Epic | ✅ (`legendary`) | ✅ (`legendary`) | ✅ (cache `legendary`) | ⚠️ lokalny licznik Hacker Mode | ✅ (lokalny cache `legendary`, offline) | ✅ (`legendary list`) |
| GOG | ✅ (`gogdl`) | ✅ (`gogdl`) | ✅ (`api.gog.com`) | ⚠️ lokalny licznik Hacker Mode | ✅ (`api.gog.com`) | ✅ (`embed.gog.com`, token z `auth.json`) |
| Amazon | ✅ (`nile`) | ✅ (`nile`) | ✅ (heurystyka z JSON `nile`) | ⚠️ lokalny licznik Hacker Mode | ⚠️ (zapasowo publiczne Steam Store API, TYLKO przy dokładnie tym samym tytule) | ✅ (`nile library list`) |
| Lutris | ✅ (`lutris:` URI) | ✅ (oznaczenie w `pga.db`; kasowanie plików opcjonalne, domyślnie wyłączone) | ✅ (heurystyka lokalna) | ✅ (kolumna `playtime` w `pga.db` Lutrisa) | ❌ (publiczne API Lutrisa nie zwraca opisu/zrzutów) | ❌ (Lutris to menedżer LOKALNYCH instalacji — nie ma pojęcia "posiadane, ale niezainstalowane") |
| EA app | ⚠️ otwiera klienta (brak prywatnego API) | ✅ (usunięcie katalogu instalacji) | ✅ (opcjonalnie, SteamGridDB po tytule) | ⚠️ lokalny licznik Hacker Mode | ⚠️ (zapasowo Steam Store API po dokładnym tytule) | ❌ (brak jakiegokolwiek publicznego API biblioteki; skanowane są tylko zainstalowane) |
| Battle.net | ⚠️ otwiera klienta (brak prywatnego API) | ✅ (usunięcie katalogu instalacji) | ✅ (opcjonalnie, SteamGridDB po tytule) | ⚠️ lokalny licznik Hacker Mode | ⚠️ (zapasowo Steam Store API po dokładnym tytule) | ❌ (brak jakiegokolwiek publicznego API biblioteki; skanowane są tylko zainstalowane) |

„⚠️ lokalny licznik Hacker Mode" (`playtime.rs`) oznacza czas mierzony
WYŁĄCZNIE dla gier uruchomionych przez sam Hacker Mode — sesje spoza niego
(np. gra odpalona bezpośrednio z klienta Epic otwartego osobno) nie są
widoczne. To jedyne dostępne źródło dla platform bez własnego API czasu
gry; nie zastępuje go, tylko uzupełnia braki.


### Zależności zewnętrzne

Wymagane do działania powłoki i sesji: `labwc` (albo `comphwde`) i
`pgrep` (`procps`). Opcjonalne — brak wyłącza tylko daną funkcję:
`Xwayland`, `steam`, `lutris`, `sqlite3` (CLI; czas gry i odinstalowanie
w Lutrisie), `legendary`/`gogdl`/`nile` (instalowane w izolowanym venv
przy pierwszym uruchomieniu), `nmcli`, `bluetoothctl`, `wpctl`/`pactl`,
`brightnessctl`, `powerprofilesctl`. Ustawienia → „Zależności systemowe”
pokazują stan każdego z nich wraz z podpowiedzią instalacji.

### Bezpieczeństwo

- `config.json` (klucze API Steam/SteamGridDB) jest zapisywany atomowo z
  uprawnieniami `0600` w katalogu `0700`; uszkodzony plik trafia do
  `config.json.corrupt-<czas>` zamiast zostać po cichu nadpisany.
  (Klucze nie są w keyringu systemowym — to świadomy kompromis; plik jest
  prywatny dla użytkownika.)
- Uprawnienia Tauri zawężone do okna `shell`; usunięte nieużywane wtyczki
  `shell`/`os`; `fs` ograniczone do odczytu/zapisu tekstu w
  Pobranych/Dokumentach/Pulpicie (+ ścieżki wybrane w oknie dialogowym).
  CSP bez `unsafe-inline` dla skryptów/arkuszy i z listą dozwolonych hostów
  obrazów (zamiast `https:`).
- Zapis `config.vdf` Steama / YAML Lutrisa: domyślnie **odmowa**, gdy klient
  działa (opcja w Ustawieniach to zmienia); zapis atomowy, kontrola
  struktury VDF przed zapisem i weryfikacja po zapisie, kopia zapasowa.
- Kasowanie plików gry Lutrisa: domyślnie wyłączone; po włączeniu tylko
  pod `~/Games` i katalogami danych Lutrisa, nigdy symlinki ani sam root.
- Dane wejściowe Wi-Fi/Bluetooth są walidowane, a `nmcli` ma limit czasu.

## Budowanie

```bash
make check           # cargo check (workspace, z testami) + tsc + parytet i18n
make test            # testy Rust + frontend + i18n
make test-session    # cykl życia sesji z labwc w trybie headless
make dev             # sama powłoka Tauri w oknie (bez kompozytora)
make all             # frontend + backend (release)
sudo make install    # instaluje też labwc/rc.xml do /usr/share/hacker-mode/
make doctor          # hacker-mode doctor
scripts/validate-desktop-entry.sh   # walidacja pliku sesji
```

(Targety są też w `build.hl`.) Build backendu wymaga zbudowanego frontendu
(`source-code/frontend/dist`) i bibliotek systemowych Tauri v2
(`libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`, `libgtk-3-dev`, …).

## Do zrobienia dalej

Rzeczy, których NIE da się zamknąć bez prawdziwego sprzętu/kont/zasobów:

- **Test sesji na SDDM/GDM/LightDM i z gołego TTY na fizycznym GPU** —
  [docs/TESTING_SESSION.md](docs/TESTING_SESSION.md).
- **Weryfikacja formatów na żywych instalacjach** (`config.vdf`, YAML
  Lutrisa, `sideload_apps/library.json` Heroic, `bottle.yml` Bottles, API
  GOG/Epic): `hacker-mode doctor` i [docs/VERIFICATION.md](docs/VERIFICATION.md).
- **Zgodność `ipc/` z prawdziwą binarką `comphwde`** (kod HWDE niedostępny
  publicznie): uruchom ignorowany test z `COMPHWDE_BIN`.
- **EA app / Battle.net: lista posiadanych-niezainstalowanych** — nie ma
  publicznego API biblioteki tych platform.
- **Czas gry poza Steamem i Lutrisem** pozostaje licznikiem sesji Hacker
  Mode (brak lokalnego źródła w innych klientach).
- **Klucze API w keyringu systemowym** zamiast pliku `0600`.
- **Zachowanie okien labwc** (maksymalizacja, fokus po powrocie z gry)
  potwierdzone tylko w trybie headless na poziomie startu/ładowania
  konfiguracji; ocena wizualna wymaga prawdziwego ekranu.
- **Dostępność:** dodano role/etykiety `aria-*` w nowych komponentach i
  nawigacji; pełny audyt czytnikiem ekranu nie był wykonany.
- **Dalsze dzielenie dużych plików frontendu** (`GameDetail.tsx` ma teraz
  ~670 linii zamiast 930; kopie zapasowe i mapowanie kontrolera nadal tam
  mieszkają).

## Testowanie sesji na SDDM/GDM/LightDM

Zobacz [docs/TESTING_SESSION.md](docs/TESTING_SESSION.md).

## Licencja

GPL-3.0.
