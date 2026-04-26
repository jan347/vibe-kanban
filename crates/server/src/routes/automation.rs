use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::models::automation::{
    AutomationRule, CreateAutomationRule, FireAutomationResult, UpdateAutomationRule,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Deserialize)]
pub struct AutomationFilter {
    pub workspace_id: Option<Uuid>,
    pub work_item_id: Option<Uuid>,
    pub enabled: Option<bool>,
}

pub async fn list_rules(
    State(deployment): State<DeploymentImpl>,
    Query(filter): Query<AutomationFilter>,
) -> Result<ResponseJson<ApiResponse<Vec<AutomationRule>>>, ApiError> {
    let pool = &deployment.db().pool;
    let rows = match (filter.workspace_id, filter.work_item_id, filter.enabled) {
        (Some(ws), _, _) => {
            sqlx::query_as!(
                AutomationRule,
                r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id!: Uuid",
                   work_item_id AS "work_item_id!: Uuid", name,
                   trigger_kind, trigger_config,
                   prompt_template_id AS "prompt_template_id?: Uuid",
                   model_preset_id AS "model_preset_id?: Uuid",
                   prompt_override,
                   enabled AS "enabled!: bool",
                   last_fired_at AS "last_fired_at?: chrono::DateTime<chrono::Utc>",
                   created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
                   updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
                   FROM automation_rules WHERE workspace_id = ?1
                   ORDER BY created_at DESC"#,
                ws
            )
            .fetch_all(pool)
            .await?
        }
        (None, Some(wi), _) => {
            sqlx::query_as!(
                AutomationRule,
                r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id!: Uuid",
                   work_item_id AS "work_item_id!: Uuid", name,
                   trigger_kind, trigger_config,
                   prompt_template_id AS "prompt_template_id?: Uuid",
                   model_preset_id AS "model_preset_id?: Uuid",
                   prompt_override,
                   enabled AS "enabled!: bool",
                   last_fired_at AS "last_fired_at?: chrono::DateTime<chrono::Utc>",
                   created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
                   updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
                   FROM automation_rules WHERE work_item_id = ?1
                   ORDER BY created_at DESC"#,
                wi
            )
            .fetch_all(pool)
            .await?
        }
        (None, None, _) => {
            sqlx::query_as!(
                AutomationRule,
                r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id!: Uuid",
                   work_item_id AS "work_item_id!: Uuid", name,
                   trigger_kind, trigger_config,
                   prompt_template_id AS "prompt_template_id?: Uuid",
                   model_preset_id AS "model_preset_id?: Uuid",
                   prompt_override,
                   enabled AS "enabled!: bool",
                   last_fired_at AS "last_fired_at?: chrono::DateTime<chrono::Utc>",
                   created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
                   updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
                   FROM automation_rules ORDER BY created_at DESC"#
            )
            .fetch_all(pool)
            .await?
        }
    };
    Ok(ResponseJson(ApiResponse::success(rows)))
}

