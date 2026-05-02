#![recursion_limit = "256"]
mod game;
mod breakout;
mod scores;
mod dungeon_gen;
mod dungeon;
mod fighting;
mod truckers;

use axum::{
    extract::{ws::WebSocketUpgrade, Path, Query, State},
    http::{
        header::{CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_AIRRACE_GHOSTS: usize = 100;
const AIRRACE_ROUNDS_JSON: &str = include_str!("airrace/airrace_rounds.json");

#[derive(Serialize, Deserialize, Clone)]
struct AirraceGhost {
    name: String,
    round: u32,
    time_ms: u32,
    samples: Vec<serde_json::Value>,
    created_at: String,
    #[serde(default)]
    status: Option<String>,
}

struct AirraceGhostStore {
    file: &'static str,
    ghosts: Vec<AirraceGhost>,
}

impl AirraceGhostStore {
    fn load(file: &'static str) -> Self {
        let ghosts = std::fs::read_to_string(file)
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<AirraceGhost>>(&s).ok())
            .unwrap_or_default();
        Self { file, ghosts }
    }

    fn list(&self, round: Option<u32>) -> Vec<AirraceGhost> {
        self.ghosts
            .iter()
            .filter(|g| round.map_or(true, |r| g.round == r))
            .cloned()
            .collect()
    }

    fn add(&mut self, mut ghost: AirraceGhost) {
        ghost.name = ghost.name.chars().take(20).collect::<String>().trim().to_string();
        if ghost.name.is_empty() {
            ghost.name = "PILOT".to_string();
        }
        ghost.samples.truncate(1200);
        self.ghosts.insert(0, ghost);
        self.ghosts.truncate(MAX_AIRRACE_GHOSTS);
        self.save();
    }

    fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.ghosts) {
            let _ = std::fs::write(self.file, json);
        }
    }
}

