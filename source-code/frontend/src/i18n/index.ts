import pl from "./locales/pl.json";
import en from "./locales/en.json";
import de from "./locales/de.json";
import fr from "./locales/fr.json";
import es from "./locales/es.json";
import it from "./locales/it.json";
import pt from "./locales/pt.json";

export type Lang = "pl" | "en" | "de" | "fr" | "es" | "it" | "pt";

/** Języki interfejsu. Tłumaczenia leżą w osobnych plikach
 * `locales/<kod>.json`; `en.json` jest kanonicznym źródłem listy kluczy
 * (patrz `Translation`) i językiem zapasowym. Dodanie języka = nowy plik
 * JSON + wpis tutaj + wpis w `translations` poniżej; `npm run check:i18n`
 * (uruchamiane też w CI) pilnuje, żeby żaden klucz ani placeholder
 * (`{count}`, `{name}` ...) nie zniknął. */
export const LANGUAGES: { id: Lang; label: string }[] = [
  { id: "pl", label: "Polski" },
  { id: "en", label: "English" },
  { id: "de", label: "Deutsch" },
  { id: "fr", label: "Français" },
  { id: "es", label: "Español" },
  { id: "it", label: "Italiano" },
  { id: "pt", label: "Português" },
];

/** Kształt tłumaczenia wyprowadzony z `en.json`. Przypisanie niżej
 * `Record<Lang, Translation>` sprawia, że brakujący albo nadmiarowy klucz
 * w KTÓRYMKOLWIEK pliku językowym jest błędem kompilacji `tsc`. */
export type Translation = typeof en;
export type TranslationKey = keyof Translation;

export const translations: Record<Lang, Translation> = { pl, en, de, fr, es, it, pt };

/** Zwraca tekst w danym języku; brakujący klucz spada do angielskiego, a
 * dopiero w ostateczności do samego klucza. Wszystkie wystąpienia
 * `{zmienna}` są podstawiane (nie tylko pierwsze). */
export function getText(
  lang: Lang,
  key: TranslationKey,
  vars?: Record<string, string | number>,
): string {
  let text: string = translations[lang]?.[key] ?? translations.en[key] ?? key;
  if (vars) {
    for (const [name, value] of Object.entries(vars)) {
      text = text.split(`{${name}}`).join(String(value));
    }
  }
  return text;
}

export function detectLang(): Lang {
  const nav = typeof navigator !== "undefined" ? navigator.language : "en";
  const known = LANGUAGES.map((l) => l.id);
  const short = (nav?.split("-")[0] ?? "en") as Lang;
  return known.includes(short) ? short : "en";
}
