import { Component, createEffect, createSignal, onMount, Show } from "solid-js";
import { Route, Router } from "@solidjs/router";
import NavBar from "@/components/NavBar";
import HackerMenu from "@/components/HackerMenu";
import Onboarding from "@/components/Onboarding";
import Home from "@/routes/Home";
import Library from "@/routes/Library";
import Store from "@/routes/Store";
import Settings from "@/routes/Settings";
import GameDetail from "@/routes/GameDetail";
import Stats from "@/routes/Stats";
import { settingsStore, useSettingsBootstrap } from "@/stores/settingsStore";
import { api } from "@/lib/tauri";
import { gamesStore } from "@/stores/gamesStore";
import { installNavigation } from "@/lib/focusNav";

const Layout: Component<{ children?: any }> = (props) => {
  const [menuOpen, setMenuOpen] = createSignal(false);
  useSettingsBootstrap();

  // BUGFIX: `Settings::theme` istniało w typie/backendzie od dawna, ale
  // nic nigdy go nie odczytywało po stronie frontendu — wybór motywu w
  // Ustawieniach nic nie zmieniał. `global.css` definiuje warianty przez
  // `[data-theme="..."]`; tu tylko ustawiamy ten atrybut na `<html>`
  // reaktywnie, za każdym razem gdy `settings().theme` się zmieni.
  createEffect(() => {
    document.documentElement.setAttribute("data-theme", settingsStore.settings().theme);
  });

  onMount(() => {
    installNavigation();

    // Gdy backend zgłosi zamknięcie uruchomionej gry/aplikacji (koniec
    // trybu wrapper), odświeżamy bibliotekę — np. zaktualizowany czas gry.
    void api.onAppClosed(() => {
      void gamesStore.refresh();
    });

    // Escape / przycisk B na padzie zamyka Hacker Menu, jeśli jest otwarte.
    window.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && menuOpen()) setMenuOpen(false);
    });
  });

  return (
    <div class="app-shell">
      <NavBar onOpenMenu={() => setMenuOpen(true)} />
      {props.children}
      {menuOpen() && <HackerMenu onClose={() => setMenuOpen(false)} />}
    </div>
  );
};

const App: Component = () => {
  // Ekran powitalny pierwszego uruchomienia (patrz `Onboarding.tsx` i
  // `first_run.rs`) — sprawdzany raz, przed zamontowaniem właściwego
  // routera/Layoutu, żeby nowy użytkownik od razu zobaczył powitanie
  // zamiast normalnego UI z pustą (bo jeszcze niczego nie zainstalował)
  // biblioteką.
  const [checked, setChecked] = createSignal(false);
  const [showOnboarding, setShowOnboarding] = createSignal(false);

  onMount(async () => {
    try {
      // BUGFIX: `is_dev_mode` istniało już w backendzie (`hacker-mode dev`
      // — tryb do testowania UI na zwykłym pulpicie, bez integracji z
      // kompozytorem), ale `App.tsx` go nie sprawdzał przed pokazaniem
      // onboardingu. Efekt: każde uruchomienie `hacker-mode dev` na
      // świeżej maszynie dewelopera (bez `~/.hackeros/Hacker-Mode/.no-first-time`)
      // odpalało PRAWDZIWY ekran instalacji narzędzi — łącznie z
      // rzeczywistym `pip install` do `~/.hackeros/Hacker-Mode/env/` na
      // komputerze dewelopera. W trybie dev pomijamy onboarding
      // całkowicie i idziemy prosto do normalnego UI.
      const devMode = await api.isDevMode();
      setShowOnboarding(!devMode && (await api.isFirstRun()));
    } catch (err) {
      console.warn("Nie udało się sprawdzić pierwszego uruchomienia:", err);
    } finally {
      setChecked(true);
    }
  });

  return (
    <Show when={checked()} fallback={<div class="app-shell" />}>
      <Show
        when={!showOnboarding()}
        fallback={<Onboarding onDone={() => setShowOnboarding(false)} />}
      >
        <Router root={Layout}>
          <Route path="/" component={Home} />
          <Route path="/library" component={Library} />
          <Route path="/game/:platform/:id" component={GameDetail} />
          <Route path="/store" component={Store} />
          <Route path="/stats" component={Stats} />
          <Route path="/settings" component={Settings} />
        </Router>
      </Show>
    </Show>
  );
};

export default App;
