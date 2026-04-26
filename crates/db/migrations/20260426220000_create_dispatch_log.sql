-- Phase 8: dispatch log — tracks orchestrated task dispatches from
-- work items to workspace sessions. One row per dispatch attempt.

CREATE TABLE dispatch_log (
    id                  BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    work_item_id        BLOB NOT NULL,
    workspace_id        BLOB NOT NULL,
    session_id          BLOB,
    prompt_template_id  BLOB,
    prompt_text         TEXT NOT NULL,
    model_preset_id     BLOB,
    status              TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending','running','completed','failed','cancelled')),
    started_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    completed_at        TEXT,
    error_message       TEXT,
    FOREIGN KEY (work_item_id) REFERENCES work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (prompt_template_id) REFERENCES prompt_templates(id) ON DELETE SET NULL
);

CREATE INDEX idx_dispatch_log_work_item_id ON dispatch_log(work_item_id);
CREATE INDEX idx_dispatch_log_workspace_id ON dispatch_log(workspace_id);
CREATE INDEX idx_dispatch_log_status ON dispatch_log(status);
