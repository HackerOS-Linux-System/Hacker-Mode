import { invoke } from '@tauri-apps/api/core';
import { Game, GameSource } from '../types';

export interface ImportResult {
  added: number; updated: number; total: number; errors: string[];
}

export interface LaunchSettings {
  closeOnLaunch: boolean; reopenOnExit: boolean;
  mangohudEnabled: boolean; gamescopeEnabled: boolean; vkbasaltEnabled: boolean;
}

// Raw types from Rust
interface RawSteamGame {
  app_id: number; name: string; installed: boolean;
  install_dir: string | null; last_played: number | null; play_time_seconds: number;
}

interface RawEpicGame {
  app_name: string; title: string; installed: boolean;
  install_path: string | null; cover_url: string | null;
  background_url: string | null; developer: string | null;
  can_run_offline: boolean;
}

interface RawGogGame {
  app_name: string; title: string; installed: boolean;
  install_path: string | null; cover_url: string | null; developer: string | null;
}

interface RawAmazonGame {
  asin: string; title: string; installed: boolean;
  install_path: string | null; image_url: string | null;
}

interface RawLutrisGame {
  id: number; slug: string; name: string;
  coverart: string | null; banner: string | null;
  lastplayed: number | null; playtime: number | null;
  runner: string | null;
}

class GameImportService {
  private games: Map<string, Game> = new Map();
  private initialized = false;

  async initialize(): Promise<void> {
    if (this.initialized) return;
    await this.loadFromDB();
    this.initialized = true;
  }

  private async loadFromDB() {
    try {
      const raw = await invoke<string>('read_games_db');
      if (raw) (JSON.parse(raw) as Game[]).forEach(g => this.games.set(g.id, g));
    } catch { /* first run */ }
  }

  private async saveDB() {
    try { await invoke('write_games_db', { data: JSON.stringify(Array.from(this.games.values())) }); }
    catch (e) { console.error('DB save failed', e); }
  }

  async importAll(onProgress?: (msg: string) => void): Promise<ImportResult> {
    const result: ImportResult = { added: 0, updated: 0, total: 0, errors: [] };

    onProgress?.('Skanowanie Steam...');
    try { const r = await this.importSteam(); result.added += r.added; result.updated += r.updated; result.errors.push(...r.errors); }
    catch (e) { result.errors.push(`Steam: ${String(e)}`); }

    onProgress?.('Skanowanie Epic Games (legendary)...');
    try { const r = await this.importEpic(); result.added += r.added; result.updated += r.updated; result.errors.push(...r.errors); }
    catch (e) { result.errors.push(`Epic: ${String(e)}`); }

    onProgress?.('Skanowanie GOG (legendary)...');
    try { const r = await this.importGog(); result.added += r.added; result.updated += r.updated; result.errors.push(...r.errors); }
    catch (e) { result.errors.push(`GOG: ${String(e)}`); }

    onProgress?.('Skanowanie Amazon Games (nile)...');
    try { const r = await this.importAmazon(); result.added += r.added; result.updated += r.updated; result.errors.push(...r.errors); }
    catch (e) { result.errors.push(`Amazon: ${String(e)}`); }

    onProgress?.('Skanowanie Lutris...');
    try { const r = await this.importLutris(); result.added += r.added; result.updated += r.updated; result.errors.push(...r.errors); }
    catch (e) { result.errors.push(`Lutris: ${String(e)}`); }

    result.total = this.games.size;
    await this.saveDB();
    return result;
  }

  private upsert(game: Game, existing?: Game): 'added' | 'updated' {
    if (existing) {
      this.games.set(game.id, { ...existing, ...game, favorite: existing.favorite, lastPlayed: Math.max(existing.lastPlayed ?? 0, game.lastPlayed ?? 0) || undefined });
      return 'updated';
    }
    this.games.set(game.id, game);
    return 'added';
  }

  private async importSteam(): Promise<ImportResult> {
    const r: ImportResult = { added: 0, updated: 0, total: 0, errors: [] };
    try {
      const raw = await invoke<RawSteamGame[]>('import_steam_games');
      for (const sg of raw) {
        const id = `steam_${sg.app_id}`;
        const existing = this.games.get(id);
        const result = this.upsert({
          id, name: sg.name, source: 'steam', appId: String(sg.app_id),
          coverUrl: `https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/${sg.app_id}/library_600x900.jpg`,
          backgroundUrl: `https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/${sg.app_id}/library_hero.jpg`,
          heroImageUrl: `https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/${sg.app_id}/capsule_616x353.jpg`,
          installed: sg.installed, installDir: sg.install_dir ?? undefined,
          lastPlayed: sg.last_played ?? undefined, playTime: sg.play_time_seconds,
          runCommand: `flatpak run com.valvesoftware.Steam steam://rungameid/${sg.app_id}`,
          favorite: existing?.favorite ?? false,
        }, existing);
        result === 'added' ? r.added++ : r.updated++;
      }
    } catch (e) { r.errors.push(String(e)); }
    return r;
  }

