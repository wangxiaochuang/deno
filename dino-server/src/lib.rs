use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::{Host, Query, State},
    http::{request::Parts, Response},
    response::IntoResponse,
    routing::any,
    Router,
};
pub use config::ProjectConfig;
use dashmap::DashMap;
use engine::{JsWorker, Req};
use error::AppError;
use matchit::Match;
pub use router::{AppRouter, SwappableAppRouter};
use tokio::net::TcpListener;
use tracing::info;

mod config;
mod engine;
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

#[derive(Clone)]
pub struct TenentRouter {
    host: String,
    router: SwappableAppRouter,
}

impl TenentRouter {
    pub fn new(host: impl Into<String>, router: SwappableAppRouter) -> Self {
        Self {
            host: host.into(),
            router,
        }
    }
}

pub async fn start_server(port: u16, routers: Vec<TenentRouter>) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(addr).await?;
    info!("listening on {}", listener.local_addr()?);

    let map = DashMap::new();
    for TenentRouter { host, router } in routers {
        map.insert(host, router);
    }

    let state = AppState::new(map);
    let app = Router::new()
        .route("/*path", any(handler))
        .with_state(state);
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

async fn handler(
    State(state): State<AppState>,
    parts: Parts,
    Host(host): Host,
    Query(query): Query<HashMap<String, String>>,
    body: Option<Bytes>,
) -> Result<impl IntoResponse, AppError> {
    let router = get_router_by_host(host, state)?;
    let matched = router.match_it(parts.method.clone(), parts.uri.path())?;
    let req = assemble_req(&matched, &parts, query, body)?;
    let handler = matched.value;
    let worker = JsWorker::try_new(&router.code)?;
    let res = worker.run(handler, req)?;
    Ok(Response::from(res))
}

fn get_router_by_host(mut host: String, state: AppState) -> Result<AppRouter, AppError> {
    let _ = host.split_off(host.find(':').unwrap_or(host.len()));
    info!("host: {:?}", host);
    let router: AppRouter = state
        .routers
        .get(&host)
        .ok_or(AppError::HostNotFound(host))?
        .load();
    Ok(router)
}

fn assemble_req(
    matched: &Match<&str>,
    parts: &Parts,
    query: HashMap<String, String>,
    body: Option<Bytes>,
) -> Result<Req, AppError> {
    let params: HashMap<String, String> = matched
        .params
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let headers = parts
        .headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_string()))
        .collect();
    // let body = body.and_then(|v| String::from_utf8(v.into()).ok());
    let body = body.and_then(|v| String::from_utf8(v.into()).ok());

    let req = Req::builder()
        .method(parts.method.to_string())
        .url(parts.uri.to_string())
        .query(query)
        .params(params)
        .headers(headers)
        .body(body)
        .build();
    Ok(req)
}
