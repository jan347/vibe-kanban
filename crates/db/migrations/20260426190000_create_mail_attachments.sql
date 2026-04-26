CREATE TABLE mail_attachments (
    id               BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    message_id       BLOB NOT NULL,
    inline_blob_path TEXT NOT NULL,
    mime_type        TEXT,
    size_bytes       INTEGER,
    filename         TEXT,
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (message_id) REFERENCES mail_messages(id) ON DELETE CASCADE
);

CREATE INDEX idx_mail_attachments_message_id ON mail_attachments(message_id);
