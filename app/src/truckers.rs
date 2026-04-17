use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{interval, Duration, MissedTickBehavior};

use crate::scores::ScoreBoard;

const TICK_MS: u64 = 33;
const W: f32 = 540.0;
const H: f32 = 540.0;

// Ship stays in lower area; Y is pulled toward SHIP_HOME_Y
const SHIP_HOME_Y: f32 = 420.0;
const SHIP_MAX_SPEED: f32 = 4.5;
const SHIP_ACCEL: f32 = 0.32;
const SHIP_DRAG: f32 = 0.88;
const SHIP_R: f32 = 12.0;

// How many ticks to travel one stage (= distance to destination)
const STAGE_TICKS: u64 = 700; // ~23 seconds
const INVINCIBLE_TICKS: u32 = 60;
const SHIP_HP: u8 = 3;
const SCORE_DELIVERY: u32 = 500;
const SCORE_HP_BONUS: u32 = 200;
const TOTAL_STATIONS: usize = 12;

const STATIONS: [(&str, &str, &str); 12] = [
    ("おひつじ", "Aries",       "♈"),
    ("おうし",   "Taurus",      "♉"),
    ("ふたご",   "Gemini",      "♊"),
    ("かに",     "Cancer",      "♋"),
    ("しし",     "Leo",         "♌"),
    ("おとめ",   "Virgo",       "♍"),
    ("てんびん", "Libra",       "♎"),
    ("さそり",   "Scorpio",     "♏"),
    ("いて",     "Sagittarius", "♐"),
    ("やぎ",     "Capricorn",   "♑"),
    ("みずがめ", "Aquarius",    "♒"),
    ("うお",     "Pisces",      "♓"),
];

struct PlayerCountGuard(Arc<AtomicUsize>);
impl PlayerCountGuard {
    fn new(c: Arc<AtomicUsize>) -> Self { c.fetch_add(1, Ordering::Relaxed); Self(c) }
}
impl Drop for PlayerCountGuard {
    fn drop(&mut self) { self.0.fetch_sub(1, Ordering::Relaxed); }
}

// ── RNG helpers ───────────────────────────────────────────────────────────────
fn lcg(s: u32) -> u32 { s.wrapping_mul(1664525).wrapping_add(1013904223) }
fn lcgf(s: u32) -> f32 { lcg(s) as f32 / u32::MAX as f32 }

// ── Asteroid ──────────────────────────────────────────────────────────────────

struct Asteroid {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,  // always positive (moving downward on screen)
    radius: f32,
    seed: u32,
}

// How often to spawn (ticks between spawns), gets shorter with progress and round
fn spawn_interval(round: u32, progress: f32) -> u64 {
    // progress 0.0..1.0 — denser as we approach destination
    let base = 32u64;
    let round_reduce = (round.saturating_sub(1) as u64) * 3;
    let progress_reduce = (progress * 14.0) as u64;
    base.saturating_sub(round_reduce + progress_reduce).max(8)
}

fn asteroid_speed(round: u32) -> f32 {
    let base_vy = 1.8_f32;
    base_vy * (1.0 + (round.saturating_sub(1) as f32) * 0.18)
}

fn asteroid_max_r(round: u32) -> f32 {
    (14.0 + (round.saturating_sub(1) as f32) * 2.0).min(26.0)
}

fn spawn_asteroid(tick: u64, i: usize, round: u32) -> Asteroid {
    let s0 = lcg((tick as u32).wrapping_add(i as u32 * 6991).wrapping_add(round * 54321));
    let s1 = lcg(s0); let s2 = lcg(s1); let s3 = lcg(s2); let s4 = lcg(s3);

    let x = 18.0 + lcgf(s0) * (W - 36.0);
    let vx = (lcgf(s1) - 0.5) * 1.2 * (1.0 + (round as f32 - 1.0) * 0.15);
    let vy = (0.6 + lcgf(s2) * 0.8) * asteroid_speed(round);
    let radius = 8.0 + lcgf(s3) * (asteroid_max_r(round) - 8.0);

    Asteroid { x, y: -radius - 2.0, vx, vy, radius, seed: s4 }
}

// ── Phase ─────────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Phase { Playing, StageClear, LapClear, GameOver }

// ── Game ──────────────────────────────────────────────────────────────────────

struct Game {
    // Ship
    sx: f32, sy: f32,
    svx: f32, svy: f32,
    hp: u8,
    invincible: u32,
    // Journey progress (0..STAGE_TICKS)
    progress: u64,
    // Overall state
    score: u32,
    round: u32,
    stage: usize,  // 0..11
    lap: u32,
    asteroids: Vec<Asteroid>,
    tick: u64,
    stage_tick: u64, // tick within current stage
    phase: Phase,
    phase_timer: u32,
    event: Option<&'static str>,
    // Input
    keys_up: bool, keys_down: bool, keys_left: bool, keys_right: bool,
}

