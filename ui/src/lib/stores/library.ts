import { writable } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";

export type Platform = "Steam" | "Epic" | "Gog" | "Amazon" | "Native";

export interface Game {
  id: string;
  title: string;
  platform: Platform;
  installed: boolean;
  install_dir: string | null;
  cover_url: string | null;
  exec: string | null;
  needs_proton: boolean;
  playtime_minutes: number;
}

export const games = writable<Game[]>([]);
export const loadingLibrary = writable(false);

export async function refreshLibrary() {
  loadingLibrary.set(true);
  try {
    const result = await invoke<Game[]>("list_all_games");
    games.set(result);
  } finally {
    loadingLibrary.set(false);
  }
}

export async function launchGame(game: Game) {
  await invoke("launch_game", { game });
}

export const platformLabels: Record<Platform, string> = {
  Steam: "Steam",
  Epic: "Epic Games",
  Gog: "GOG",
  Amazon: "Amazon Games",
  Native: "Ręcznie dodane",
};
