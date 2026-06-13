-- Fase 2: tabla principal de servidores Minecraft
CREATE TABLE IF NOT EXISTS servers (
    id          UUID        PRIMARY KEY,
    name        TEXT        NOT NULL UNIQUE,
    version     TEXT        NOT NULL,
    software    TEXT        NOT NULL,
    port        INTEGER     NOT NULL UNIQUE CHECK (port >= 1024 AND port <= 65535),
    java_path   TEXT        NOT NULL,
    status      TEXT        NOT NULL DEFAULT 'offline',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_servers_status ON servers (status);
