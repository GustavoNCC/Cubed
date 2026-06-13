-- Fase 2: tabla de backups
CREATE TABLE IF NOT EXISTS backups (
    id          UUID        PRIMARY KEY,
    server_id   UUID        NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    path        TEXT        NOT NULL,
    size_bytes  BIGINT      NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_backups_server_id ON backups (server_id);
