//! Scheduled automation runner — polls automation_rules with
//! `trigger_kind = 'schedule'` and fires them when due. v1 supports
//! `trigger_config = {"interval_seconds": N}` only; cron expressions
//! can be added later by extending `is_due`.

use std::time::Duration;

use serde::Deserialize;
use sqlx::SqlitePool;
use uuid::Uuid;

const POLL_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize, Default)]
struct ScheduleConfig {
    interval_seconds: Option<i64>,
}

pub fn spawn(pool: SqlitePool) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(err) = tick(&pool).await {
                tracing::warn!(?err, "automation_runner tick failed");
            }
        }
    })
}

async fn tick(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!: Uuid",
           workspace_id AS "workspace_id!: Uuid",
           work_item_id AS "work_item_id!: Uuid",
           trigger_config,
           prompt_template_id AS "prompt_template_id?: Uuid",
           model_preset_id AS "model_preset_id?: Uuid",
           prompt_override,
           last_fired_at AS "last_fired_at?: chrono::DateTime<chrono::Utc>"
           FROM automation_rules
           WHERE enabled = 1 AND trigger_kind = 'schedule'"#,
    )
    .fetch_all(pool)
    .await?;

    let now = chrono::Utc::now();
    for r in rows {
        let cfg: ScheduleConfig = serde_json::from_str(&r.trigger_config).unwrap_or_default();
        let Some(interval) = cfg.interval_seconds else {
            continue;
        };
        if interval <= 0 {
            continue;
        }
        let due = match r.last_fired_at {
            Some(last) => (now - last).num_seconds() >= interval,
            None => true,
        };
        if !due {
            continue;
        }

        let prompt_text = if let Some(text) = r.prompt_override.clone() {
            text
        } else if let Some(tpl) = r.prompt_template_id {
            match sqlx::query_scalar!("SELECT body_text FROM prompt_templates WHERE id = ?1", tpl)
                .fetch_optional(pool)
                .await?
            {
                Some(t) => t,
                None => continue,
            }
        } else {
            continue;
        };

        let dispatch_id = Uuid::new_v4();
        let pending = "pending";
        sqlx::query!(
            r#"INSERT INTO dispatch_log
               (id, work_item_id, workspace_id, prompt_text, prompt_template_id, model_preset_id, status)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            dispatch_id,
            r.work_item_id,
            r.workspace_id,
            prompt_text,
            r.prompt_template_id,
            r.model_preset_id,
            pending,
        )
        .execute(pool)
        .await?;

        let now_str = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        sqlx::query!(
            "UPDATE automation_rules SET last_fired_at = ?1, updated_at = ?1 WHERE id = ?2",
            now_str,
            r.id,
        )
        .execute(pool)
        .await?;

        tracing::info!(rule_id = %r.id, dispatch_id = %dispatch_id, "fired scheduled automation");
    }

    Ok(())
}
