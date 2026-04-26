-- Phase 10: automation rules — programmatic dispatch triggers.
-- A rule binds a work_item + prompt_template + model_preset to a
-- trigger (manual, schedule, or event). Firing creates a dispatch_log
-- entry that downstream orchestration evaluates against safety_config.

CREATE TABLE automation_rules (
    id                  BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    workspace_id        BLOB NOT NULL,
    work_item_id        BLOB NOT NULL,
    name                TEXT NOT NULL,
    trigger_kind        TEXT NOT NULL DEFAULT 'manual'
                        CHECK (trigger_kind IN ('manual','schedule','event')),
    trigger_config      TEXT NOT NULL DEFAULT '{}',
    prompt_template_id  BLOB,
    model_preset_id     BLOB,
    prompt_override     TEXT,
    enabled             INTEGER NOT NULL DEFAULT 1,
    last_fired_at       TEXT,
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (work_item_id) REFERENCES work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (prompt_template_id) REFERENCES prompt_templates(id) ON DELETE SET NULL,
    FOREIGN KEY (model_preset_id) REFERENCES model_presets(id) ON DELETE SET NULL
);

CREATE INDEX idx_automation_rules_workspace ON automation_rules(workspace_id);
CREATE INDEX idx_automation_rules_work_item ON automation_rules(work_item_id);
CREATE INDEX idx_automation_rules_enabled ON automation_rules(enabled);
