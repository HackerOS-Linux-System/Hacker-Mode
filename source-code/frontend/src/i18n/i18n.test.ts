import { describe, expect, it } from "vitest";
import { LANGUAGES, detectLang, getText, translations, type Lang } from "./index";

describe("i18n", () => {
  it("every registered language has exactly the English key set", () => {
    const enKeys = Object.keys(translations.en).sort();
    for (const { id } of LANGUAGES) {
      expect(Object.keys(translations[id as Lang]).sort(), id).toEqual(enKeys);
    }
  });

  it("keeps placeholders identical across languages", () => {
    const ph = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
    for (const [key, value] of Object.entries(translations.en)) {
      for (const { id } of LANGUAGES) {
        const other = (translations[id as Lang] as Record<string, string>)[key];
        expect(ph(other), `${id}:${key}`).toEqual(ph(value));
      }
    }
  });

  it("substitutes every occurrence of a variable, not only the first", () => {
    const text = getText("en", "launch_cooldown", { seconds: 7, app: "Steam" });
    expect(text).toContain("7");
    expect(text).toContain("Steam");
    expect(text).not.toContain("{");
    const twice = getText("en", "bulk_confirm_install", { count: 3 });
    expect(twice).toContain("3");
  });

  it("falls back to English and then to the key itself", () => {
    const broken = { ...translations.pl } as Record<string, string>;
    delete broken.settings;
    (translations as Record<string, unknown>).pl = broken;
    try {
      expect(getText("pl", "settings")).toBe(translations.en.settings);
    } finally {
      (translations as Record<string, unknown>).pl = { ...broken, settings: "Ustawienia" };
    }
    expect(getText("en", "no_such_key" as never)).toBe("no_such_key");
  });

  it("detectLang falls back to English for unknown locales", () => {
    Object.defineProperty(globalThis, "navigator", { value: { language: "xx-YY" }, configurable: true });
    expect(detectLang()).toBe("en");
    Object.defineProperty(globalThis, "navigator", { value: { language: "it-IT" }, configurable: true });
    expect(detectLang()).toBe("it");
  });

  it("new UI strings exist in the Polish and English files", () => {
    for (const key of ["compositor_label", "dependencies_panel_title", "aria_main_nav"] as const) {
      expect(translations.pl[key]).toBeTruthy();
      expect(translations.en[key]).toBeTruthy();
    }
  });
});
