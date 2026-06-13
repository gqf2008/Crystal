/// HTTP 管理服务，对应 C# Utils/HttpServer.cs
/// 提供新账号注册和广播的 REST 接口

use axum::{Router, routing::get, extract::Query, response::Json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type SharedState = Arc<Mutex<HttpState>>;

pub struct HttpState {
    pub game_name: String,
    pub trusted_ips: Vec<String>,
    pub account_creator: Option<Box<dyn Fn(String, String, String, String, String, String, String) -> String + Send + Sync>>,
    pub broadcaster: Option<Box<dyn Fn(String) + Send + Sync>>,
    pub name_list_writer: Option<Box<dyn Fn(String, String) + Send + Sync>>,
}

/// 启动 HTTP 服务。
///
/// Phase 1.1: 不再 unwrap()。bind/serve 失败时 log error 并返回,
/// 让调用方决定是否致命(而不是 panic 整个进程)。
pub async fn start_http_server(state: SharedState, port: u16) {
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/newaccount", get(new_account_handler))
        .route("/addnamelist", get(add_name_list_handler))
        .route("/broadcast", get(broadcast_handler))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("HTTP admin server failed to bind {}: {}", addr, e);
            return;
        }
    };
    tracing::info!("HTTP admin server listening on {}", addr);
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("HTTP admin server error: {}", e);
    }
}

// Handlers
async fn root_handler(Query(params): Query<HashMap<String, String>>, state: axum::extract::State<SharedState>) -> String {
    let ip = params.get("ip").cloned().unwrap_or_default();
    let s = state.lock().await;
    format!("{}:{}", s.game_name, ip)
}

async fn new_account_handler(
    Query(params): Query<HashMap<String, String>>,
    state: axum::extract::State<SharedState>,
) -> Json<HashMap<String, String>> {
    let s = state.lock().await;
    let mut result = HashMap::new();
    if let Some(creator) = &s.account_creator {
        let id = params.get("id").cloned().unwrap_or_default();
        let psd = params.get("psd").cloned().unwrap_or_default();
        let email = params.get("email").cloned().unwrap_or_default();
        let name = params.get("name").cloned().unwrap_or_default();
        let question = params.get("question").cloned().unwrap_or_default();
        let answer = params.get("answer").cloned().unwrap_or_default();
        let ip = params.get("ip").cloned().unwrap_or_default();
        let resp = creator(id, psd, email, name, question, answer, ip);
        result.insert("result".into(), resp);
    } else {
        result.insert("result".into(), "not_configured".into());
    }
    Json(result)
}

async fn add_name_list_handler(
    Query(params): Query<HashMap<String, String>>,
    state: axum::extract::State<SharedState>,
) -> String {
    let s = state.lock().await;
    if let Some(writer) = &s.name_list_writer {
        let id = params.get("id").cloned().unwrap_or_default();
        let file_name = params.get("fileName").cloned().unwrap_or_default();
        writer(id, file_name);
        "ok".into()
    } else {
        "not_configured".into()
    }
}

async fn broadcast_handler(
    Query(params): Query<HashMap<String, String>>,
    state: axum::extract::State<SharedState>,
) -> String {
    let s = state.lock().await;
    if let Some(broadcaster) = &s.broadcaster {
        let msg = params.get("msg").cloned().unwrap_or_default();
        broadcaster(msg);
        "ok".into()
    } else {
        "not_configured".into()
    }
}
