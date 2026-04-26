-- Phase 9: safety configuration — per-workspace guardrails for
-- dispatch operations. Controls confirmation requirements, rate
-- limits, and budget caps.

CREATE TABLE safety_config (
    id                      BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    workspace_id            BLOB,
    scope                   TEXT NOT NULL DEFAULT 'global'
                            CHECK (scope IN ('global','workspace')),
    require_human_approval  INTEGER NOT NULL DEFAULT 1,
    max_concurrent_dispatch INTEGER NOT NULL DEFAULT 3,
    max_daily_dispatch      INTEGER NOT NULL DEFAULT 50,
    cooldown_seconds        INTEGER NOT NULL DEFAULT 30,
    created_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_safety_config_scope ON safety_config(scope, workspace_id);

INSERT INTO safety_config (id, scope, require_human_approval, max_concurrent_dispatch, max_daily_dispatch, cooldown_seconds)
VALUES (randomblob(16), 'global', 1, 3, 50, 30);