#[derive(Clone)]
struct AppState {
    pong:                Arc<game::AppState>,
    scores:              Arc<Mutex<scores::ScoreBoard>>,
    dungeon_scores:      Arc<Mutex<scores::ScoreBoard>>,
    fighting_scores:     Arc<Mutex<scores::ScoreBoard>>,
    truckers_scores:     Arc<Mutex<scores::ScoreBoard>>,
    airrace_scores:      Arc<Mutex<scores::ScoreBoard>>,
    airrace_ghosts:      Arc<Mutex<AirraceGhostStore>>,
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
        airrace_scores:      Arc::new(Mutex::new(scores::ScoreBoard::load("airrace_scores.json"))),
        airrace_ghosts:      Arc::new(Mutex::new(AirraceGhostStore::load("airrace_ghosts.json"))),
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
        .route("/airrace",             get(airrace_page))
        .route("/airrace.css",         get(airrace_css))
        .route("/airrace.js",          get(airrace_js))
        .route("/airrace/js/:name",    get(airrace_js_module))
        .route("/airrace.webmanifest", get(airrace_manifest))
        .route("/airrace3d",           get(airrace3d_index))
        .route("/airrace3d/",          get(airrace3d_index))
        .route("/airrace3d/StreamingAssets/*path", get(airrace3d_streaming_file))
        .route("/airrace3d/:name",     get(airrace3d_root_file))
        .route("/airrace3d/Build/:name", get(airrace3d_build_file))
        .route("/airrace3d/TemplateData/:name", get(airrace3d_template_file))
        .route("/api/airrace3d/course-catalog", get(airrace3d_course_catalog))
        .route("/api/airrace3d/round-index", get(airrace3d_round_index))
        .route("/api/airrace3d/course/:round", get(airrace3d_course_round))
        .route("/api/airrace3d/field-catalog", get(airrace3d_field_catalog))
        .route("/api/airrace3d/world-object-catalog", get(airrace3d_world_object_catalog))
        .route("/api/airrace3d/round-world-catalog", get(airrace3d_round_world_catalog))
        .route("/api/airrace3d/aircraft-catalog", get(airrace3d_aircraft_catalog))
        .route("/api/airrace3d/aircraft-models", get(airrace3d_aircraft_models))
        .route("/api/airrace3d/aircraft-prefab-catalog", get(airrace3d_aircraft_prefab_catalog))
        .route("/api/airrace/round/:round", get(get_airrace_round_bundle))
        .route("/api/airrace/scores",  get(get_airrace_scores).post(post_airrace_score))
        .route("/api/airrace/ghosts",  get(get_airrace_ghosts).post(post_airrace_ghost))
        .route("/favicon.ico",         get(favicon))
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
async fn airrace_page()  -> Html<&'static str> { Html(include_str!("airrace/airrace.html")) }
async fn airrace_css() -> impl IntoResponse {
    ([(CONTENT_TYPE, "text/css; charset=utf-8")], include_str!("airrace/airrace.css")).into_response()
}
async fn airrace_js() -> impl IntoResponse {
    ([(CONTENT_TYPE, "application/javascript; charset=utf-8")], include_str!("airrace/airrace.js")).into_response()
}
async fn airrace_js_module(Path(name): Path<String>) -> impl IntoResponse {
    let source = match name.as_str() {
        "config.js" => include_str!("airrace/js/config.js"),
        "course.js" => include_str!("airrace/js/course.js"),
        "aircraft.js" => include_str!("airrace/js/aircraft.js"),
        "game.js" => include_str!("airrace/js/game.js"),
        "render.js" => include_str!("airrace/js/render.js"),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    ([(CONTENT_TYPE, "application/javascript; charset=utf-8")], source).into_response()
}
async fn airrace_manifest() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "application/manifest+json; charset=utf-8"),
            (CACHE_CONTROL, "public, max-age=3600"),
        ],
        include_str!("airrace/airrace.webmanifest"),
    ).into_response()
}
async fn airrace3d_index() -> impl IntoResponse {
    serve_airrace3d_file(PathBuf::from("index.html")).await
}
async fn airrace3d_root_file(Path(name): Path<String>) -> impl IntoResponse {
    serve_airrace3d_file(PathBuf::from(name)).await
}
async fn airrace3d_streaming_file(Path(path): Path<String>) -> impl IntoResponse {
    serve_airrace3d_file(PathBuf::from("StreamingAssets").join(path)).await
}
async fn airrace3d_build_file(Path(name): Path<String>) -> impl IntoResponse {
    serve_airrace3d_file(PathBuf::from("Build").join(name)).await
}
async fn airrace3d_template_file(Path(name): Path<String>) -> impl IntoResponse {
    serve_airrace3d_file(PathBuf::from("TemplateData").join(name)).await
}
async fn airrace3d_course_catalog() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "application/json; charset=utf-8"),
            (CACHE_CONTROL, "no-store"),
        ],
        AIRRACE_ROUNDS_JSON,
    ).into_response()
}
async fn airrace3d_round_index() -> impl IntoResponse {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(AIRRACE_ROUNDS_JSON) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Some(map) = root.as_object() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let mut rounds: Vec<u32> = map.keys().filter_map(|k| k.parse::<u32>().ok()).collect();
    rounds.sort_unstable();
    let final_round = rounds.last().copied().unwrap_or(1);
    let payload = serde_json::json!({
        "finalRound": final_round,
        "rounds": rounds,
    });
    (
        [
            (CONTENT_TYPE, "application/json; charset=utf-8"),
            (CACHE_CONTROL, "no-store"),
        ],
        payload.to_string(),
    ).into_response()
}
async fn airrace3d_course_round(Path(round): Path<u32>) -> impl IntoResponse {
    let key = round.to_string();
    let Ok(root) = serde_json::from_str::<serde_json::Value>(AIRRACE_ROUNDS_JSON) else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Some(course) = root.get(key.as_str()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    (
        [
            (CONTENT_TYPE, "application/json; charset=utf-8"),
            (CACHE_CONTROL, "no-store"),
        ],
        course.to_string(),
    ).into_response()
}
async fn airrace3d_field_catalog() -> impl IntoResponse {
    let fallback = r#"{"version":"airrace3d-field-fallback-1","globalField":{"minX":-16000,"maxX":16000,"minZ":-16000,"maxZ":16000,"margin":640},"overrides":[]}"#;
    let body = std::fs::read_to_string("src/airrace3d/StreamingAssets/AirRace/airrace3d_field.json")
        .unwrap_or_else(|_| fallback.to_string());
    (
        [
            (CONTENT_TYPE, "application/json; charset=utf-8"),
            (CACHE_CONTROL, "no-store"),
        ],
        body,
    ).into_response()
}
async fn airrace3d_world_object_catalog() -> impl IntoResponse {
    let fallback = r#"{"version":"airrace3d-world-objects-fallback-1","objects":[]}"#;
    let body = std::fs::read_to_string("src/airrace3d/StreamingAssets/AirRace/world_object_catalog.json")
        .unwrap_or_else(|_| fallback.to_string());
    (
        [
            (CONTENT_TYPE, "application/json; charset=utf-8"),
            (CACHE_CONTROL, "no-store"),
        ],
        body,
    ).into_response()
}
async fn airrace3d_round_world_catalog() -> impl IntoResponse {
    let fallback = r#"{"version":"airrace3d-round-world-fallback-1","worlds":[]}"#;
    let body = std::fs::read_to_string("src/airrace3d/StreamingAssets/AirRace/round_world_catalog.json")
        .unwrap_or_else(|_| fallback.to_string());
    (
        [
            (CONTENT_TYPE, "application/json; charset=utf-8"),
            (CACHE_CONTROL, "no-store"),
        ],
        body,
    ).into_response()
}
async fn airrace3d_aircraft_catalog() -> impl IntoResponse {
    let fallback = r#"{"version":"airrace3d-aircraft-fallback-1","defaultAircraftId":"skylancer","aircrafts":[{"id":"skylancer","nameJa":"スカイランサー","nameEn":"Skylancer","summaryJa":"標準機","summaryEn":"Standard","handling":0.5,"maxSpeed":0.5,"boostMultiplier":1.0,"rollResponse":1.0,"pitchResponse":1.0}]}"#;
    let body = std::fs::read_to_string("src/airrace3d/StreamingAssets/AirRace/aircraft_catalog.json")
        .unwrap_or_else(|_| fallback.to_string());
    (
        [
            (CONTENT_TYPE, "application/json; charset=utf-8"),
            (CACHE_CONTROL, "no-store"),
        ],
        body,
    ).into_response()
}
async fn airrace3d_aircraft_models() -> impl IntoResponse {
    let fallback = r#"{"version":"airrace3d-aircraft-models-fallback-1","models":[]}"#;
    let body = std::fs::read_to_string("src/airrace3d/StreamingAssets/AirRace/aircraft_models.json")
        .unwrap_or_else(|_| fallback.to_string());
    (
        [
            (CONTENT_TYPE, "application/json; charset=utf-8"),
            (CACHE_CONTROL, "no-store"),
        ],
        body,
    ).into_response()
}
async fn airrace3d_aircraft_prefab_catalog() -> impl IntoResponse {
    let fallback = r#"{"version":"airrace3d-aircraft-prefabs-fallback-1","prefabs":[]}"#;
    let body = std::fs::read_to_string("src/airrace3d/StreamingAssets/AirRace/aircraft_prefab_catalog.json")
        .unwrap_or_else(|_| fallback.to_string());
    (
        [
            (CONTENT_TYPE, "application/json; charset=utf-8"),
            (CACHE_CONTROL, "no-store"),
        ],
        body,
    ).into_response()
}
async fn serve_airrace3d_file(relative_path: PathBuf) -> axum::response::Response {
    if !is_safe_static_path(&relative_path) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let path = FsPath::new("src").join("airrace3d").join(&relative_path);
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(content_type_for_path(&relative_path)));
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static(cache_control_for_airrace3d_path(&relative_path)),
    );
    if is_gzip_file(&relative_path) {
        headers.insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    }

    (headers, bytes).into_response()
}
fn is_safe_static_path(path: &FsPath) -> bool {
    path.components().all(|component| matches!(component, std::path::Component::Normal(_)))
}
fn content_type_for_path(path: &FsPath) -> &'static str {
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
    if file_name.ends_with(".js.gz") {
        return "application/javascript; charset=utf-8";
    }
    if file_name.ends_with(".wasm.gz") {
        return "application/wasm";
    }
    if file_name.ends_with(".data.gz") || file_name.ends_with(".symbols.gz") {
        return "application/octet-stream";
    }

    match path.extension().and_then(|ext| ext.to_str()).unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "data" => "application/octet-stream",
        "symbols" => "application/octet-stream",
        "json" => "application/json; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "br" => "application/octet-stream",
        "gz" => "application/gzip",
        _ => "application/octet-stream",
    }
}
fn is_gzip_file(path: &FsPath) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("gz")
}
fn cache_control_for_airrace3d_path(path: &FsPath) -> &'static str {
    let p = path.to_string_lossy();
    if p.starts_with("Build/") || p.starts_with("TemplateData/") {
        return "public, max-age=31536000, immutable";
    }
    "no-store"
}
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
        "game.js" => include_str!("truckers/js/game.js"),
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

