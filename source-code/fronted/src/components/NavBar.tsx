import { A, useLocation, useNavigate } from "@solidjs/router";
import { Component, onMount } from "solid-js";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";

const ROUTES = ["/", "/library", "/store", "/settings"];

const NavBar: Component<{ onOpenMenu: () => void }> = (props) => {
  const location = useLocation();
  const navigate = useNavigate();
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);
  const isActive = (path: string) => location.pathname === path;

  onMount(() => {
    // Zderzaki (L1/R1, przyciski 4/5) przełączają zakładki — wygodne w
    // trybie big-picture, gdzie D-pad jest zwykle zajęty nawigacją w
    // siatce gier, a nie przełączaniem sekcji.
    let lastSwitchAt = 0;
    const raf = () => {
      const pads = navigator.getGamepads?.() ?? [];
      for (const pad of pads) {
        if (!pad) continue;
        const now = performance.now();
        const l1 = pad.buttons[4]?.pressed ?? false;
        const r1 = pad.buttons[5]?.pressed ?? false;
        if ((l1 || r1) && now - lastSwitchAt > 300) {
          const idx = ROUTES.indexOf(location.pathname);
          const nextIdx = idx === -1 ? 0 : (idx + (r1 ? 1 : -1) + ROUTES.length) % ROUTES.length;
          navigate(ROUTES[nextIdx]);
          lastSwitchAt = now;
          break;
        }
      }
      requestAnimationFrame(raf);
    };
    const handle = requestAnimationFrame(raf);
    return () => cancelAnimationFrame(handle);
  });

  return (
    <nav class="sidebar">
      <div class="logo">HM</div>

      <A href="/" class={`nav-btn ${isActive("/") ? "active" : ""}`} data-focusable tabIndex={0}>
        <span>🏠</span>
        {t("home")}
      </A>
      <A href="/library" class={`nav-btn ${isActive("/library") ? "active" : ""}`} data-focusable tabIndex={0}>
        <span>🎮</span>
        {t("library")}
      </A>
      <A href="/store" class={`nav-btn ${isActive("/store") ? "active" : ""}`} data-focusable tabIndex={0}>
        <span>🛒</span>
        {t("store")}
      </A>
      <A href="/settings" class={`nav-btn ${isActive("/settings") ? "active" : ""}`} data-focusable tabIndex={0}>
        <span>⚙️</span>
        {t("settings")}
      </A>

      <div style={{ "margin-top": "auto" }}>
        <button class="nav-btn" onClick={props.onOpenMenu} data-focusable tabIndex={0}>
          <span>⏻</span>
          {t("hacker_menu")}
        </button>
      </div>
    </nav>
  );
};

export default NavBar;
