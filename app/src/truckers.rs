use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{interval, Duration, MissedTickBehavior};

use crate::scores::ScoreBoard;

const TICK_MS: u64 = 33;
const W: f32 = 540.0;
const H: f32 = 540.0;
const SHIP_R: f32 = 12.0;
const SHIP_MAX_SPEED: f32 = 4.5;
const SHIP_ACCEL: f32 = 0.32;
const SHIP_DRAG: f32 = 0.90;
const SHIP_START_Y: f32 = 80.0;
const DEST_Y: f32 = 460.0;
const DEST_R: f32 = 36.0;
const INVINCIBLE_TICKS: u32 = 60;
const SHIP_HP: u8 = 3;
const SCORE_DELIVERY: u32 = 500;
const SCORE_HP_BONUS: u32 = 200;
const TOTAL_STATIONS: usize = 12;

// 12 zodiac stations
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

// ── Asteroid ──────────────────────────────────────────────────────────────────

struct Asteroid {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
    seed: u32, // for visual rendering variety
}

fn lcg(s: u32) -> u32 { s.wrapping_mul(1664525).wrapping_add(1013904223) }
fn lcgf(s: u32) -> f32 { lcg(s) as f32 / u32::MAX as f32 }

fn make_asteroid(tick: u64, i: usize, round: u32) -> Asteroid {
    let seed0 = lcg((tick as u32).wrapping_add(i as u32 * 7919).wrapping_add(round * 31337));
    let seed1 = lcg(seed0);
    let seed2 = lcg(seed1);
    let seed3 = lcg(seed2);
    let seed4 = lcg(seed3);

    let x = 20.0 + lcgf(seed0) * (W - 40.0);
    // horizontal drift: slight, centered near 0
    let vx = (lcgf(seed1) - 0.5) * 1.4 * speed_scale(round);
    let base_vy = 1.2 + lcgf(seed2) * 1.8;
    let vy = base_vy * speed_scale(round);
    let radius = 8.0 + lcgf(seed3) * max_radius_extra(round);

    Asteroid { x, y: -radius - 4.0, vx, vy, radius, seed: seed4 }
}

fn speed_scale(round: u32) -> f32 {
    1.0 + (round.saturating_sub(1) as f32) * 0.18
}

fn max_radius_extra(round: u32) -> f32 {
    // extra radius beyond base 8px: grows with round, capped
    (14.0 + round.saturating_sub(1) as f32 * 2.0).min(22.0)
}

// ticks between asteroid spawns (decreases with round)
fn spawn_interval(round: u32) -> u64 {
    let base = 28u64;
    let reduce = (round.saturating_sub(1) as u64) * 3;
    base.saturating_sub(reduce).max(10)
}

// ── Phase ─────────────────────────────────────────────────────────────────────

#[derive(PartialEq, Clone, Copy)]
enum Phase {
    Playing,
    StageClear,  // brief pause between stages
    LapClear,    // 12 deliveries done
    GameOver,
}

// ── Game ──────────────────────────────────────────────────────────────────────

struct Game {
    // ship
    sx: f32, sy: f32,
    svx: f32, svy: f32,
    sangle: f32,
    hp: u8,
    invincible: u32,
    // destination x (randomised slightly each stage, but always at DEST_Y)
    dest_x: f32,
    // game state
    score: u32,
    round: u32,
    stage: usize,        // 0..11, which delivery leg within the current LAP
    lap: u32,
    asteroids: Vec<Asteroid>,
    tick: u64,
    phase: Phase,
    event: Option<&'static str>,
    phase_timer: u32,    // ticks remaining in StageClear / LapClear pause
    // input
    keys_up: bool, keys_down: bool, keys_left: bool, keys_right: bool,
}

