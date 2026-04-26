CREATE TABLE work_items (
    id          BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    title       TEXT NOT NULL,
    description TEXT,
    status      TEXT NOT NULL DEFAULT 'open'
                CHECK (status IN ('open','in_progress','blocked','done','archived')),
    priority    INTEGER NOT NULL DEFAULT 0,
    tags_json   TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX idx_work_items_status ON work_items(status);
CREATE INDEX idx_work_items_priority ON work_items(priority);
CREATE INDEX idx_work_items_updated_at ON work_items(updated_at);

CREATE TABLE work_item_runs (
    work_item_id BLOB NOT NULL,
    workspace_id BLOB NOT NULL,
    role         TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (work_item_id, workspace_id),
    FOREIGN KEY (work_item_id) REFERENCES work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_work_item_runs_workspace_id ON work_item_runs(workspace_id);
CREATE INDEX idx_work_item_runs_role ON work_item_runs(role);
