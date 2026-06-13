-- Fase 2: tabla de configuración global (fila única)
CREATE TABLE IF NOT EXISTS settings (
    id                  INTEGER     PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    servers_dir         TEXT        NOT NULL DEFAULT '/home/cubed/servers',
    backups_dir         TEXT        NOT NULL DEFAULT '/home/cubed/backups',
    downloads_dir       TEXT        NOT NULL DEFAULT '/home/cubed/downloads',
    default_java_path   TEXT        NOT NULL DEFAULT '/usr/bin/java',
    backup_interval_secs BIGINT     NOT NULL DEFAULT 18000
);

-- Insertar configuración por defecto si no existe
INSERT INTO settings DEFAULT VALUES ON CONFLICT DO NOTHING;
