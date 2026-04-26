CREATE TABLE repo_bundles (
    id                              BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    name                            TEXT NOT NULL UNIQUE,
    description                     TEXT,
    repo_ids_json                   TEXT NOT NULL,
    default_branch_overrides_json   TEXT,
    default_preset_id               BLOB,
    created_at                      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at                      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (default_preset_id) REFERENCES model_presets(id) ON DELETE SET NULL
);

CREATE INDEX idx_repo_bundles_name ON repo_bundles(name);
CREATE INDEX idx_repo_bundles_default_preset_id ON repo_bundles(default_preset_id);
