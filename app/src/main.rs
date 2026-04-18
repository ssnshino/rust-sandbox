mod game;
mod breakout;
mod scores;
mod dungeon_gen;
mod dungeon;
mod fighting;
mod truckers;

use axum::{
    extract::{ws::WebSocketUpgrade, Path, State},
    http::{header::{CACHE_CONTROL, CONTENT_TYPE}, StatusCode},
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
    fighting_scores:     Arc<Mutex<scores::ScoreBoard>>,
    truckers_scores:     Arc<Mutex<scores::ScoreBoard>>,
    pong_single_players: Arc<AtomicUsize>,
    pong_2p_players:     Arc<AtomicUsize>,
    breakout_players:    Arc<AtomicUsize>,
    dungeon_players:     Arc<AtomicUsize>,
    fighting_players:    Arc<AtomicUsize>,
    truckers_players:    Arc<AtomicUsize>,
    dungeon_room:        dungeon::SharedRoom,
    fight_room:          fighting::SharedFightRoom,
}

#[tokio::main]
async fn main() {
    let dungeon_scores = Arc::new(Mutex::new(scores::ScoreBoard::load("dungeon_scores.json")));
    let state = AppState {
        pong:                Arc::new(game::AppState::new()),
        scores:              Arc::new(Mutex::new(scores::ScoreBoard::load("breakout_scores.json"))),
        dungeon_scores:      Arc::clone(&dungeon_scores),
        fighting_scores:     Arc::new(Mutex::new(scores::ScoreBoard::load("fighting_scores.json"))),
        truckers_scores:     Arc::new(Mutex::new(scores::ScoreBoard::load("truckers_scores.json"))),
        pong_single_players: Arc::new(AtomicUsize::new(0)),
        pong_2p_players:     Arc::new(AtomicUsize::new(0)),
        breakout_players:    Arc::new(AtomicUsize::new(0)),
        dungeon_players:     Arc::new(AtomicUsize::new(0)),
        fighting_players:    Arc::new(AtomicUsize::new(0)),
        truckers_players:    Arc::new(AtomicUsize::new(0)),
        dungeon_room:        Arc::new(Mutex::new(dungeon::Room::new(dungeon_scores))),
        fight_room:          Arc::new(fighting::FightRoom::new()),
    };
    let app = Router::new()
        .route("/",                    get(index))
        .route("/pong",                get(pong))
        .route("/ws",                  get(ws_1p))
        .route("/ws2p",                get(ws_2p))
        .route("/breakout",            get(breakout_page))
        .route("/ws/breakout",         get(ws_breakout))
        .route("/api/breakout/scores", get(get_scores).post(post_score))
        .route("/api/status",          get(get_status))
        .route("/dungeon",             get(dungeon_page))
        .route("/ws/dungeon",          get(ws_dungeon))
        .route("/fighting",            get(fighting_page))
        .route("/ws/fighting",         get(ws_fighting))
        .route("/ws/fighting/2p",      get(ws_fighting_2p))
        .route("/truckers",            get(truckers_page))
        .route("/truckers.css",        get(truckers_css))
        .route("/truckers.js",         get(truckers_js))
        .route("/truckers/js/:name",   get(truckers_js_module))
        .route("/truckers.i18n.json",  get(truckers_i18n))
        .route("/truckers/refs/:name", get(truckers_ref))
        .route("/ws/truckers",         get(ws_truckers))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("rust-sandbox listening on :3000");
    axum::serve(listener, app).await.unwrap();
}

async fn index()         -> Html<&'static str> { Html(include_str!("index.html")) }
async fn pong()          -> Html<&'static str> { Html(include_str!("pong.html")) }
async fn breakout_page() -> Html<&'static str> { Html(include_str!("breakout.html")) }
async fn dungeon_page()  -> Html<String> {
    let build_hash = std::env::var("GIT_HASH").unwrap_or_else(|_| "dev".to_string());
    Html(include_str!("dungeon.html").replace("__BUILD_HASH__", &build_hash))
}
async fn fighting_page() -> Html<&'static str> { Html(include_str!("fighting.html")) }
async fn truckers_page() -> Html<&'static str> { Html(include_str!("truckers/truckers.html")) }
async fn truckers_css() -> impl IntoResponse {
    ([(CONTENT_TYPE, "text/css; charset=utf-8")], include_str!("truckers/truckers.css")).into_response()
}
async fn truckers_js() -> impl IntoResponse {
    ([(CONTENT_TYPE, "application/javascript; charset=utf-8")], include_str!("truckers/truckers.js")).into_response()
}
async fn truckers_js_module(Path(name): Path<String>) -> impl IntoResponse {
    let source = match name.as_str() {
        "main.js" => include_str!("truckers/js/main.js"),
        "state.js" => include_str!("truckers/js/state.js"),
        "i18n.js" => include_str!("truckers/js/i18n.js"),
        "ui.js" => include_str!("truckers/js/ui.js"),
        "render.js" => include_str!("truckers/js/render.js"),
        "audio.js" => include_str!("truckers/js/audio.js"),
        "input.js" => include_str!("truckers/js/input.js"),
        "ws.js" => include_str!("truckers/js/ws.js"),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    ([(CONTENT_TYPE, "application/javascript; charset=utf-8")], source).into_response()
}
async fn truckers_i18n() -> impl IntoResponse {
    ([(CONTENT_TYPE, "application/json; charset=utf-8")], include_str!("truckers/truckers.i18n.json")).into_response()
}
async fn truckers_ref(Path(name): Path<String>) -> impl IntoResponse {
    let bytes: &[u8] = match name.as_str() {
        "man.jpg" => include_bytes!("truckers/truckers_man.jpg"),
        "girl.jpg" => include_bytes!("truckers/truckers_girl.jpg"),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    (
        [
            (CONTENT_TYPE, "image/jpeg"),
            (CACHE_CONTROL, "public, max-age=3600"),
        ],
        bytes,
    ).into_response()
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
    ws.on_upgrade(move |socket| dungeon::run(socket, s.dungeon_scores, s.dungeon_players, s.dungeon_room))
}
async fn ws_fighting(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| fighting::run(socket, s.fighting_scores, s.fighting_players))
}
async fn ws_fighting_2p(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| fighting::run_2p(socket, s.fighting_scores, s.fighting_players, s.fight_room))
}
async fn ws_truckers(ws: WebSocketUpgrade, State(s): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| truckers::run(socket, s.truckers_scores, s.truckers_players))
}

async fn get_status(State(s): State<AppState>) -> impl IntoResponse {
    let pong_2p_waiting  = s.pong.waiting_count().await;
    let fight_2p_waiting = s.fight_room.waiting_count().await;
    Json(serde_json::json!({
        "pong_single":      s.pong_single_players.load(Ordering::Relaxed),
        "pong_2p":          s.pong_2p_players.load(Ordering::Relaxed),
        "pong_2p_waiting":  pong_2p_waiting,
        "breakout":         s.breakout_players.load(Ordering::Relaxed),
        "dungeon":          s.dungeon_players.load(Ordering::Relaxed),
        "fighting":         s.fighting_players.load(Ordering::Relaxed),
        "fighting_2p_waiting": fight_2p_waiting,
        "truckers":         s.truckers_players.load(Ordering::Relaxed),
    }))
}

async fn get_scores(State(s): State<AppState>) -> impl IntoResponse {
    let sb = s.scores.lock().unwrap();
    let list: Vec<_> = sb.list().to_vec();
    let min = sb.min_score();
    Json(serde_json::json!({ "scores": list, "min_score": min }))
}

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
