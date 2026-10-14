import { Component, createSignal, For } from "solid-js";
import { api, type Platform } from "@/lib/tauri";
import { settingsStore } from "@/stores/settingsStore";
import { getText } from "@/i18n";

/** Tagi użytkownika — patrz `Settings::game_tags`. Czysto organizacyjne,
 * filtrowane w `Library.tsx`; Hacker Mode nigdy nie interpretuje ich treści.
 * Czytane/zapisywane wprost przez `settingsStore` (klucz `platform:id`
 * zawsze wskazuje aktualne ustawienia, więc nawigacja między grami nie
 * pokazuje „przeterminowanych" tagów). */
const GameTags: Component<{ platform: Platform; id: string }> = (props) => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);
  const [tagInput, setTagInput] = createSignal("");
  const tagKey = () => `${props.platform}:${props.id}`;
  const currentTags = () => settingsStore.settings().game_tags[tagKey()] ?? [];

  async function addTag() {
    const value = tagInput().trim();
    if (!value) return;
    const next = [...new Set([...currentTags(), value])];
    setTagInput("");
    try {
      await api.setGameTags(props.platform, props.id, next);
      await settingsStore.reload();
    } catch (err) {
      console.warn("Nie udało się zapisać tagu:", err);
    }
  }

  async function removeTag(tag: string) {
    const next = currentTags().filter((existing) => existing !== tag);
    try {
      await api.setGameTags(props.platform, props.id, next);
      await settingsStore.reload();
    } catch (err) {
      console.warn("Nie udało się usunąć tagu:", err);
    }
  }

  return (
    <div
      role="group"
      aria-label="Tagi"
      style={{ display: "flex", "flex-wrap": "wrap", gap: "6px", "align-items": "center", "margin-bottom": "12px" }}
    >
      <For each={currentTags()}>
        {(tag) => (
          <span
            style={{
              display: "inline-flex", "align-items": "center", gap: "4px",
              "font-size": "11px", padding: "2px 8px", "border-radius": "999px",
              background: "rgba(255,255,255,0.08)", color: "var(--text)",
            }}
          >
            {tag}
            <button
              data-focusable
              tabIndex={0}
              aria-label={`Usuń tag ${tag}`}
              onClick={() => removeTag(tag)}
              style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", padding: "0", "font-size": "12px" }}
            >
              <span aria-hidden="true">✕</span>
            </button>
          </span>
        )}
      </For>
      <input
        data-focusable
        tabIndex={0}
        type="text"
        aria-label={t("tag_input_placeholder")}
        value={tagInput()}
        onInput={(e) => setTagInput(e.currentTarget.value)}
        onKeyDown={(e) => e.key === "Enter" && addTag()}
        placeholder={t("tag_input_placeholder")}
        style={{ width: "80px", "font-size": "11px", padding: "3px 8px", "border-radius": "999px", border: "1px dashed rgba(255,255,255,0.2)", background: "transparent", color: "var(--text)" }}
      />
    </div>
  );
};

export default GameTags;
