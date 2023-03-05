use std::thread;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::{Response, StatusCode},
    response::{Html, Response as DefaultResponse},
    routing::get,
    Router, Server,
};
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::broadcast;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let capacity = thread::available_parallelism().unwrap().get();
    let (tx, _) = broadcast::channel::<Snapshot>(capacity);
    let app_state = AppState { tx: tx.clone() };

    let router = Router::new()
        .route("/", get(root_get))
        .route("/index.mjs", get(indexmjs_get))
        .route("/index.css", get(indexcss_get))
        .route("/realtime/cpus", get(cpus_ws))
        .with_state(app_state.clone());

    // update cpu in the background
    // could also use tokio::spawn_blocking + thread::sleep
    tokio::task::spawn(async move {
        let mut sys = System::new();
        loop {
            sys.refresh_cpu();
            let cpus: Vec<_> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
            let _ = tx.send(cpus); // returns error when there are no receivers --> ignoring this error
            tokio::time::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL).await;
        }
    });

    let addr = "0.0.0.0:7032".parse().unwrap();
    let server = Server::bind(&addr).serve(router.into_make_service());

    println!("Listening on {addr}");

    server.await.unwrap();
}

type Snapshot = Vec<f32>;
type HttpError = (StatusCode, String);
type Res<T> = Result<T, HttpError>;

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<Snapshot>,
}

async fn read_file_content(path: &str) -> Result<String, HttpError> {
    if let Ok(content) = tokio::fs::read_to_string(path).await {
        Ok(content)
    } else {
        Err((StatusCode::NOT_FOUND, format!("Resource {path} not found")))
    }
}

#[axum::debug_handler]
async fn root_get() -> Res<Html<String>> {
    read_file_content("web/index.html").await.map(Html)
}

async fn indexmjs_get() -> Res<Response<String>> {
    let js = read_file_content("web/index.mjs").await?;

    let res = Response::builder()
        .header("content-type", "application/javascript;charset=utf-8")
        .body(js)
        .unwrap();

    Ok(res)
}

async fn indexcss_get() -> Res<Response<String>> {
    let js = read_file_content("web/index.css").await?;

    let res = Response::builder()
        .header("content-type", "text/css;charset=utf-8")
        .body(js)
        .unwrap();

    Ok(res)
}

#[axum::debug_handler]
async fn cpus_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> DefaultResponse {
    ws.on_upgrade(|ws| async { realtime_cpus_stream(state, ws).await })
}

async fn realtime_cpus_stream(app_state: AppState, mut ws: WebSocket) {
    println!("Client connected");

    let mut rx = app_state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        let payload = serde_json::to_string(&msg).unwrap();
        if let Err(err) = ws.send(Message::Text(payload)).await {
            println!("Failed to send message to client. Client probably disconnected. {err}");
            break;
        }
    }
}