impl Game {
    fn new() -> Self {
        let mut g = Game {
            sx: W / 2.0, sy: SHIP_START_Y,
            svx: 0.0, svy: 0.0, sangle: std::f32::consts::FRAC_PI_2, // facing down
            hp: SHIP_HP, invincible: 0,
            dest_x: W / 2.0,
            score: 0, round: 1, stage: 0, lap: 1,
            asteroids: Vec::new(),
            tick: 0, phase: Phase::Playing, event: None, phase_timer: 0,
            keys_up: false, keys_down: false, keys_left: false, keys_right: false,
        };
        g.dest_x = stage_dest_x(0, 1);
        g
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

        if self.invincible > 0 { self.invincible -= 1; }

        // ── Ship movement ──
        let ax = if self.keys_right { SHIP_ACCEL } else if self.keys_left { -SHIP_ACCEL } else { 0.0 };
        let ay = if self.keys_down  { SHIP_ACCEL } else if self.keys_up   { -SHIP_ACCEL } else { 0.0 };
        self.svx = (self.svx + ax) * SHIP_DRAG;
        self.svy = (self.svy + ay) * SHIP_DRAG;

        let spd = (self.svx * self.svx + self.svy * self.svy).sqrt();
        if spd > SHIP_MAX_SPEED {
            self.svx = self.svx / spd * SHIP_MAX_SPEED;
            self.svy = self.svy / spd * SHIP_MAX_SPEED;
        }
        self.sx = (self.sx + self.svx).clamp(SHIP_R, W - SHIP_R);
        self.sy = (self.sy + self.svy).clamp(SHIP_R, H - SHIP_R);

        // Visual angle: tilt toward velocity direction
        if spd > 0.3 {
            self.sangle = self.svy.atan2(self.svx);
        }

        // ── Spawn asteroids ──
        if self.tick % spawn_interval(self.round) == 0 {
            self.asteroids.push(make_asteroid(self.tick, self.asteroids.len(), self.round));
        }

        // ── Move asteroids, remove off-screen ──
        let mut i = 0;
        while i < self.asteroids.len() {
            let a = &mut self.asteroids[i];
            a.x += a.vx;
            a.y += a.vy;
            // Wrap horizontally
            if a.x < -a.radius { a.x = W + a.radius; }
            if a.x > W + a.radius { a.x = -a.radius; }
            // Remove when past bottom
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

        // ── Check destination ──
        let dx = self.sx - self.dest_x;
        let dy = self.sy - DEST_Y;
        if (dx * dx + dy * dy).sqrt() < DEST_R {
            // Delivery!
            let pts = SCORE_DELIVERY * self.round;
            self.score += pts;
            self.event = Some("delivery");

            let next_stage = self.stage + 1;
            if next_stage >= TOTAL_STATIONS {
                // LAP CLEAR
                let hp_bonus = self.hp as u32 * SCORE_HP_BONUS * self.round;
                self.score += hp_bonus;
                self.event = Some("lap_clear");
                self.phase = Phase::LapClear;
                self.phase_timer = 120; // ~4 seconds
            } else {
                self.phase = Phase::StageClear;
                self.phase_timer = 60; // ~2 seconds
            }
        }
    }

    fn begin_next_stage(&mut self) {
        let was_lap = self.stage + 1 >= TOTAL_STATIONS;
        if was_lap {
            self.round += 1;
            self.stage = 0;
            self.lap += 1;
            self.hp = SHIP_HP; // full HP recovery
        } else {
            self.stage += 1;
        }
        // Reset ship to top
        self.sx = W / 2.0;
        self.sy = SHIP_START_Y;
        self.svx = 0.0;
        self.svy = 0.0;
        self.sangle = std::f32::consts::FRAC_PI_2;
        self.dest_x = stage_dest_x(self.stage, self.round);
        // Clear asteroids
        self.asteroids.clear();
        self.invincible = 0;
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

        let asteroids: Vec<_> = self.asteroids.iter().map(|a| {
            serde_json::json!({ "x": a.x, "y": a.y, "r": a.radius, "seed": a.seed })
        }).collect();

        let from_idx = self.stage % TOTAL_STATIONS;
        let to_idx   = (self.stage + 1) % TOTAL_STATIONS;
        let from_st  = &STATIONS[from_idx];
        let to_st    = &STATIONS[to_idx];

        serde_json::json!({
            "type": "state",
            "phase": phase_str,
            "ship": {
                "x": self.sx, "y": self.sy,
                "vx": self.svx, "vy": self.svy,
                "angle": self.sangle,
                "invincible": self.invincible > 0,
            },
            "dest_x": self.dest_x,
            "hp": self.hp,
            "score": self.score,
            "round": self.round,
            "stage": self.stage,
            "lap": self.lap,
            "from": { "name_ja": from_st.0, "name_en": from_st.1, "symbol": from_st.2 },
            "to":   { "name_ja": to_st.0,   "name_en": to_st.1,   "symbol": to_st.2 },
            "asteroids": asteroids,
            "event": self.event,
            "tick": self.tick,
        }).to_string()
    }
}

/// Destination x varies per stage/round to add variety
fn stage_dest_x(stage: usize, round: u32) -> f32 {
    let seed = lcg((stage as u32).wrapping_mul(1009).wrapping_add(round * 997));
    100.0 + lcgf(seed) * (W - 200.0)
}

// ── Input ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
struct Keys {
    #[serde(default)] up:    bool,
    #[serde(default)] down:  bool,
    #[serde(default)] left:  bool,
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
    let sb = scores.lock().unwrap();
    serde_json::json!(sb.list())
}

// ── WebSocket handler ─────────────────────────────────────────────────────────

pub async fn run(
    mut socket: WebSocket,
    scores: Arc<Mutex<ScoreBoard>>,
    players: Arc<AtomicUsize>,
) {
    let _guard = PlayerCountGuard::new(players);

    // Title
    let _ = socket.send(Message::Text(
        serde_json::json!({
            "type": "state", "phase": "title",
            "scores": get_score_list(&scores),
        }).to_string().into()
    )).await;

    loop {
        // Wait for Start
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

        let disconnected = game_session(&mut socket, &scores, &player_name).await;
        if disconnected { return; }
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
                    // Submit score
                    let rank = {
                        let mut sb = scores.lock().unwrap();
                        sb.add(player_name.to_string(), game.score)
                    };
                    let _ = socket.send(Message::Text(
                        serde_json::json!({
                            "type": "state", "phase": "gameover",
                            "score": game.score, "round": game.round, "lap": game.lap,
                            "rank": rank,
                            "scores": get_score_list(scores),
                        }).to_string().into()
                    )).await;

                    // Wait for restart
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
