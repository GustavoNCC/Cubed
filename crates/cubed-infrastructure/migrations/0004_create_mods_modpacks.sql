-- Fase 12/13: mods individuales y modpacks importados
CREATE TABLE IF NOT EXISTS mods (
    id          UUID PRIMARY KEY,
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    file_name   TEXT NOT NULL,
    path        TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mods_server_id ON mods (server_id);

CREATE TABLE IF NOT EXISTS modpacks (
    id          UUID PRIMARY KEY,
    server_id   UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    format      TEXT NOT NULL,
    source_path TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_modpacks_server_id ON modpacks (server_id);