impl Game {
    fn new() -> Self {
        Game {
            sx: W / 2.0, sy: SHIP_HOME_Y,
            svx: 0.0, svy: 0.0,
            hp: SHIP_HP, invincible: 0,
            progress: 0,
            score: 0, round: 1, stage: 0, lap: 1,
            asteroids: Vec::new(),
            tick: 0, stage_tick: 0,
            phase: Phase::Playing, phase_timer: 0,
            event: None,
            keys_up: false, keys_down: false, keys_left: false, keys_right: false,
        }
    }

    fn tick(&mut self) {
        self.tick += 1;
        self.event = None;

        match self.phase {
            Phase::StageClear | Phase::LapClear => {
                if self.phase_timer > 0 { self.phase_timer -= 1; }
                else { self.begin_next_stage(); }
                return;
            }
            Phase::GameOver => return,
            Phase::Playing => {}
        }

        self.stage_tick += 1;
        if self.invincible > 0 { self.invincible -= 1; }

        // ── Ship movement ──
        let ax = if self.keys_right { SHIP_ACCEL } else if self.keys_left { -SHIP_ACCEL } else { 0.0 };
        // Vertical: pull ship back toward home Y unless key pressed
        let target_dy = SHIP_HOME_Y - self.sy;
        let ay = if self.keys_down {
            SHIP_ACCEL
        } else if self.keys_up {
            -SHIP_ACCEL
        } else {
            // gentle centering
            (target_dy * 0.04).clamp(-SHIP_ACCEL * 0.5, SHIP_ACCEL * 0.5)
        };

        self.svx = (self.svx + ax) * SHIP_DRAG;
        self.svy = (self.svy + ay) * SHIP_DRAG;

        let spd = (self.svx * self.svx + self.svy * self.svy).sqrt();
        if spd > SHIP_MAX_SPEED {
            self.svx = self.svx / spd * SHIP_MAX_SPEED;
            self.svy = self.svy / spd * SHIP_MAX_SPEED;
        }

        self.sx = (self.sx + self.svx).clamp(SHIP_R, W - SHIP_R);
        self.sy = (self.sy + self.svy).clamp(H * 0.3, H - SHIP_R - 10.0);

        // ── Spawn asteroids ──
        let progress_ratio = self.stage_tick as f32 / STAGE_TICKS as f32;
        if self.tick % spawn_interval(self.round, progress_ratio.min(1.0)) == 0 {
            self.asteroids.push(spawn_asteroid(self.tick, self.asteroids.len(), self.round));
        }

        // ── Move asteroids (downward), remove off-screen ──
        let mut i = 0;
        while i < self.asteroids.len() {
            let a = &mut self.asteroids[i];
            a.x += a.vx;
            a.y += a.vy;
            // horizontal wrap
            if a.x < -a.radius - 10.0 { a.x += W + a.radius * 2.0; }
            if a.x > W + a.radius + 10.0 { a.x -= W + a.radius * 2.0; }
            // remove below screen
            if a.y > H + a.radius + 20.0 {
                self.asteroids.swap_remove(i);
            } else {
                i += 1;
            }
        }

        // ── Collision ──
        if self.invincible == 0 {
            for a in &self.asteroids {
                let dx = self.sx - a.x;
                let dy = self.sy - a.y;
                if (dx * dx + dy * dy).sqrt() < SHIP_R + a.radius {
                    self.hp -= 1;
                    self.invincible = INVINCIBLE_TICKS;
                    self.event = Some("damage");
                    if self.hp == 0 {
                        self.phase = Phase::GameOver;
                        self.event = Some("gameover");
                        return;
                    }
                    break;
                }
            }
        }

        // ── Progress → arrival ──
        if self.stage_tick >= STAGE_TICKS {
            let pts = SCORE_DELIVERY * self.round;
            self.score += pts;
            let next_stage = self.stage + 1;
            if next_stage >= TOTAL_STATIONS {
                let hp_bonus = self.hp as u32 * SCORE_HP_BONUS * self.round;
                self.score += hp_bonus;
                self.event = Some("lap_clear");
                self.phase = Phase::LapClear;
                self.phase_timer = 130;
            } else {
                self.event = Some("delivery");
                self.phase = Phase::StageClear;
                self.phase_timer = 70;
            }
        }
    }

    fn begin_next_stage(&mut self) {
        if self.stage + 1 >= TOTAL_STATIONS {
            self.round += 1;
            self.stage = 0;
            self.lap += 1;
            self.hp = SHIP_HP;
        } else {
            self.stage += 1;
        }
        self.sx = W / 2.0;
        self.sy = SHIP_HOME_Y;
        self.svx = 0.0; self.svy = 0.0;
        self.asteroids.clear();
        self.invincible = 0;
        self.stage_tick = 0;
        self.phase = Phase::Playing;
        self.event = None;
    }

