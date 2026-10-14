import { Component } from "solid-js";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";

/** Przełączniki bezpieczeństwa: obie opcje są domyślnie WYŁĄCZONE. */
const SafetyPanel: Component = () => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);

  return (
    <div class="settings-panel" role="group" aria-labelledby="safety-heading">
      <h3 id="safety-heading">{t("advanced_safety_panel_title")}</h3>
      <div class="settings-row">
        <label for="allow-config-write">{t("allow_config_write_label")}</label>
        <input
          id="allow-config-write"
          data-focusable
          tabIndex={0}
          type="checkbox"
          checked={settingsStore.settings().allow_config_write_while_running}
          onChange={(e) => settingsStore.update({ allow_config_write_while_running: e.currentTarget.checked })}
        />
      </div>
      <div class="settings-row">
        <label for="lutris-delete-files">{t("lutris_delete_files_label")}</label>
        <input
          id="lutris-delete-files"
          data-focusable
          tabIndex={0}
          type="checkbox"
          checked={settingsStore.settings().lutris_delete_files_on_uninstall}
          onChange={(e) => settingsStore.update({ lutris_delete_files_on_uninstall: e.currentTarget.checked })}
        />
      </div>
    </div>
  );
};

export default SafetyPanel;
