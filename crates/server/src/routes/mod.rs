use axum::{
    Router,
    routing::{IntoMakeService, get},
};
use tower_http::{compression::CompressionLayer, validate_request::ValidateRequestHeaderLayer};

use crate::{DeploymentImpl, middleware};

pub mod approvals;
pub mod artifacts;
pub mod attachments;
pub mod automation;
pub mod config;
pub mod containers;
pub mod dispatch;
pub mod events;
pub mod execution_processes;
pub mod filesystem;
pub mod frontend;
pub mod health;
pub mod mail;
pub mod model_presets;
pub mod prompt_templates;
pub mod releases;
pub mod repo;
pub mod repo_bundles;
pub mod safety;
pub mod scratch;
pub mod search;
pub mod sessions;
pub mod tags;
pub mod terminal;
pub mod work_items;
pub mod workspaces;

pub fn router(deployment: DeploymentImpl) -> IntoMakeService<Router> {
    let api_routes = Router::new()
        .route("/health", get(health::health_check))
        .merge(config::router())
        .merge(containers::router(&deployment))
        .merge(workspaces::router(&deployment))
        .merge(execution_processes::router(&deployment))
        .merge(tags::router(&deployment))
        .merge(filesystem::router())
        .merge(repo::router())
        .merge(events::router(&deployment))
        .merge(approvals::router())
        .merge(mail::router(&deployment))
        .merge(model_presets::router())
        .merge(prompt_templates::router())
        .merge(repo_bundles::router())
        .merge(work_items::router())
        .merge(artifacts::router())
        .merge(dispatch::router())
        .merge(safety::router())
        .merge(automation::router())
        .merge(scratch::router(&deployment))
        .merge(search::router(&deployment))
        .merge(releases::router())
        .merge(sessions::router(&deployment))
        .merge(terminal::router())
        .nest("/attachments", attachments::routes())
        .layer(axum::middleware::from_fn(middleware::log_server_errors))
        .layer(ValidateRequestHeaderLayer::custom(
            middleware::validate_origin,
        ))
        .with_state(deployment);

    Router::new()
        .route("/", get(frontend::serve_frontend_root))
        .route("/{*path}", get(frontend::serve_frontend))
        .nest("/api", api_routes)
        .layer(CompressionLayer::new())
        .into_make_service()
}
