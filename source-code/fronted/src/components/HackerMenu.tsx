import { Component } from "solid-js";
import { api } from "@/lib/tauri";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";

const HackerMenu: Component<{ onClose: () => void }> = (props) => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);

  return (
    <div class="hacker-menu-overlay" onClick={props.onClose}>
      <div class="hacker-menu" onClick={(e) => e.stopPropagation()}>
        <button class="ghost-btn" onClick={() => api.restartApps()}>
          🔄 {t("restart_apps")}
        </button>
        <button class="ghost-btn" onClick={() => api.switchToDesktopSession("plasma")}>
          🖥 {t("switch_desktop")} (KDE Plasma)
        </button>
        <button class="ghost-btn" onClick={() => api.powerAction("sleep")}>
          🌙 {t("sleep")}
        </button>
        <button class="ghost-btn" onClick={() => api.powerAction("restart")}>
          ♻️ {t("restart")}
        </button>
        <button class="ghost-btn" style={{ color: "var(--danger)" }} onClick={() => api.powerAction("shutdown")}>
          ⏻ {t("shutdown")}
        </button>
        <button class="ghost-btn" onClick={props.onClose}>
          {t("close")}
        </button>
      </div>
    </div>
  );
};

export default HackerMenu;
