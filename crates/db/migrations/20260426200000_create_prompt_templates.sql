CREATE TABLE prompt_templates (
    id          BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    name        TEXT NOT NULL UNIQUE,
    role        TEXT NOT NULL
                CHECK (role IN (
                    'implement','investigate','review','qa',
                    'design_polish','docs','security','other'
                )),
    description TEXT,
    body_text   TEXT NOT NULL,
    preset_id   BLOB,
    bundle_id   BLOB,
    tags_json   TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (preset_id) REFERENCES model_presets(id) ON DELETE SET NULL,
    FOREIGN KEY (bundle_id) REFERENCES repo_bundles(id) ON DELETE SET NULL
);

CREATE INDEX idx_prompt_templates_role ON prompt_templates(role);
CREATE INDEX idx_prompt_templates_preset_id ON prompt_templates(preset_id);
CREATE INDEX idx_prompt_templates_bundle_id ON prompt_templates(bundle_id);
