CREATE TABLE model_presets (
    id                 BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    name               TEXT NOT NULL UNIQUE,
    description        TEXT,
    role               TEXT NOT NULL
                       CHECK (role IN (
                           'planner','implementer','reviewer','qa',
                           'diagrammer','summarizer','other'
                       )),
    executor           TEXT NOT NULL
                       CHECK (executor IN (
                           'CLAUDE_CODE','CODEX','QWEN_CODE','OPENCODE',
                           'GEMINI','CURSOR_AGENT','AMP','DROID'
                       )),
    model_id           TEXT NOT NULL,
    permission_mode    TEXT,
    reasoning_effort   TEXT,
    env_vars_json      TEXT,
    labels_json        TEXT,
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX idx_model_presets_role ON model_presets(role);
CREATE INDEX idx_model_presets_executor ON model_presets(executor);
CREATE INDEX idx_model_presets_updated_at ON model_presets(updated_at);