async fn favicon() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "image/x-icon"),
            (CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_bytes!("favicon.ico").as_slice(),
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

async fn get_airrace_scores(State(s): State<AppState>) -> impl IntoResponse {
    let sb = s.airrace_scores.lock().unwrap();
    let list: Vec<_> = sb.list().to_vec();
    let min = sb.min_score();
    Json(serde_json::json!({ "scores": list, "min_score": min }))
}

async fn post_airrace_score(State(s): State<AppState>, Json(p): Json<SubmitPayload>) -> impl IntoResponse {
    let mut sb = s.airrace_scores.lock().unwrap();
    let rank = sb.add(p.name, p.score);
    let list: Vec<_> = sb.list().to_vec();
    let min = sb.min_score();
    let rank_val = match rank {
        Some(r) => serde_json::Value::Number(r.into()),
        None    => serde_json::Value::Null,
    };
    Json(serde_json::json!({ "rank": rank_val, "scores": list, "min_score": min }))
}

fn airrace_round_course(round: u32) -> Option<serde_json::Value> {
    let root = serde_json::from_str::<serde_json::Value>(AIRRACE_ROUNDS_JSON).ok()?;
    let key = round.to_string();
    Some(root.get(key.as_str())?.clone())
}

async fn get_airrace_round_bundle(
    Path(round): Path<u32>,
    State(s): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(course) = airrace_round_course(round) else {
        return Err(StatusCode::NOT_FOUND);
    };
    let ghosts = s.airrace_ghosts.lock().unwrap().list(Some(round));
    Ok(Json(serde_json::json!({
        "round": round,
        "course": course,
        "ghosts": ghosts,
    })))
}

#[derive(Deserialize)]
struct AirraceGhostQuery {
    round: Option<u32>,
}

async fn get_airrace_ghosts(
    State(s): State<AppState>,
    Query(q): Query<AirraceGhostQuery>,
) -> impl IntoResponse {
    let ghosts = s.airrace_ghosts.lock().unwrap().list(q.round);
    Json(serde_json::json!({ "ghosts": ghosts }))
}

async fn post_airrace_ghost(
    State(s): State<AppState>,
    Json(p): Json<AirraceGhost>,
) -> impl IntoResponse {
    let mut store = s.airrace_ghosts.lock().unwrap();
    store.add(p);
    Json(serde_json::json!({ "ok": true, "count": store.ghosts.len() }))
}
