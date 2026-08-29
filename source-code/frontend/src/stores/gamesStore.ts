import { createSignal } from "solid-js";
import { api, type Game, type Platform, type PlatformWarning } from "@/lib/tauri";

const [games, setGames] = createSignal<Game[]>([]);
const [isLoading, setIsLoading] = createSignal(false);
const [error, setError] = createSignal<string | null>(null);
/** BUGFIX: dawniej błędy per-platforma (np. `legendary` zwracające błąd)
 * ginęły całkowicie w logu backendu — `list_games` zwracało `Vec<Game>`
 * zamiast czegoś, co mogłoby też nieść błędy, więc `error` powyżej mogło
 * się ustawić tylko przy awarii samego IPC (w praktyce niemal nigdy).
 * Teraz backend (`stores::LibraryLoadResult`) zwraca też listę ostrzeżeń
 * per platforma — wyświetlanych w `Library.tsx`. */
const [warnings, setWarnings] = createSignal<PlatformWarning[]>([]);

async function refresh() {
  setIsLoading(true);
  setError(null);
  try {
    const result = await api.listGames();
    setGames(result.games);
    setWarnings(result.warnings);
  } catch (err) {
    console.warn("Nie udało się wczytać biblioteki gier:", err);
    setError(String(err));
    setGames([]);
    setWarnings([]);
  } finally {
    setIsLoading(false);
  }
}

function gamesByPlatform(platform: Platform) {
  return games().filter((g) => g.platform === platform);
}

export const gamesStore = {
  games,
  isLoading,
  error,
  warnings,
  refresh,
  gamesByPlatform,
};
