-- Phase 13: link auto_approval_log entries to the in-memory approvals
-- service. When the supervisor escalates a tool call, we insert a log
-- row with approval_id set to the Approvals::request id. Resolving
-- the log entry then routes back through Approvals::respond to
-- unblock the executor that's still waiting on the gate.

ALTER TABLE auto_approval_log ADD COLUMN approval_id TEXT;

CREATE INDEX idx_auto_approval_log_approval_id ON auto_approval_log(approval_id);