    fn to_json(&self) -> String {
        let phase_str = match self.phase {
            Phase::Playing    => "playing",
            Phase::StageClear => "stage_clear",
            Phase::LapClear   => "lap_clear",
            Phase::GameOver   => "gameover",
        };

        let progress_pct = (self.stage_tick as f32 / STAGE_TICKS as f32 * 100.0).min(100.0);
        let from_st = &STATIONS[self.stage % TOTAL_STATIONS];
        let to_st   = &STATIONS[(self.stage + 1) % TOTAL_STATIONS];

        let asteroids: Vec<_> = self.asteroids.iter().map(|a| {
            serde_json::json!({ "x": a.x, "y": a.y, "r": a.radius, "seed": a.seed })
        }).collect();

        serde_json::json!({
            "type": "state",
            "phase": phase_str,
            "ship": {
                "x": self.sx, "y": self.sy,
                "vx": self.svx, "vy": self.svy,
                "invincible": self.invincible > 0,
            },
            "hp": self.hp,
            "score": self.score,
            "round": self.round,
            "stage": self.stage,
            "lap": self.lap,
            "progress": progress_pct,
            "stage_tick": self.stage_tick,
            "from": { "name_ja": from_st.0, "name_en": from_st.1, "symbol": from_st.2 },
            "to":   { "name_ja": to_st.0,   "name_en": to_st.1,   "symbol": to_st.2 },
            "asteroids": asteroids,
            "event": self.event,
            "tick": self.tick,
        }).to_string()
    }
}

// ── Input / messages ──────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
struct Keys {
    #[serde(default)] up: bool,
    #[serde(default)] down: bool,
    #[serde(default)] left: bool,
    #[serde(default)] right: bool,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Input { keys: Keys },
    Start { #[serde(default)] name: String },
    Restart,
}

fn get_score_list(scores: &Arc<Mutex<ScoreBoard>>) -> serde_json::Value {
    serde_json::json!(scores.lock().unwrap().list())
}

// ── WebSocket handler ─────────────────────────────────────────────────────────

pub async fn run(
    mut socket: WebSocket,
    scores: Arc<Mutex<ScoreBoard>>,
    players: Arc<AtomicUsize>,
) {
    let _guard = PlayerCountGuard::new(players);

    let _ = socket.send(Message::Text(
        serde_json::json!({
            "type": "state", "phase": "title",
            "scores": get_score_list(&scores),
        }).to_string().into()
    )).await;

    loop {
        let player_name = loop {
            match socket.recv().await {
                Some(Ok(Message::Text(txt))) => {
                    if let Ok(ClientMsg::Start { name }) = serde_json::from_str(&txt) {
                        break if name.trim().is_empty() { "野郎".to_string() }
                             else { name.chars().take(20).collect() };
                    }
                }
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                _ => {}
            }
        };

        let dc = game_session(&mut socket, &scores, &player_name).await;
        if dc { return; }
    }
}

async fn game_session(
    socket: &mut WebSocket,
    scores: &Arc<Mutex<ScoreBoard>>,
    player_name: &str,
) -> bool {
    let mut game = Game::new();
    let mut ticker = interval(Duration::from_millis(TICK_MS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                game.tick();
                if socket.send(Message::Text(game.to_json().into())).await.is_err() {
                    return true;
                }
                if game.phase == Phase::GameOver {
                    let rank = { let mut sb = scores.lock().unwrap(); sb.add(player_name.to_string(), game.score) };
                    let _ = socket.send(Message::Text(
                        serde_json::json!({
                            "type": "state", "phase": "gameover",
                            "score": game.score, "round": game.round, "lap": game.lap,
                            "rank": rank,
                            "scores": get_score_list(scores),
                        }).to_string().into()
                    )).await;
                    loop {
                        match socket.recv().await {
                            Some(Ok(Message::Text(txt))) => {
                                if let Ok(ClientMsg::Restart) = serde_json::from_str(&txt) {
                                    let _ = socket.send(Message::Text(
                                        serde_json::json!({
                                            "type": "state", "phase": "title",
                                            "scores": get_score_list(scores),
                                        }).to_string().into()
                                    )).await;
                                    return false;
                                }
                            }
                            None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                            _ => {}
                        }
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(txt))) => {
                        if let Ok(ClientMsg::Input { keys }) = serde_json::from_str(&txt) {
                            game.keys_up    = keys.up;
                            game.keys_down  = keys.down;
                            game.keys_left  = keys.left;
                            game.keys_right = keys.right;
                        }
                    }
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                    _ => {}
                }
            }
        }
    }
}
