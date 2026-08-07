import { createSignal } from "solid-js";
import { api, type Game, type Platform } from "@/lib/tauri";

const [games, setGames] = createSignal<Game[]>([]);
const [isLoading, setIsLoading] = createSignal(false);
const [error, setError] = createSignal<string | null>(null);

async function refresh() {
  setIsLoading(true);
  setError(null);
  try {
    const result = await api.listGames();
    setGames(result);
  } catch (err) {
    console.warn("Nie udało się wczytać biblioteki gier:", err);
    setError(String(err));
    setGames([]);
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
  refresh,
  gamesByPlatform,
};