  private async importEpic(): Promise<ImportResult> {
    const r: ImportResult = { added: 0, updated: 0, total: 0, errors: [] };
    try {
      const raw = await invoke<RawEpicGame[]>('import_epic_games');
      for (const eg of raw) {
        const id = `epic_${eg.app_name}`;
        const existing = this.games.get(id);
        const result = this.upsert({
          id, name: eg.title, source: 'epic', appId: eg.app_name,
          legendaryAppName: eg.app_name,
          coverUrl: eg.cover_url ?? undefined,
          backgroundUrl: eg.background_url ?? undefined,
          installed: eg.installed, installDir: eg.install_path ?? undefined,
          developer: eg.developer ?? undefined,
          lastPlayed: existing?.lastPlayed, playTime: existing?.playTime ?? 0,
          // Launch via legendary in venv
          runCommand: `hm-launch-epic ${eg.app_name}`,
          favorite: existing?.favorite ?? false,
        }, existing);
        result === 'added' ? r.added++ : r.updated++;
      }
    } catch (e) { r.errors.push(String(e)); }
    return r;
  }

  private async importGog(): Promise<ImportResult> {
    const r: ImportResult = { added: 0, updated: 0, total: 0, errors: [] };
    try {
      const raw = await invoke<RawGogGame[]>('import_gog_games');
      for (const gg of raw) {
        const id = `gog_${gg.app_name}`;
        const existing = this.games.get(id);
        const res = this.upsert({
          id, name: gg.title, source: 'gog', appId: gg.app_name,
          legendaryAppName: gg.app_name,
          coverUrl: gg.cover_url ?? undefined,
          installed: gg.installed, installDir: gg.install_path ?? undefined,
          developer: gg.developer ?? undefined,
          lastPlayed: existing?.lastPlayed, playTime: existing?.playTime ?? 0,
          runCommand: `hm-launch-gog ${gg.app_name}`,
          favorite: existing?.favorite ?? false,
        }, existing);
        res === 'added' ? r.added++ : r.updated++;
      }
    } catch (e) { r.errors.push(String(e)); }
    return r;
  }

  private async importAmazon(): Promise<ImportResult> {
    const r: ImportResult = { added: 0, updated: 0, total: 0, errors: [] };
    try {
      const raw = await invoke<RawAmazonGame[]>('import_amazon_games');
      for (const ag of raw) {
        const id = `amazon_${ag.asin}`;
        const existing = this.games.get(id);
        const res = this.upsert({
          id, name: ag.title, source: 'amazon', appId: ag.asin,
          coverUrl: ag.image_url ?? undefined,
          installed: ag.installed, installDir: ag.install_path ?? undefined,
          lastPlayed: existing?.lastPlayed, playTime: existing?.playTime ?? 0,
          runCommand: `hm-launch-amazon ${ag.asin}`,
          favorite: existing?.favorite ?? false,
        }, existing);
        res === 'added' ? r.added++ : r.updated++;
      }
    } catch (e) { r.errors.push(String(e)); }
    return r;
  }

  private async importLutris(): Promise<ImportResult> {
    const r: ImportResult = { added: 0, updated: 0, total: 0, errors: [] };
    try {
      const raw = await invoke<RawLutrisGame[]>('import_lutris_games');
      for (const lg of raw) {
        const id = `lutris_${lg.slug}`;
        const existing = this.games.get(id);
        const coverUrl = lg.coverart
          ? (lg.coverart.startsWith('/') ? `file://${lg.coverart}` : lg.coverart)
          : undefined;
        const res = this.upsert({
          id, name: lg.name, source: 'lutris', appId: String(lg.id),
          coverUrl, backgroundUrl: lg.banner ? `file://${lg.banner}` : undefined,
          installed: true,
          lastPlayed: lg.lastplayed ? lg.lastplayed * 1000 : undefined,
          playTime: lg.playtime ? lg.playtime * 3600 : 0,
          // Launch lutris game directly via lutris CLI (without opening the GUI)
          runCommand: `lutris lutris:rungame/${lg.id}`,
          favorite: existing?.favorite ?? false,
        }, existing);
        res === 'added' ? r.added++ : r.updated++;
      }
    } catch (e) { r.errors.push(String(e)); }
    return r;
  }

  getGames(): Game[] { return Array.from(this.games.values()); }
  getGame(id: string): Game | undefined { return this.games.get(id); }

  async updateGame(id: string, updates: Partial<Game>) {
    const ex = this.games.get(id);
    if (ex) { this.games.set(id, { ...ex, ...updates }); await this.saveDB(); }
  }

  async addManualGame(game: Omit<Game, 'id' | 'source'>): Promise<Game> {
    const id = `manual_${Date.now()}`;
    const ng: Game = { ...game, id, source: 'manual' };
    this.games.set(id, ng); await this.saveDB(); return ng;
  }

  async removeGame(id: string) { this.games.delete(id); await this.saveDB(); }

  async launchGame(gameId: string, settings: LaunchSettings) {
    const game = this.games.get(gameId);
    if (!game) throw new Error('Game not found');
    await this.updateGame(gameId, { lastPlayed: Date.now() });
    const env: Record<string, string> = {};
    if (settings.mangohudEnabled) env['MANGOHUD'] = '1';
    if (settings.vkbasaltEnabled) env['ENABLE_VKBASALT'] = '1';
    await invoke('launch_game', {
      gameId, command: game.runCommand ?? '', source: game.source,
      closeOnLaunch: settings.closeOnLaunch, reopenOnExit: settings.reopenOnExit,
      env, gamescopeEnabled: settings.gamescopeEnabled,
    });
  }
}

export const gameImportService = new GameImportService();
