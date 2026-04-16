mod game;
mod breakout;
mod scores;
mod dungeon_gen;
mod dungeon;

use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
struct AppState {
    pong:                Arc<game::AppState>,
    scores:              Arc<Mutex<scores::ScoreBoard>>,
    dungeon_scores:      Arc<Mutex<scores::ScoreBoard>>,
    pong_single_players: Arc<AtomicUsize>,
    pong_2p_players:     Arc<AtomicUsize>,
    breakout_players:    Arc<AtomicUsize>,
    dungeon_players:     Arc<AtomicUsize>,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        pong:                Arc::new(game::AppState::new()),
        scores:              Arc::new(Mutex::new(scores::ScoreBoard::load("breakout_scores.json"))),
        dungeon_scores:      Arc::new(Mutex::new(scores::ScoreBoard::load("dungeon_scores.json"))),
        pong_single_players: Arc::new(AtomicUsize::new(0)),
        pong_2p_players:     Arc::new(AtomicUsize::new(0)),
        breakout_players:    Arc::new(AtomicUsize::new(0)),
        dungeon_players:     Arc::new(AtomicUsize::new(0)),
    };
    let app = Router::new()
        .route("/",                    get(index))
        .route("/pong",                get(pong))
        .route("/ws",                  get(ws_1p))
        .route("/ws2p",                get(ws_2p))
        .route("/breakout",            get(breakout_page))
        .route("/ws/breakout",         get(ws_breakout))
        .route("/api/breakout/scores", get(get_scores).post(post_score))
        .route("/api/status",           get(get_status))
        .route("/dungeon",              get(dungeon_page))
        .route("/ws/dungeon",          get(ws_dungeon))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("rust-sandbox listening on :3000");
    axum::serve(listener, app).await.unwrap();
}

async fn index()        -> axum::response::Html<&'static str> { axum::response::Html(include_str!("index.html")) }
async fn pong()         -> axum::response::Html<&'static str> { axum::response::Html(include_str!("pong.html")) }
async fn breakout_page()-> axum::response::Html<&'static str> { axum::response::Html(include_str!("breakout.html")) }
async fn dungeon_page() -> Html<String> {
    let build_hash = std::env::var("GIT_HASH").unwrap_or_else(|_| "2c2a0cb".to_string());
    Html(include_str!("dungeon.html").replace("__BUILD_HASH__", &build_hash))
}

async fn ws_1p(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| game::run_1p(socket, s.pong_single_players))
}
async fn ws_2p(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| game::handle_2p(socket, s.pong, s.pong_2p_players))
}
async fn ws_breakout(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| breakout::run(socket, s.breakout_players))
}
async fn ws_dungeon(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| dungeon::run(socket, s.dungeon_scores, s.dungeon_players))
}

async fn get_status(State(s): State<AppState>) -> impl IntoResponse {
    let pong_2p_waiting = s.pong.waiting_count().await;
    Json(serde_json::json!({
        "pong_single": s.pong_single_players.load(Ordering::Relaxed),
        "pong_2p": s.pong_2p_players.load(Ordering::Relaxed),
        "pong_2p_waiting": pong_2p_waiting,
        "breakout": s.breakout_players.load(Ordering::Relaxed),
        "dungeon": s.dungeon_players.load(Ordering::Relaxed),
    }))
}

// GET /api/breakout/scores
async fn get_scores(State(s): State<AppState>) -> impl IntoResponse {
    let sb = s.scores.lock().unwrap();
    let list: Vec<_> = sb.list().to_vec();
    let min = sb.min_score();
    Json(serde_json::json!({ "scores": list, "min_score": min }))
}

// POST /api/breakout/scores
#[derive(Deserialize)]
struct SubmitPayload { name: String, score: u32 }

async fn post_score(State(s): State<AppState>, Json(p): Json<SubmitPayload>) -> impl IntoResponse {
    let mut sb = s.scores.lock().unwrap();
    let rank = sb.add(p.name, p.score);
    let list: Vec<_> = sb.list().to_vec();
    let min = sb.min_score();
    let rank_val = match rank {
        Some(r) => serde_json::Value::Number(r.into()),
        None    => serde_json::Value::Null,
    };
    Json(serde_json::json!({ "rank": rank_val, "scores": list, "min_score": min }))
}
