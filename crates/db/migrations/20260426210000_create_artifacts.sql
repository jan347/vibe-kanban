-- Phase 6: artifact canvas — Excalidraw + Markdown only (per design doc).
-- Mermaid, screenshots, JSON/CSV, QA reports deferred to follow-up passes
-- if dogfooding shows demand.

CREATE TABLE artifacts (
    id                              BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    work_item_id                    BLOB,
    kind                            TEXT NOT NULL
                                    CHECK (kind IN ('excalidraw','markdown')),
    title                           TEXT NOT NULL,
    body_text                       TEXT,
    body_json                       TEXT,
    version                         INTEGER NOT NULL DEFAULT 1,
    parent_artifact_id              BLOB,
    created_by_kind                 TEXT NOT NULL
                                    CHECK (created_by_kind IN ('workspace','human')),
    created_by_workspace_id         BLOB,
    created_by_execution_process_id BLOB,
    created_at                      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    accepted_at                     TEXT,
    FOREIGN KEY (work_item_id) REFERENCES work_items(id) ON DELETE SET NULL,
    FOREIGN KEY (parent_artifact_id) REFERENCES artifacts(id) ON DELETE SET NULL,
    FOREIGN KEY (created_by_workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL,
    FOREIGN KEY (created_by_execution_process_id) REFERENCES execution_processes(id) ON DELETE SET NULL
);

CREATE INDEX idx_artifacts_work_item_id ON artifacts(work_item_id);
CREATE INDEX idx_artifacts_kind ON artifacts(kind);
CREATE INDEX idx_artifacts_parent_artifact_id ON artifacts(parent_artifact_id);
CREATE INDEX idx_artifacts_accepted_at ON artifacts(accepted_at) WHERE accepted_at IS NOT NULL;
CREATE INDEX idx_artifacts_created_at ON artifacts(created_at);

-- Phase 6 also adds artifact_id to mail_attachments so attachments
-- can link to artifacts from the canvas. Per design doc fix #7:
-- artifact_id is nullable, inline_blob_path becomes nullable, XOR
-- constraint ensures exactly one is set.
--
-- SQLite doesn't support ALTER COLUMN to drop NOT NULL, so we recreate.
CREATE TABLE mail_attachments_new (
    id               BLOB PRIMARY KEY,
    message_id       BLOB NOT NULL,
    inline_blob_path TEXT,
    artifact_id      BLOB,
    mime_type        TEXT,
    size_bytes       INTEGER,
    filename         TEXT,
    created_at       TEXT NOT NULL,
    CHECK ((artifact_id IS NULL) != (inline_blob_path IS NULL)),
    FOREIGN KEY (message_id) REFERENCES mail_messages(id) ON DELETE CASCADE,
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE SET NULL
);

INSERT INTO mail_attachments_new (id, message_id, inline_blob_path, mime_type, size_bytes, filename, created_at)
SELECT id, message_id, inline_blob_path, mime_type, size_bytes, filename, created_at
FROM mail_attachments;

DROP TABLE mail_attachments;
ALTER TABLE mail_attachments_new RENAME TO mail_attachments;

CREATE INDEX idx_mail_attachments_message_id ON mail_attachments(message_id);
CREATE INDEX idx_mail_attachments_artifact_id ON mail_attachments(artifact_id) WHERE artifact_id IS NOT NULL;

-- Shared-context view: aggregates accepted artifacts + answered mail
-- responses for a work item (used by Phase 5 templates and Phase 7 MCP).
CREATE VIEW work_item_shared_context AS
SELECT
    wi.id AS work_item_id,
    'artifact' AS context_type,
    a.id AS context_id,
    a.title AS context_title,
    COALESCE(a.body_text, a.body_json) AS context_body,
    a.accepted_at AS context_at
FROM work_items wi
JOIN artifacts a ON a.work_item_id = wi.id AND a.accepted_at IS NOT NULL
UNION ALL
SELECT
    t.work_item_id AS work_item_id,
    'mail_response' AS context_type,
    r.id AS context_id,
    t.subject AS context_title,
    r.response_value_json AS context_body,
    r.responded_at AS context_at
FROM mail_threads t
JOIN mail_messages m ON m.thread_id = t.id AND m.requires_response = 1
JOIN mail_recipients r ON r.message_id = m.id
    AND r.responded_at IS NOT NULL
    AND r.response_value_json IS NOT NULL
WHERE t.work_item_id IS NOT NULL;
