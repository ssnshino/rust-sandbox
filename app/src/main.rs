mod game;
mod breakout;
mod scores;
mod dungeon_gen;
mod dungeon;

use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct AppState {
    pong:           Arc<game::AppState>,
    scores:         Arc<Mutex<scores::ScoreBoard>>,
    dungeon_scores: Arc<Mutex<scores::ScoreBoard>>,
}

#[tokio::main]
async fn main() {
    let state = AppState {
        pong:           Arc::new(game::AppState::new()),
        scores:         Arc::new(Mutex::new(scores::ScoreBoard::load("breakout_scores.json"))),
        dungeon_scores: Arc::new(Mutex::new(scores::ScoreBoard::load("dungeon_scores.json"))),
    };
    let app = Router::new()
        .route("/",                    get(index))
        .route("/pong",                get(pong))
        .route("/ws",                  get(ws_1p))
        .route("/ws2p",                get(ws_2p))
        .route("/breakout",            get(breakout_page))
        .route("/ws/breakout",         get(ws_breakout))
        .route("/api/breakout/scores", get(get_scores).post(post_score))
        .route("/dungeon",             get(dungeon_page))
        .route("/ws/dungeon",          get(ws_dungeon))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("rust-sandbox listening on :3000");
    axum::serve(listener, app).await.unwrap();
}

async fn index()        -> axum::response::Html<&'static str> { axum::response::Html(include_str!("index.html")) }
async fn pong()         -> axum::response::Html<&'static str> { axum::response::Html(include_str!("pong.html")) }
async fn breakout_page()-> axum::response::Html<&'static str> { axum::response::Html(include_str!("breakout.html")) }
async fn dungeon_page() -> axum::response::Html<&'static str> { axum::response::Html(include_str!("dungeon.html")) }

async fn ws_1p(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(game::run_1p)
}
async fn ws_2p(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| game::handle_2p(socket, s.pong))
}
async fn ws_breakout(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(breakout::run)
}
async fn ws_dungeon(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| dungeon::run(socket, s.dungeon_scores))
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
