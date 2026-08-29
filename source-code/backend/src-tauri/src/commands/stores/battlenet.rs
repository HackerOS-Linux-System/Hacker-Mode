use super::{Game, Platform, StoreError, StoreProvider};
use std::process::Command;

pub struct BattleNetProvider {
    pub explicit_prefix: Option<std::path::PathBuf>,
}

impl Default for BattleNetProvider {
    fn default() -> Self {
        // Patrz komentarz przy `EaProvider::default()` w `ea.rs` — ten sam
        // wzorzec (samodzielny odczyt `config.json`), żeby zachować
        // jednolite `Platform::BattleNet => BattleNetProvider::default()`.
        Self {
            explicit_prefix: crate::settings::Settings::load_or_default()
                .battlenet_wine_prefix
                .map(std::path::PathBuf::from),
        }
    }
}

impl From<bnet::BnetError> for StoreError {
    fn from(err: bnet::BnetError) -> Self {
        match err {
            bnet::BnetError::WineNotInstalled => StoreError::ToolNotInstalled("wine".into()),
            other => StoreError::ExecFailed("battle.net".into(), other.to_string()),
        }
    }
}

impl StoreProvider for BattleNetProvider {
    fn platform(&self) -> Platform {
        Platform::BattleNet
    }

    fn is_available(&self) -> bool {
        bnet::is_available() && bnet::find_prefix(self.explicit_prefix.as_deref()).is_ok()
    }

    fn list_games(&self) -> Result<Vec<Game>, StoreError> {
        let prefix = bnet::find_prefix(self.explicit_prefix.as_deref())?;
        let games = bnet::list_installed_games(&prefix)?;

        // Zapasowe okładki przez SteamGridDB — patrz identyczny wzorzec i
        // pełny komentarz w `ea.rs::list_games`.
        let api_key = crate::settings::Settings::load_or_default().steamgriddb_api_key;
        let covers = fetch_covers_if_configured(&games, api_key.as_deref(), "battlenet");

        Ok(games
            .into_iter()
            .zip(covers)
            .map(|(g, cover_path)| Game {
                id: g.id,
                title: g.title,
                platform: Platform::BattleNet,
                installed: true,
                install_dir: Some(g.install_dir.to_string_lossy().into_owned()),
                cover_path,
                // Uzupełniane zapasowo centralnie przez `playtime::merge_into`
                // — patrz komentarz w `ea.rs::list_games`.
                playtime_minutes: None,
            })
            .collect())
    }

    fn install_command(&self, _game_id: &str) -> Result<Command, StoreError> {
        // Patrz `ea.rs::install_command` — identyczny powód i te same
        // ograniczenia (brak prywatnego API Blizzarda), tu dla klienta
        // Battle.net zamiast EA Desktop.
        self.launch_command(_game_id)
    }

    fn uninstall_command(&self, game_id: &str) -> Result<Command, StoreError> {
        // Patrz `ea.rs::uninstall_command` po pełne uzasadnienie —
        // identyczny wzorzec (usunięcie katalogu instalacji gry), tu przez
        // `bnet::resolve_game_dir`, który dodatkowo chroni sam katalog
        // klienta Battle.net i systemowe foldery Wine
        // (`bnet::NON_GAME_DIRS`) przed przypadkowym "odinstalowaniem".
        let prefix = bnet::find_prefix(self.explicit_prefix.as_deref())?;
        let dir = bnet::resolve_game_dir(&prefix, game_id)?;
        let mut cmd = Command::new("rm");
        cmd.args(["-rf", "--"]);
        cmd.arg(&dir);
        Ok(cmd)
    }

    fn launch_command(&self, _game_id: &str) -> Result<Command, StoreError> {
        // Tak jak w EA — uruchamiamy sam klient Battle.net, nie konkretną
        // grę z jego pominięciem (nieznane prywatne API Blizzarda).
        let prefix = bnet::find_prefix(self.explicit_prefix.as_deref())?;
        Ok(bnet::build_launch_battlenet_command(&prefix)?)
    }
}

/// Patrz `ea.rs::fetch_covers_if_configured` po pełny opis — identyczna
/// logika nad `bnet::InstalledGame`.
fn fetch_covers_if_configured(
    games: &[bnet::InstalledGame],
    api_key: Option<&str>,
    cache_prefix: &str,
) -> Vec<Option<String>> {
    let mut covers: Vec<Option<String>> = vec![None; games.len()];
    let Some(api_key) = api_key.filter(|k| !k.trim().is_empty()) else {
        return covers;
    };
    let Some(client) = crate::cover_cache::build_client() else {
        return covers;
    };
    for (game, slot) in games.iter().zip(covers.iter_mut()) {
        let cache_key = format!("{cache_prefix}-{}", game.id);
        *slot = crate::steamgriddb::fetch_cover_by_title(&game.title, &cache_key, api_key, &client);
    }
    covers
}
