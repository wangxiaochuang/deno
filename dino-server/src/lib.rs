use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::{Host, Query, State},
    http::request::Parts,
    response::IntoResponse,
    routing::any,
    Json, Router,
};
pub use config::ProjectConfig;
use dashmap::DashMap;
use error::AppError;
pub use router::{AppRouter, SwappableAppRouter};
use serde_json::json;
use tokio::net::TcpListener;
use tracing::info;

mod config;
mod error;
mod router;

#[derive(Clone)]
pub struct AppState {
    routers: DashMap<String, SwappableAppRouter>,
}
impl AppState {
    pub fn new(routes: DashMap<String, SwappableAppRouter>) -> Self {
        Self { routers: routes }
    }
}
pub async fn start_server(
    port: u16,
    routes: DashMap<String, SwappableAppRouter>,
) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(addr).await?;
    info!("listening on {}", listener.local_addr()?);
    let state = AppState::new(routes);
    let app = Router::new()
        .route("/*path", any(handler))
        .with_state(state);
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

async fn handler(
    State(state): State<AppState>,
    parts: Parts,
    Host(mut host): Host,
    Query(query): Query<serde_json::Value>,
    body: Option<Bytes>,
) -> Result<impl IntoResponse, AppError> {
    info!("host: {:?}", host);
    let _ = host.split_off(host.find(':').unwrap_or(host.len()));
    let router: AppRouter = state
        .routers
        .get(&host)
        .ok_or(AppError::HostNotFound(host))?
        .load();
    let matched = router.match_it(parts.method, parts.uri.path())?;
    let handler = matched.value;
    let params: HashMap<String, String> = matched
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let body = match body {
        Some(body) => serde_json::from_slice(&body)?,
        None => serde_json::Value::Null,
    };
    Ok(Json(json!({
        "handler": handler,
        "params": params,
        "query": query,
        "body": body,
    })))
}
