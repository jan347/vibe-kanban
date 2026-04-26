-- Phase 11: auto-approval supervisor — extends safety_config with a
-- per-workspace policy describing what an automated supervisor model
-- may approve without prompting a human. auto_approval_log records
-- every decision (approved/denied/escalated) for audit.

ALTER TABLE safety_config ADD COLUMN auto_approval_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE safety_config ADD COLUMN auto_approval_policy TEXT;
ALTER TABLE safety_config ADD COLUMN auto_approval_model_preset_id BLOB
    REFERENCES model_presets(id) ON DELETE SET NULL;

CREATE TABLE auto_approval_log (
    id              BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    workspace_id    BLOB NOT NULL,
    action_kind     TEXT NOT NULL,
    action_summary  TEXT NOT NULL,
    decision        TEXT NOT NULL
                    CHECK (decision IN ('approved','denied','escalated')),
    reasoning       TEXT,
    decided_by      TEXT NOT NULL DEFAULT 'policy'
                    CHECK (decided_by IN ('policy','model','human')),
    decided_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_auto_approval_log_workspace ON auto_approval_log(workspace_id);
CREATE INDEX idx_auto_approval_log_decided_at ON auto_approval_log(decided_at);
