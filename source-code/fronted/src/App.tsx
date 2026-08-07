import { Component, createSignal, onMount } from "solid-js";
import { Route, Router } from "@solidjs/router";
import NavBar from "@/components/NavBar";
import HackerMenu from "@/components/HackerMenu";
import Home from "@/routes/Home";
import Library from "@/routes/Library";
import Store from "@/routes/Store";
import Settings from "@/routes/Settings";
import GameDetail from "@/routes/GameDetail";
import { useSettingsBootstrap } from "@/stores/settingsStore";
import { api } from "@/lib/tauri";
import { gamesStore } from "@/stores/gamesStore";
import { installNavigation } from "@/lib/focusNav";

const Layout: Component<{ children?: any }> = (props) => {
  const [menuOpen, setMenuOpen] = createSignal(false);
  useSettingsBootstrap();

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
  return (
    <Router root={Layout}>
      <Route path="/" component={Home} />
      <Route path="/library" component={Library} />
      <Route path="/game/:platform/:id" component={GameDetail} />
      <Route path="/store" component={Store} />
      <Route path="/settings" component={Settings} />
    </Router>
  );
};

export default App;
