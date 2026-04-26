CREATE TABLE mail_threads (
    id           BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    work_item_id BLOB,
    subject      TEXT NOT NULL,
    kind         TEXT NOT NULL
                 CHECK (kind IN ('agent_human','agent_agent','broadcast')),
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    closed_at    TEXT
);

CREATE INDEX idx_mail_threads_work_item_id ON mail_threads(work_item_id);
CREATE INDEX idx_mail_threads_kind ON mail_threads(kind);
CREATE INDEX idx_mail_threads_created_at ON mail_threads(created_at);

CREATE TABLE mail_messages (
    id                          BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    thread_id                   BLOB NOT NULL,
    sender_kind                 TEXT NOT NULL
                                CHECK (sender_kind IN ('workspace','human')),
    sender_workspace_id         BLOB,
    sender_execution_process_id BLOB,
    body                        TEXT NOT NULL,
    requires_response           INTEGER NOT NULL DEFAULT 0
                                CHECK (requires_response IN (0, 1)),
    response_kind               TEXT
                                CHECK (
                                    response_kind IS NULL
                                    OR response_kind IN ('options','free_text','file','approval','none')
                                ),
    response_options_json       TEXT,
    expires_at                  TEXT,
    idempotency_key             TEXT,
    created_at                  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    CHECK ((sender_kind = 'workspace') = (sender_workspace_id IS NOT NULL)),
    CHECK (sender_execution_process_id IS NULL OR sender_kind = 'workspace'),
    FOREIGN KEY (thread_id) REFERENCES mail_threads(id) ON DELETE CASCADE,
    FOREIGN KEY (sender_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (sender_execution_process_id) REFERENCES execution_processes(id) ON DELETE CASCADE
);

CREATE INDEX idx_mail_messages_thread_id ON mail_messages(thread_id);
CREATE INDEX idx_mail_messages_sender_workspace_id ON mail_messages(sender_workspace_id);
CREATE INDEX idx_mail_messages_sender_execution_process_id
    ON mail_messages(sender_execution_process_id);
CREATE INDEX idx_mail_messages_created_at ON mail_messages(created_at);
CREATE UNIQUE INDEX idx_mail_messages_idempotency_key_unique
    ON mail_messages(idempotency_key)
    WHERE idempotency_key IS NOT NULL;

CREATE TABLE mail_recipients (
    id                     BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    message_id             BLOB NOT NULL,
    recipient_kind         TEXT NOT NULL
                           CHECK (recipient_kind IN ('workspace','human')),
    recipient_workspace_id BLOB,
    read_at                TEXT,
    responded_at           TEXT,
    response_value_json    TEXT,
    CHECK ((recipient_kind = 'workspace') = (recipient_workspace_id IS NOT NULL)),
    FOREIGN KEY (message_id) REFERENCES mail_messages(id) ON DELETE CASCADE,
    FOREIGN KEY (recipient_workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_mail_recipients_message_id ON mail_recipients(message_id);
CREATE INDEX idx_mail_recipients_recipient_workspace_id
    ON mail_recipients(recipient_workspace_id);
CREATE INDEX idx_mail_recipients_read_at ON mail_recipients(read_at);
CREATE INDEX idx_mail_recipients_responded_at ON mail_recipients(responded_at);
CREATE UNIQUE INDEX idx_mail_recipients_workspace_unique
    ON mail_recipients(message_id, recipient_workspace_id)
    WHERE recipient_kind = 'workspace';
CREATE UNIQUE INDEX idx_mail_recipients_human_unique
    ON mail_recipients(message_id)
    WHERE recipient_kind = 'human';
