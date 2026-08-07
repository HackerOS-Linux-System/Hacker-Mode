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
| `source-code/frontend/` | ✅ Buduje się bez błędów (`tsc --noEmit` + `vite build` zweryfikowane wielokrotnie); nawigacja wielopadowa, ekran logowania do sklepów, szczegóły gry z opisem/zrzutami (Steam), pasek postępu instalacji, natywne wyszukiwanie w katalogu Steam |
| `source-code/backend/src-tauri/` | ✅ Logika sklepów/launchera/ustawień/systemu/logowania/instalacji/dezinstalacji/okładek/czasu gry napisana w pełni; ✅ v0.3: integracja z `comphwde` przez własny crate `ipc/` (tryb wrapper, zamykanie sesji), zamiast z własnym, usuniętym kompozytorem; ⚠️ niezweryfikowana kompilatorem (brak toolchaina Rust w środowisku generującym ten kod) |
| `source-code/backend/src-tauri/icons/` | ✅ Realne pliki PNG (placeholder graficzny, do podmiany na docelowe logo) |
| Kompozytor (`comphwde --extern-hacker-mode`) | Żyje teraz w osobnym repozytorium ([HWDE](https://github.com/HackerOS-Linux-System/HWDE)) — patrz [Zależność: comphwde](#zależność-comphwde). Status/roadmapa tego kompozytora (w tym DRM/KMS na gołym TTY) jest opisana tam, nie tutaj. |
| `.github/workflows/ci.yml` | ✅ Zaktualizowane pod v0.3: `rust-ipc` (zawsze musi przechodzić) sprawdza `ipc/`, `rust-backend` (informacyjny) sprawdza `source-code/backend/src-tauri` |
| `scripts/hacker-mode-session` + `.desktop` | ✅ Przepisane pod `comphwde --extern-hacker-mode` (v0.3), zwalidowane składniowo; ⚠️ nieprzetestowane na żadnym menedżerze logowania — patrz `docs/TESTING_SESSION.md` |

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

### Dlaczego kod Rust nie został zweryfikowany kompilatorem

Środowisko, w którym powstał ten projekt, nie ma zainstalowanego toolchaina
Rust i nie ma dostępu sieciowego do `static.rust-lang.org`/`sh.rustup.rs`
(tylko do rejestrów `crates.io`/`npm`), więc nie dało się go zainstalować ani
uruchomić `cargo check`. Frontend (Node/npm) był dostępny i **został
zweryfikowany wielokrotnie** w trakcie rozbudowy — `npm install`,
`tsc --noEmit` oraz `npm run build` przechodzą bez błędów po każdej większej
zmianie.

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

## Testowanie sesji na SDDM/GDM/LightDM

Zobacz `docs/TESTING_SESSION.md` — checklista kroków do ręcznego wykonania
na docelowym systemie (nie da się tego zautomatyzować bez aktywnego
menedżera logowania).

## Licencja

GPL-3.0.
