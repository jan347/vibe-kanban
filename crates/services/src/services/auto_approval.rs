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
use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";
const LLM_TIMEOUT_SECS: u64 = 12;

/// Override the supervisor's HTTP target. Setting either env var routes the
/// request through a Bifrost-style OpenAI-Chat-Completions proxy at
/// `<base>/v1/chat/completions` instead of calling Anthropic directly. The
/// proxy holds the upstream credentials (Bedrock, Vertex, etc.), so the
/// supervisor sends no auth header in proxy mode.
const PROXY_BASE_ENVS: &[&str] = &["BIFROST_BASE_URL", "LLM_BASE_URL"];
const SUPERVISOR_SYSTEM_PROMPT: &str = "You are an auto-approval supervisor for an AI agent platform. \
Decide whether the proposed agent action is safe to run unattended given the workspace policy. \
Reply with EXACTLY one line in the format `<decision>:<one-sentence reason>` where decision is one of: approved, denied, escalated. \
Use `approved` only for clearly safe, low-impact actions matching the policy. \
Use `denied` for actions the policy forbids or that are obviously destructive. \
Use `escalated` whenever you are unsure — the human will review.";

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

/// Runs the supervisor against the workspace's effective safety_config and
/// records the decision in `auto_approval_log`. The optional `approval_id`
/// links the log row to a pending [`crate::services::approvals::Approvals`]
/// request so the UI's resolve action can unblock the waiter.
pub async fn evaluate_and_log(
    pool: &SqlitePool,
    workspace_id: Uuid,
    action_kind: &str,
    action_summary: &str,
    approval_id: Option<&str>,
) -> Result<AutoApprovalDecision, sqlx::Error> {
    let config = sqlx::query_as!(
        SafetyConfig,
        r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id?: Uuid",
           scope, require_human_approval AS "require_human_approval!: bool",
           max_concurrent_dispatch AS "max_concurrent_dispatch!: i64",
           max_daily_dispatch AS "max_daily_dispatch!: i64",
           cooldown_seconds AS "cooldown_seconds!: i64",
           auto_approval_enabled AS "auto_approval_enabled!: bool",
           auto_approval_policy,
           auto_approval_model_preset_id AS "auto_approval_model_preset_id?: Uuid",
           created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
           updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
           FROM safety_config
           WHERE (scope = 'workspace' AND workspace_id = ?1) OR scope = 'global'
           ORDER BY CASE WHEN scope = 'workspace' THEN 0 ELSE 1 END
           LIMIT 1"#,
        workspace_id,
    )
    .fetch_one(pool)
    .await?;

    let req = AutoApprovalRequest {
        workspace_id,
        action_kind: action_kind.to_string(),
        action_summary: action_summary.to_string(),
    };

    // Honor the kill-switch BEFORE we spend tokens on the LLM. Without
    // this guard a workspace with auto-approval disabled but a model
    // preset still set could see the LLM return "approved" and unblock
    // the gate behind the user's back.
    let decision = if !config.auto_approval_enabled {
        evaluate(&config, &req)
    } else if let Some(preset_id) = config.auto_approval_model_preset_id {
        match resolve_supervisor_model(pool, preset_id).await {
            Ok(Some(model_id)) => match evaluate_with_llm(&config, &req, &model_id).await {
                Ok(d) => d,
                Err(err) => {
                    tracing::warn!(?err, "LLM supervisor failed; falling back to policy");
                    evaluate(&config, &req)
                }
            },
            Ok(None) => evaluate(&config, &req),
            Err(err) => {
                tracing::warn!(?err, "could not resolve supervisor preset; falling back");
                evaluate(&config, &req)
            }
        }
    } else {
        evaluate(&config, &req)
    };

    let id = Uuid::new_v4();
    sqlx::query!(
        r#"INSERT INTO auto_approval_log
           (id, workspace_id, action_kind, action_summary, decision, reasoning, decided_by, approval_id)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
        id,
        workspace_id,
        req.action_kind,
        req.action_summary,
        decision.decision,
        decision.reasoning,
        decision.decided_by,
        approval_id,
    )
    .execute(pool)
    .await?;

    Ok(decision)
}

fn escalated(reason: &str) -> AutoApprovalDecision {
    AutoApprovalDecision {
        approved: false,
        decision: "escalated".into(),
        reasoning: reason.into(),
        decided_by: "policy".into(),
    }
}

async fn resolve_supervisor_model(
    pool: &SqlitePool,
    preset_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    // With Bifrost in front, the model_id is just whatever slug the proxy
    // recognizes (e.g. `bedrock-claude-opus-4`, `vertex-gemini-2.5-pro`,
    // `claude-opus-4-7`). We don't filter by executor anymore.
    let row = sqlx::query!(
        r#"SELECT model_id FROM model_presets WHERE id = ?1"#,
        preset_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.model_id))
}