pub async fn create_rule(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(payload): ResponseJson<CreateAutomationRule>,
) -> Result<ResponseJson<ApiResponse<AutomationRule>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();
    let trigger_config = payload
        .trigger_config
        .clone()
        .unwrap_or_else(|| "{}".to_string());
    sqlx::query!(
        r#"INSERT INTO automation_rules
           (id, workspace_id, work_item_id, name, trigger_kind, trigger_config,
            prompt_template_id, model_preset_id, prompt_override)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
        id,
        payload.workspace_id,
        payload.work_item_id,
        payload.name,
        payload.trigger_kind,
        trigger_config,
        payload.prompt_template_id,
        payload.model_preset_id,
        payload.prompt_override,
    )
    .execute(pool)
    .await?;
    let row = fetch_rule(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn get_rule(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<AutomationRule>>, ApiError> {
    let pool = &deployment.db().pool;
    let row = fetch_rule(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn update_rule(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    ResponseJson(payload): ResponseJson<UpdateAutomationRule>,
) -> Result<ResponseJson<ApiResponse<AutomationRule>>, ApiError> {
    let pool = &deployment.db().pool;
    let now_str = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();
    sqlx::query!(
        r#"UPDATE automation_rules SET
           name = COALESCE(?1, name),
           trigger_kind = COALESCE(?2, trigger_kind),
           trigger_config = COALESCE(?3, trigger_config),
           prompt_template_id = COALESCE(?4, prompt_template_id),
           model_preset_id = COALESCE(?5, model_preset_id),
           prompt_override = COALESCE(?6, prompt_override),
           enabled = COALESCE(?7, enabled),
           updated_at = ?8
           WHERE id = ?9"#,
        payload.name,
        payload.trigger_kind,
        payload.trigger_config,
        payload.prompt_template_id,
        payload.model_preset_id,
        payload.prompt_override,
        payload.enabled,
        now_str,
        id,
    )
    .execute(pool)
    .await?;
    let row = fetch_rule(pool, id).await?;
    Ok(ResponseJson(ApiResponse::success(row)))
}

pub async fn delete_rule(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    sqlx::query!("DELETE FROM automation_rules WHERE id = ?1", id)
        .execute(pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn fire_rule(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<ResponseJson<ApiResponse<FireAutomationResult>>, ApiError> {
    let pool = &deployment.db().pool;
    let rule = fetch_rule(pool, id).await?;
    if !rule.enabled {
        return Err(ApiError::BadRequest("rule is disabled".into()));
    }

    let prompt_text = if let Some(text) = rule.prompt_override.clone() {
        text
    } else if let Some(tpl_id) = rule.prompt_template_id {
        sqlx::query_scalar!(
            r#"SELECT body_text FROM prompt_templates WHERE id = ?1"#,
            tpl_id
        )
        .fetch_one(pool)
        .await?
    } else {
        return Err(ApiError::BadRequest(
            "rule has neither prompt_override nor prompt_template_id".into(),
        ));
    };

    let dispatch_id = Uuid::new_v4();
    let pending = "pending";
    sqlx::query!(
        r#"INSERT INTO dispatch_log
           (id, work_item_id, workspace_id, prompt_text, prompt_template_id, model_preset_id, status)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
        dispatch_id,
        rule.work_item_id,
        rule.workspace_id,
        prompt_text,
        rule.prompt_template_id,
        rule.model_preset_id,
        pending,
    )
    .execute(pool)
    .await?;

    let now = chrono::Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    sqlx::query!(
        "UPDATE automation_rules SET last_fired_at = ?1, updated_at = ?1 WHERE id = ?2",
        now_str,
        id,
    )
    .execute(pool)
    .await?;

    Ok(ResponseJson(ApiResponse::success(FireAutomationResult {
        rule_id: id,
        dispatch_id,
        fired_at: now,
    })))
}

async fn fetch_rule(pool: &sqlx::SqlitePool, id: Uuid) -> Result<AutomationRule, ApiError> {
    sqlx::query_as!(
        AutomationRule,
        r#"SELECT id AS "id!: Uuid", workspace_id AS "workspace_id!: Uuid",
           work_item_id AS "work_item_id!: Uuid", name,
           trigger_kind, trigger_config,
           prompt_template_id AS "prompt_template_id?: Uuid",
           model_preset_id AS "model_preset_id?: Uuid",
           prompt_override,
           enabled AS "enabled!: bool",
           last_fired_at AS "last_fired_at?: chrono::DateTime<chrono::Utc>",
           created_at AS "created_at!: chrono::DateTime<chrono::Utc>",
           updated_at AS "updated_at!: chrono::DateTime<chrono::Utc>"
           FROM automation_rules WHERE id = ?1"#,
        id
    )
    .fetch_one(pool)
    .await
    .map_err(ApiError::from)
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/automations", get(list_rules).post(create_rule))
        .route(
            "/automations/{id}",
            get(get_rule).patch(update_rule).delete(delete_rule),
        )
        .route("/automations/{id}/fire", post(fire_rule))
}
