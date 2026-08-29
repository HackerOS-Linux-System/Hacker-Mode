# Hacker Mode

Sesja do gier dla HackerOS — odpowiednik Steam Big Picture / Gamescope
Session, ale jako **jedna binarka** obsługująca Steam, Epic Games, GOG,
Amazon Games i Lutris bez konieczności ręcznego wrapperowania osobnych
aplikacji.

> **Status projektu: przebudowa w toku (v0.3).** Wersja 0.1 (Electron)
> została zastąpiona nowym stosem: TypeScript + Solid.js (frontend) + Rust +
> Tauri (backend). Zobacz [Status komponentów](#status-komponentów) —
> frontend jest w pełni zweryfikowany (buduje się bez błędów), logika
> backendu jest kompletna.
>
> **Zmiana architektoniczna (v0.3):** Hacker Mode **nie ma już własnego
> kompozytora Wayland**. Dawny `compositor/` (Rust + Smithay, pisany od
> zera specjalnie dla tego projektu) został usunięty. Sesję Wayland
> zapewnia teraz **`comphwde`** — kompozytor z projektu
> [HWDE](https://github.com/HackerOS-Linux-System/HWDE), uruchamiany jako
> `comphwde --extern-hacker-mode`, dokładnie tak samo jak SDE (druga
> powłoka HackerOS) go używa (`comphwde --extern-sde`). Powód: `comphwde`
> jest już dojrzałym, współdzielonym kompozytorem używanym przez inne
> powłoki HackerOS, więc Hacker Mode nie musi utrzymywać i naprawiać
> własnej, równoległej implementacji Wayland/XWayland dla dokładnie tego
> samego zadania.

## Struktura repozytorium

```
source-code/backend/     # Backend Tauri (Rust) — logika gier, sklepów, ustawień
source-code/frontend/    # Frontend Solid.js + TypeScript — UI powłoki
ipc/                       # Własny, samodzielny klient protokołu IPC do comphwde
                            # + binarka hacker-mode-ipcctl (patrz niżej)
scripts/                   # Skrypt sesji, plik .desktop, walidator
docs/                       # Dodatkowa dokumentacja (testowanie sesji itd.)
.github/workflows/         # CI (GitHub Actions)
Makefile
Cargo.toml                 # Workspace (ipc, backend/src-tauri)
```

Cały ten workspace (`make all`) buduje się **bez żadnej zależności
źródłowej od HWDE** — `ipc/` to własna, niezależna implementacja klienta
protokołu, którym mówi `comphwde` (patrz jej moduł docs), nie zależność
git/path do repozytorium HWDE. Jedyna zależność od HWDE jest **w czasie
działania**: sama binarka `comphwde` musi być zainstalowana osobno, żeby
uruchomić realną sesję — patrz [Zależność: comphwde](#zależność-comphwde)
poniżej.

## Zależność: comphwde

Hacker Mode wymaga, żeby w `PATH` była zainstalowana binarka **`comphwde`**
z osobnego projektu [HWDE](https://github.com/HackerOS-Linux-System/HWDE)
— `scripts/hacker-mode-session` uruchamia ją jako
`comphwde --extern-hacker-mode`. To jedyna zewnętrzna zależność: **budowa
Hacker Mode samego siebie (`make all`) nigdy nie wymaga checkoutu ani
kompilacji HWDE** — potrzebne jest to dopiero, żeby faktycznie
*uruchomić* sesję.

```bash
make check-comphwde   # sprawdza, czy binarka comphwde jest w PATH
```

`hacker-mode-ipcctl` (mała binarka CLI, której `hacker-mode-session` używa
do poproszenia comphwde o odpalenie powłoki jako jego klienta Wayland —
żeby dostała poprawny `WAYLAND_DISPLAY`, patrz komentarz w tym skrypcie)
oraz biblioteka `hacker-mode-ipc`, przez którą backend Tauri
(`source-code/backend/src-tauri`) łączy się z comphwde, są częścią **tego**
repozytorium (`ipc/`) — obie budują się i instalują razem z resztą
(`make all` / `make install`), bez potrzeby sięgania do HWDE. `ipc/`
celowo zawiera własną, zwielokrotnioną kopię protokołu, którym mówi
`comphwde` w trybie `--extern-<nazwa>` (ten sam, którym mówi też SDE) —
patrz moduł docs w `ipc/src/lib.rs` po pełne uzasadnienie tej decyzji.

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
Proton/Wine), nie tylko natywnych klientów Wayland. Od v0.3 realizowane
jest to przez wywołania IPC do `comphwde` (`hacker-mode-ipc::enter_wrapper`/
`exit_wrapper`/`maximize_next_new_window` — patrz ich komentarze w
repozytorium HWDE), a nie przez dedykowany tryb we własnym kompozytorze.

Nawigacja działa myszką/klawiaturą **i padem** — również wieloma padami
jednocześnie (D-pad/lewy analog do poruszania się po siatce gier, A do
zatwierdzenia, B do cofnięcia, zderzaki do przełączania zakładek).

## Komendy binarki

```bash
hacker-mode         # Normalny start — zakłada sesję z menedżera logowania/TTY
hacker-mode dev      # Tryb deweloperski: okno zamiast pełnego ekranu,
                     # bez integracji z comphwde i akcjami zasilania
```

## Status komponentów

| Komponent | Status |
|---|---|
| `ipc/` | ✅ Kompletne, testy jednostkowe przechodzą; własna, samodzielna implementacja klienta protokołu comphwde + binarka `hacker-mode-ipcctl` (patrz [Zależność: comphwde](#zależność-comphwde)) |
| `source-code/frontend/` | ✅ Od v0.9.1 faktycznie zweryfikowane (`npm install && npx tsc --noEmit && npm run build` przechodzą bez błędów — patrz [CHANGELOG](CHANGELOG.md#091) po historię błędu, który to ujawnił); nawigacja wielopadowa, ekran logowania do sklepów, szczegóły gry z opisem/zrzutami (Steam/GOG/Epic), pasek postępu instalacji, natywne wyszukiwanie w katalogu Steam/GOG, statystyki biblioteki, operacje zbiorcze |
| `source-code/backend/src-tauri/` | ✅ Logika sklepów/launchera/ustawień/systemu/logowania/instalacji/dezinstalacji/okładek/czasu gry napisana w pełni, w tym v0.5–v0.10: status "w trakcie gry" + zatrzymywanie uruchomionej gry (PID-tracking, `stop_game`), poprawka uruchamiania Steam (`-applaunch` zamiast `steam://rungameid/`), globalny prefiks uruchamiania, granularne powiadomienia systemowe; ⚠️ błędy kompilacji z realnych buildów użytkownika naprawiane na bieżąco i weryfikowane w izolacji (v0.9.2, v0.10.0 — patrz CHANGELOG) — pełny `cargo check --workspace` wciąż niemożliwy w środowisku weryfikacyjnym z powodu zbyt starego `rustc` (patrz [Dlaczego kod Rust nie został (w pełni) zweryfikowany kompilatorem](#dlaczego-kod-rust-nie-został-w-pełni-zweryfikowany-kompilatorem)), na nowszym Rust u użytkownika powinien przejść normalnie |
| `source-code/backend/src-tauri/icons/` | ✅ Realne pliki PNG (placeholder graficzny, do podmiany na docelowe logo) |
| Kompozytor (`comphwde --extern-hacker-mode`) | Żyje teraz w osobnym repozytorium ([HWDE](https://github.com/HackerOS-Linux-System/HWDE)) — patrz [Zależność: comphwde](#zależność-comphwde). Status/roadmapa tego kompozytora (w tym DRM/KMS na gołym TTY) jest opisana tam, nie tutaj. |
| `.github/workflows/ci.yml` | ✅ Zaktualizowane pod v0.3: `rust-ipc` (zawsze musi przechodzić) sprawdza `ipc/`, `rust-backend` (informacyjny) sprawdza `source-code/backend/src-tauri` |
| `scripts/hacker-mode-session` + `.desktop` | ✅ Przepisane pod `comphwde --extern-hacker-mode` (v0.3), zwalidowane składniowo; ⚠️ nieprzetestowane na żadnym menedżerze logowania — patrz `docs/TESTING_SESSION.md` |

### Zakres wsparcia platform

Rdzeń (Steam) ma dziś najpełniejsze wsparcie — reszta platform to różne
podzbiory, ograniczone tym, co ich CLI/API w ogóle udostępnia:

| Platforma | Instalacja | Deinstalacja | Okładki | Czas gry | Opis/zrzuty ekranu | Posiadane-niezainstalowane |
|---|---|---|---|---|---|---|
| Steam | ✅ | ✅ | ✅ (lokalny cache) | ✅ (Web API) | ✅ (Store API) | ✅ (Web API) |
| Epic | ✅ (`legendary`) | ✅ (`legendary`) | ✅ (cache `legendary`) | ⚠️ lokalny licznik Hacker Mode | ✅ (lokalny cache `legendary`, offline) | ✅ (`legendary list`) |
| GOG | ✅ (`gogdl`) | ✅ (`gogdl`) | ✅ (`api.gog.com`) | ⚠️ lokalny licznik Hacker Mode | ✅ (`api.gog.com`) | ✅ (`embed.gog.com`, token z `auth.json`) |
| Amazon | ✅ (`nile`) | ✅ (`nile`) | ✅ (heurystyka z JSON `nile`) | ⚠️ lokalny licznik Hacker Mode | ❌ (brak publicznego API katalogu) | ✅ (`nile library list`) |
| Lutris | ✅ (`lutris:` URI) | ✅ (oznaczenie w `pga.db`, bez kasowania plików) | ✅ (heurystyka lokalna) | ✅ (kolumna `playtime` w `pga.db` Lutrisa) | ❌ (publiczne API Lutrisa nie zwraca opisu/zrzutów) | ❌ (Lutris to menedżer LOKALNYCH instalacji — nie ma pojęcia "posiadane, ale niezainstalowane") |
| EA app | ⚠️ otwiera klienta (brak prywatnego API) | ✅ (usunięcie katalogu instalacji) | ✅ (opcjonalnie, SteamGridDB po tytule) | ⚠️ lokalny licznik Hacker Mode | ❌ (brak publicznego API katalogu) | ❌ (heurystyka skanuje tylko już zainstalowane) |
| Battle.net | ⚠️ otwiera klienta (brak prywatnego API) | ✅ (usunięcie katalogu instalacji) | ✅ (opcjonalnie, SteamGridDB po tytule) | ⚠️ lokalny licznik Hacker Mode | ❌ (brak publicznego API katalogu) | ❌ (heurystyka skanuje tylko już zainstalowane) |

„⚠️ lokalny licznik Hacker Mode" (`playtime.rs`) oznacza czas mierzony
WYŁĄCZNIE dla gier uruchomionych przez sam Hacker Mode — sesje spoza niego
(np. gra odpalona bezpośrednio z klienta Epic otwartego osobno) nie są
widoczne. To jedyne dostępne źródło dla platform bez własnego API czasu
gry; nie zastępuje go, tylko uzupełnia braki.

### Historia: dawny własny kompozytor (`compositor/`, do v0.2)

Do wersji 0.2 Hacker Mode miał własny, pisany od zera kompozytor Wayland
(Rust + Smithay) w `compositor/`, z własnym protokołem IPC w ówczesnym
`ipc/` (protokół w wersji 0.2, całkowicie inny niż obecny). `compositor/`
został usunięty w v0.3 na rzecz `comphwde --extern-hacker-mode` (patrz
wyżej); `ipc/` w tym repozytorium istnieje nadal, ale od v0.3 to zupełnie
inna zawartość — samodzielny klient nowego protokołu, którym mówi
`comphwde` (patrz [Zależność: comphwde](#zależność-comphwde)), a nie
protokół do usuniętego własnego kompozytora. Jeśli szukasz historii
tamtego podejścia (w tym opisu błędu "`smithay` nie ma na crates.io
wydania z modułem `desktop`" i jego obejścia), znajdziesz ją w historii
Gita tego pliku sprzed v0.3.

### Dlaczego kod Rust nie został (w pełni) zweryfikowany kompilatorem

Aktualizacja v0.9.2: **`rustc`/`cargo` w wersji 1.75.0 (Ubuntu 24.04 `apt`)
JEST dostępny** w środowisku, w którym rozbudowywany jest ten projekt —
wcześniejsze wersje tego README błędnie twierdziły, że nie da się go
w ogóle zainstalować; w rzeczywistości `apt-get install rustc cargo`
działa (`static.rust-lang.org`/`sh.rustup.rs`, potrzebne do zainstalowania
NOWSZEGO Rusta przez `rustup`, wciąż są zablokowane sieciowo).

Problem: `rustc 1.75.0` (grudzień 2023) jest zbyt stary, by uruchomić
**pełny** `cargo check --workspace` na tym projekcie w 2026 roku — nie z
powodu błędu w kodzie Hacker Mode, tylko dlatego, że współczesne wersje
NAWET pośrednich, niepowiązanych zależności (`mail-parser` przez
`ribbit-client` w `bnet`, `home` przez `which` w `ea-cli`) wymagają dziś
`edition = "2024"` w swoim `Cargo.toml`, co wymaga Cargo ≥ 1.85 (luty
2025) — starszy Cargo nie potrafi nawet SPARSOWAĆ ich manifestu, więc
rozwiązywanie zależności wywala się, zanim dojdzie do kompilacji
jakiegokolwiek kodu z tego repozytorium. Potwierdzone tym, że nawet
`cargo check -p ea-cli` (dwie proste zależności) trafia w ten sam
problem.

Co to oznacza praktycznie: pełny `cargo check --workspace` nadal nie był
możliwy w tym konkretnym środowisku, ALE pojedyncze, zgłoszone błędy
kompilacji SĄ weryfikowalne — przez odtworzenie ich logiki w izolowanym,
minimalnym projekcie (bez zależności workspace'u, więc bez problemu
edition2024) i realną kompilację/uruchomienie tym samym `rustc`. Tak
naprawiono i zweryfikowano 4 błędy zgłoszone w v0.9.2 (patrz CHANGELOG).
To słabszy standard weryfikacji niż pełny `cargo check` — potwierdza
poprawność WYIZOLOWANEJ logiki zmiany, nie całego programu naraz — ale
znacząco lepszy niż wcześniejsze "napisane, nigdy nie uruchomione".

Frontend (Node/npm) BYŁ dostępny przez cały czas, ale przez wersje do
v0.9.0 **nie był faktycznie uruchamiany** — kod TypeScript był pisany
"na oko", tym samym trybem co backend Rust, mimo że nic nie stało na
przeszkodzie, żeby to zweryfikować. Efekt: w v0.9.0 trzy niemieckie
tłumaczenia w `i18n/index.ts` miały źle domknięty cudzysłów wewnątrz
literału stringu, co powodowało **296 kaskadowych błędów kompilacji** —
zgłoszone przez użytkownika dopiero po realnym `make frontend`, nie
wyłapane wcześniej właśnie dlatego, że nikt (łącznie ze mną) nie
uruchomił kompilatora. Naprawione w v0.9.1, **od tej wersji frontend
faktycznie jest weryfikowany** — `npm install`, `npx tsc --noEmit` i
`npm run build` (`tsc -b && vite build`) zostały uruchomione i przechodzą
bez błędów.

**Jeśli budujesz to na swojej maszynie z nowszym Rustem (≥1.85, co
najprawdopodobniej masz — patrz wyżej), pełny `cargo check --workspace`
powinien zadziałać normalnie** i wyłapie ewentualne dalsze błędy, których
ograniczenia tego środowiska weryfikacyjnego nie pozwoliły złapać.

**Przed pierwszym realnym buildem wykonaj:**

```bash
rustup default stable
sudo pacman -S webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg
cargo check --workspace
```

(Pakiety typu `wayland`/`libinput`/`mesa`/`seatd` nie są już tu potrzebne od
v0.3 — to zależności kompozytora, który teraz żyje w osobnym repozytorium
HWDE, patrz [Zależność: comphwde](#zależność-comphwde); jeśli budujesz też
`comphwde`, zainstaluj je tam, zgodnie z jego własnym README.)

i popraw ewentualne rozjazdy nazw metod/pól względem konkretnych wersji
`tauri`/`hacker-mode-ipc` przypiętych w `Cargo.lock` — CI (o ile
zaktualizowane, patrz [Do zrobienia dalej](#do-zrobienia-dalej)) powinno
robić to samo automatycznie przy każdym PR i pokazywać dokładne błędy.

## Budowanie

```bash
make check           # cargo check (workspace) + tsc --noEmit
make dev              # uruchamia samą powłokę Tauri w oknie (bez comphwde)
make all              # buduje frontend + backend (release)
make check-comphwde   # sprawdza, czy binarka comphwde jest w PATH
sudo make install
scripts/validate-desktop-entry.sh   # walidacja pliku sesji
```

`make all` buduje `hacker-mode` **i** `hacker-mode-ipcctl` (oba z tego
repozytorium, `make install` instaluje oba) — **nie** buduje ani nie
instaluje samego `comphwde`, to osobny projekt, patrz
[Zależność: comphwde](#zależność-comphwde).

## Do zrobienia dalej

Rzeczy warte zrobienia po tej przebudowie (v0.3), spoza zakresu samej
migracji z własnego kompozytora na `comphwde`:

- **Status "w trakcie gry" dla Steam (v0.10) pozostaje ograniczony z
  przyczyn architektonicznych, nie do naprawienia samym Hacker Mode.**
  `steam -applaunch` (nawet po poprawce z v0.10) i tak przekazuje
  faktyczne uruchomienie gry działającej instancji Steam przez IPC, więc
  proces, który Hacker Mode obserwuje, kończy się dużo wcześniej niż
  realna sesja gry. Jedyny sposób na prawdziwe śledzenie stanu gry Steam
  wymagałby integracji z Steam Web API (`GetPlayerSummaries` pokazuje
  `gameid`/`gameextrainfo` gracza) albo nasłuchiwania lokalnego IPC
  samego klienta Steam — obie opcje to osobne, większe zadania.
- **Heurystyka wykrycia crasha (próg 8 sekund) jest zduplikowana** w
  trzech miejscach z tą samą stałą wpisaną na sztywno (`launcher.rs`,
  `GameCard.tsx`, `GameDetail.tsx`) zamiast jednego, konfigurowalnego
  źródła prawdy — warto to ujednolicić, najlepiej jako kolejne ustawienie
  w panelu "Zaawansowane: uruchamianie".
- **Backup przy zapisie `config.vdf`/YAML Lutrisa (`backup_before_write`)
  chroni tylko przed BŁĘDEM PATCHA Hacker Mode, nie przed współbieżnym
  zapisem przez sam Steam/Lutris.** `is_process_running` (v0.9) tylko
  OSTRZEGA, gdy dany klient działa podczas zapisu — nie ma tu prawdziwej
  blokady plikowej międzyprocesowej (Hacker Mode nie może wymusić, żeby
  Steam/Lutris respektowały jakikolwiek zamek pliku, którego same nie
  próbują sprawdzać). To fundamentalne ograniczenie tego podejścia
  (chirurgiczna podmiana tekstu zamiast API), nie coś, co da się naprawić
  bez współpracy po stronie Steam/Lutris.
- **Kształt `sideload_apps/library.json` Heroic (`external_sources.rs`,
  v0.9) nie jest w pełni zweryfikowany** — parser próbuje dwóch
  prawdopodobnych wariantów (tablica najwyższego poziomu vs obiekt z
  kluczem `"games"`), ale nigdy nie został sprawdzony na prawdziwym pliku.
  Podobnie schemat `bottle.yml` Bottles jest całkowicie nieznany — dlatego
  wykrywanie bottle'i ogranicza się do samej nazwy katalogu, bez próby
  odczytania z niego listy zainstalowanych programów.
- **Stan weryfikacji formatów plików innych programów (v0.7.1).**
  `compat_tools.rs` opiera się na dwóch formatach plików spoza kontroli
  Hacker Mode: `CompatToolMapping` w Steam `config.vdf` i sekcja
  `wine:`/`version:` w YAML Lutrisa. Oba zostały potwierdzone researchem
  (wiele niezależnych, zgodnych ze sobą źródeł — zgłoszenia błędów
  `lutris/lutris`, `steamtinkerlaunch`, wątki społeczności Steam Deck —
  patrz cytaty w komentarzach modułu), więc pewność co do nich jest
  wyraźnie wyższa niż reszty poniższych punktów. Mimo to ŻADEN z nich nie
  został przetestowany przez faktyczny zapis na żywej instalacji Steam/
  Lutrisa — priorytetowy kandydat do ręcznej weryfikacji przed użyciem na
  własnej bibliotece, mimo poprawki błędu w 0.7.1 (patrz CHANGELOG).
- **`comphwde` dziś nie ma backendu DRM/KMS podpiętego pod `main()`**
  (`drm-experimental` istnieje, ale nie jest domyślne/gotowe) — czyli
  uruchomienie sesji Hacker Mode z gołego TTY (bez X11/Wayland pod spodem)
  najprawdopodobniej **wciąż nie zadziała**, dopóki ta praca nie zostanie
  dokończona po stronie HWDE. Warto to jawnie przetestować i, jeśli
  potwierdzone, opisać jako znane ograniczenie w `docs/TESTING_SESSION.md`.
- **`maximize_next_new_window`** w `hacker-mode-ipc` dopasowuje "pierwsze
  nowe okno, które nie jest powłoką" — przy jednoczesnym uruchomieniu kilku
  procesów (np. dwóch gier pod rząd, zanim pierwsza zdąży zmapować okno)
  mogłoby to zmaksymalizować niewłaściwe okno. Dziś nie jest to problem
  (uruchamianie jest debounce'owane przez `LAUNCH_COOLDOWN`), ale warto
  o tym pamiętać przy ewentualnym luzowaniu tego limitu.
- **Ikony powłoki** (`source-code/backend/src-tauri/icons/`) są nadal
  placeholderem — do podmiany na docelowe logo przed wydaniem.
- **`ipc/` duplikuje protokół comphwde zamiast go współdzielić** (celowo,
  patrz jej moduł docs) — jeśli protokół `--extern-<nazwa>` po stronie
  HWDE kiedyś się zmieni, tę kopię trzeba będzie ręcznie zaktualizować;
  `wire_shape_matches_comphwde_expectations` w `ipc/src/lib.rs` to
  minimalny test tego kształtu, ale nie zastąpi testu integracyjnego
  przeciw realnej binarce `comphwde`.
- Realny `cargo check --workspace` na maszynie z zainstalowanym Rustem —
  ten kod nigdy nie przeszedł przez faktyczny kompilator (patrz sekcja
  niżej), więc drobne niezgodności typów/nazw metod względem wersji
  `tauri`/`hacker-mode-ipc` przypiętych w `Cargo.lock` są prawdopodobne.
- **Deinstalacja i czas gry Lutrisa (v0.5) wymagają CLI `sqlite3`** w
  `PATH` (odczyt/zapis bazy `~/.local/share/lutris/pga.db`) — bez niego
  te dwie funkcje po prostu nic nie robią (reszta Lutrisa działa normalnie).
  Warto dodać `sqlite3` do listy zależności runtime w dokumentacji
  instalacyjnej, obok `legendary`/`gogdl`/`nile`.
- **Wyszukiwanie katalogu GOG (`catalog.gog.com`) i szczegóły GOG/Epic
  (v0.5) nie zostały nigdy wywołane na żywo** — środowisko generujące ten
  kod nie miało dostępu sieciowego do tych domen, więc dokładny kształt
  odpowiedzi JSON (nazwy pól typu `coverVertical`/`slug` w katalogu GOG,
  `embed.gog.com/user/data/games`) jest oparty na dokumentacji
  społecznościowej (GOGDB, `gogapidocs`, kod otwartoźródłowych klientów
  GOG), nie na zweryfikowanym wywołaniu — do potwierdzenia przy pierwszym
  realnym uruchomieniu.

## Testowanie sesji na SDDM/GDM/LightDM

Zobacz `docs/TESTING_SESSION.md` — checklista kroków do ręcznego wykonania
na docelowym systemie (nie da się tego zautomatyzować bez aktywnego
menedżera logowania).

## Licencja

GPL-3.0.