fn proxy_base_url() -> Option<String> {
    for name in PROXY_BASE_ENVS {
        if let Ok(v) = std::env::var(name) {
            let trimmed = v.trim().trim_end_matches('/').to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

#[derive(Debug)]
enum LlmError {
    NoApiKey,
    Http(reqwest::Error),
    Status(u16, String),
    Parse(String),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::NoApiKey => write!(
                f,
                "no LLM credentials: set BIFROST_BASE_URL/LLM_BASE_URL for proxy mode, or ANTHROPIC_API_KEY for direct mode"
            ),
            LlmError::Http(e) => write!(f, "http: {e}"),
            LlmError::Status(s, b) => write!(f, "llm {s}: {b}"),
            LlmError::Parse(m) => write!(f, "parse: {m}"),
        }
    }
}

impl std::error::Error for LlmError {}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    #[serde(default)]
    content: String,
}

async fn evaluate_with_llm(
    config: &SafetyConfig,
    req: &AutoApprovalRequest,
    model_id: &str,
) -> Result<AutoApprovalDecision, LlmError> {
    let policy = config
        .auto_approval_policy
        .as_deref()
        .unwrap_or("(no policy)");
    let user_prompt = format!(
        "Workspace policy:\n{policy}\n\nProposed action:\n  kind: {}\n  summary: {}\n\nDecide.",
        req.action_kind, req.action_summary
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(LLM_TIMEOUT_SECS))
        .build()
        .map_err(LlmError::Http)?;

    if let Some(base) = proxy_base_url() {
        // Bifrost-shaped OpenAI Chat Completions. The proxy owns upstream
        // auth (Bedrock, Vertex, Anthropic, OpenAI, …); we send no key.
        let url = format!("{base}/v1/chat/completions");
        let body = serde_json::json!({
            "model": model_id,
            "max_tokens": 200,
            "messages": [
                {"role": "system", "content": SUPERVISOR_SYSTEM_PROMPT},
                {"role": "user", "content": user_prompt},
            ],
        });
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(LlmError::Http)?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(LlmError::Http)?;
        if !status.is_success() {
            let snippet = String::from_utf8_lossy(&bytes).chars().take(400).collect();
            return Err(LlmError::Status(status.as_u16(), snippet));
        }
        let parsed: OpenAiResponse =
            serde_json::from_slice(&bytes).map_err(|e| LlmError::Parse(e.to_string()))?;
        let text = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content.trim().to_string())
            .filter(|t| !t.is_empty())
            .ok_or_else(|| LlmError::Parse("no choices in response".into()))?;
        return Ok(parse_supervisor_reply(&text));
    }

    // Direct Anthropic fallback.
    let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| LlmError::NoApiKey)?;
    let body = serde_json::json!({
        "model": model_id,
        "max_tokens": 200,
        "system": SUPERVISOR_SYSTEM_PROMPT,
        "messages": [{"role": "user", "content": user_prompt}],
    });
    let resp = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_API_VERSION)
        .json(&body)
        .send()
        .await
        .map_err(LlmError::Http)?;
    let status = resp.status();
    let bytes = resp.bytes().await.map_err(LlmError::Http)?;
    if !status.is_success() {
        let snippet = String::from_utf8_lossy(&bytes).chars().take(400).collect();
        return Err(LlmError::Status(status.as_u16(), snippet));
    }
    let parsed: AnthropicResponse =
        serde_json::from_slice(&bytes).map_err(|e| LlmError::Parse(e.to_string()))?;
    let text = parsed
        .content
        .iter()
        .find(|c| c.kind == "text")
        .map(|c| c.text.trim().to_string())
        .ok_or_else(|| LlmError::Parse("no text content".into()))?;

    Ok(parse_supervisor_reply(&text))
}

fn parse_supervisor_reply(text: &str) -> AutoApprovalDecision {
    // Expect "<decision>:<reason>". Match the decision PREFIX only, not a
    // substring of the whole line — a denial whose reason mentions
    // "approved" must not be parsed as approval.
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or(text);
    let (head, tail) = match line.split_once(':') {
        Some((h, t)) => (h.trim().to_lowercase(), t.trim().to_string()),
        None => (line.trim().to_lowercase(), String::new()),
    };
    let (decision_str, approved) = match head.as_str() {
        "approved" | "approve" => ("approved", true),
        "denied" | "deny" => ("denied", false),
        _ => ("escalated", false),
    };
    let reasoning = if tail.is_empty() {
        line.trim().to_string()
    } else {
        tail
    };
    AutoApprovalDecision {
        approved,
        decision: decision_str.to_string(),
        reasoning,
        decided_by: "model".to_string(),
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

    #[test]
    fn parse_reply_picks_decision() {
        let d = parse_supervisor_reply("approved: matches the read-only policy");
        assert_eq!(d.decision, "approved");
        assert!(d.approved);
        assert_eq!(d.decided_by, "model");
        assert!(d.reasoning.contains("read-only"));

        let d = parse_supervisor_reply("denied: writes to production data");
        assert_eq!(d.decision, "denied");
        assert!(!d.approved);

        let d = parse_supervisor_reply("escalated: not sure if this is safe");
        assert_eq!(d.decision, "escalated");

        let d = parse_supervisor_reply("the model is confused");
        assert_eq!(d.decision, "escalated");
    }

    #[test]
    fn parse_reply_rejects_substring_trap() {
        // The substring-based parser used to match this as approved.
        let d = parse_supervisor_reply("denied: not approved by policy");
        assert_eq!(d.decision, "denied");
        assert!(!d.approved);
        assert!(d.reasoning.contains("not approved"));
    }
}
