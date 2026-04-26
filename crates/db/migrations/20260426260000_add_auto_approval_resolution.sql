-- Phase 12: track human resolution of escalated auto-approval decisions.
-- Pending = decision='escalated' AND resolved_decision IS NULL.

ALTER TABLE auto_approval_log ADD COLUMN resolved_decision TEXT
    CHECK (resolved_decision IN ('approved','denied'));
ALTER TABLE auto_approval_log ADD COLUMN resolved_at TEXT;

CREATE INDEX idx_auto_approval_log_pending
    ON auto_approval_log(workspace_id, decision, resolved_decision);
