//! Auto-approval supervisor — evaluates whether an agent action is
//! allowed under a workspace's policy without prompting a human.
//!
//! v1 implementation: deterministic, line-based policy. Each line is
//! either `allow: <substring>` or `deny: <substring>`. A deny match
//! short-circuits to `denied`; otherwise an allow match produces
//! `approved`; no match produces `escalated` (human required).
//!
//! The integration point for an LLM-backed supervisor is `evaluate` —
//! swap the body for a model call (using
//! `auto_approval_model_preset_id` to pick the executor) once the
//! safety surface is exercised end-to-end.

use db::models::safety::{AutoApprovalDecision, AutoApprovalRequest, SafetyConfig};

pub fn evaluate(config: &SafetyConfig, req: &AutoApprovalRequest) -> AutoApprovalDecision {
    if !config.auto_approval_enabled {
        return escalated("auto-approval disabled for workspace");
    }

    let Some(policy) = config.auto_approval_policy.as_deref() else {
        return escalated("no auto-approval policy configured");
    };

    let summary = req.action_summary.to_lowercase();

    let mut allow_matched: Option<String> = None;
    for raw in policy.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("deny:") {
            let needle = rest.trim().to_lowercase();
            if !needle.is_empty() && summary.contains(&needle) {
                return AutoApprovalDecision {
                    approved: false,
                    decision: "denied".into(),
                    reasoning: format!("matched deny rule: {needle}"),
                    decided_by: "policy".into(),
                };
            }
        } else if let Some(rest) = line.strip_prefix("allow:") {
            let needle = rest.trim().to_lowercase();
            if !needle.is_empty() && summary.contains(&needle) && allow_matched.is_none() {
                allow_matched = Some(needle);
            }
        }
    }

    if let Some(rule) = allow_matched {
        AutoApprovalDecision {
            approved: true,
            decision: "approved".into(),
            reasoning: format!("matched allow rule: {rule}"),
            decided_by: "policy".into(),
        }
    } else {
        escalated("no allow rule matched action summary")
    }
}

fn escalated(reason: &str) -> AutoApprovalDecision {
    AutoApprovalDecision {
        approved: false,
        decision: "escalated".into(),
        reasoning: reason.into(),
        decided_by: "policy".into(),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::*;

    fn cfg(enabled: bool, policy: Option<&str>) -> SafetyConfig {
        SafetyConfig {
            id: Uuid::new_v4(),
            workspace_id: None,
            scope: "global".into(),
            require_human_approval: true,
            max_concurrent_dispatch: 3,
            max_daily_dispatch: 50,
            cooldown_seconds: 30,
            auto_approval_enabled: enabled,
            auto_approval_policy: policy.map(str::to_string),
            auto_approval_model_preset_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn req(summary: &str) -> AutoApprovalRequest {
        AutoApprovalRequest {
            workspace_id: Uuid::new_v4(),
            action_kind: "dispatch".into(),
            action_summary: summary.into(),
        }
    }

    #[test]
    fn disabled_escalates() {
        let d = evaluate(&cfg(false, Some("allow: anything")), &req("run tests"));
        assert_eq!(d.decision, "escalated");
        assert!(!d.approved);
    }

    #[test]
    fn deny_short_circuits() {
        let d = evaluate(
            &cfg(true, Some("allow: run tests\ndeny: rm -rf")),
            &req("rm -rf /tmp"),
        );
        assert_eq!(d.decision, "denied");
    }

    #[test]
    fn allow_approves() {
        let d = evaluate(&cfg(true, Some("allow: run tests")), &req("run tests now"));
        assert_eq!(d.decision, "approved");
        assert!(d.approved);
    }

    #[test]
    fn no_match_escalates() {
        let d = evaluate(&cfg(true, Some("allow: deploy")), &req("run tests"));
        assert_eq!(d.decision, "escalated");
    }
}
