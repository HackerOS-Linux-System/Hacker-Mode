-- HackerOS Hacker Mode – initial schema

CREATE TABLE IF NOT EXISTS games (
    id             TEXT    NOT NULL,
    store_id       TEXT    NOT NULL,
    source         TEXT    NOT NULL,   -- steam|epic|gog|amazon|itchio|lutris|native
    title          TEXT    NOT NULL,
    developer      TEXT,
    publisher      TEXT,
    install_path   TEXT,
    executable     TEXT,
    cover_path     TEXT,
    hero_path      TEXT,
    size_bytes     INTEGER,
    runner         TEXT    NOT NULL DEFAULT 'null',
    proton_version TEXT,
    wine_prefix    TEXT,
    is_installed   INTEGER NOT NULL DEFAULT 0,
    is_favourite   INTEGER NOT NULL DEFAULT 0,
    last_played    INTEGER,
    added_at       INTEGER NOT NULL,
    PRIMARY KEY (id),
    UNIQUE (store_id, source)
);

CREATE INDEX IF NOT EXISTS games_source ON games (source);
CREATE INDEX IF NOT EXISTS games_last_played ON games (last_played DESC);

CREATE TABLE IF NOT EXISTS playtime (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id        TEXT    NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    session_start  INTEGER NOT NULL,
    session_end    INTEGER,
    duration_secs  INTEGER
);

CREATE INDEX IF NOT EXISTS playtime_game ON playtime (game_id);

CREATE TABLE IF NOT EXISTS auth_tokens (
    store   TEXT    NOT NULL PRIMARY KEY,   -- epic|gog|itchio|amazon
    data    TEXT    NOT NULL,               -- JSON blob (encrypted fields stored in keyring)
    updated INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
);
