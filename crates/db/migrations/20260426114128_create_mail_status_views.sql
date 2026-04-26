CREATE VIEW workspace_awaiting_reply AS
SELECT
    m.sender_workspace_id AS workspace_id,
    m.id AS message_id,
    m.thread_id,
    m.created_at
FROM mail_messages m
WHERE m.requires_response = 1
  AND m.sender_workspace_id IS NOT NULL
  AND EXISTS (
      SELECT 1
      FROM mail_recipients r
      WHERE r.message_id = m.id
        AND r.responded_at IS NULL
  )
  AND NOT EXISTS (
      SELECT 1
      FROM mail_recipients r
      WHERE r.message_id = m.id
        AND r.responded_at IS NOT NULL
        AND r.response_value_json IS NOT NULL
  );

CREATE VIEW workspace_inbox_unread AS
SELECT
    r.recipient_workspace_id AS workspace_id,
    r.id AS recipient_id,
    r.message_id,
    m.thread_id,
    m.created_at
FROM mail_recipients r
JOIN mail_messages m ON m.id = r.message_id
WHERE r.recipient_kind = 'workspace'
  AND r.recipient_workspace_id IS NOT NULL
  AND r.read_at IS NULL;
