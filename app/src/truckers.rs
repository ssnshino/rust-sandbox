use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{interval, Duration, MissedTickBehavior};

use crate::scores::ScoreBoard;

const TICK_MS: u64 = 33;
const CANVAS: f32 = 540.0;
const CENTER: f32 = 270.0;
const STATION_R: f32 = 210.0;
const SHIP_R: f32 = 12.0;
const DOCK_R: f32 = 34.0;
const MAX_SPEED: f32 = 5.0;
const ACCEL: f32 = 0.35;
const DRAG: f32 = 0.94;
const INVINCIBLE_TICKS: u32 = 60;
const SHIP_HP: u8 = 3;
const SCORE_DELIVERY: u32 = 500;
const SCORE_HP_BONUS: u32 = 200;

// 12 zodiac stations: (name_ja, name_en, symbol, angle_deg)
// angle_deg: standard math angle (0=right, 90=top, CCW)
// screen y is flipped, so y = CENTER - R*sin(angle)
const STATIONS: [(&str, &str, &str, f32); 12] = [
    ("おひつじ", "Aries",       "♈",  90.0),
    ("おうし",   "Taurus",      "♉",  60.0),
    ("ふたご",   "Gemini",      "♊",  30.0),
    ("かに",     "Cancer",      "♋",   0.0),
    ("しし",     "Leo",         "♌", 330.0),
    ("おとめ",   "Virgo",       "♍", 300.0),
    ("てんびん", "Libra",       "♎", 270.0),
    ("さそり",   "Scorpio",     "♏", 240.0),
    ("いて",     "Sagittarius", "♐", 210.0),
    ("やぎ",     "Capricorn",   "♑", 180.0),
    ("みずがめ", "Aquarius",    "♒", 150.0),
    ("うお",     "Pisces",      "♓", 120.0),
];

fn station_pos(angle_deg: f32) -> (f32, f32) {
    let r = angle_deg.to_radians();
    (CENTER + STATION_R * r.cos(), CENTER - STATION_R * r.sin())
}

fn asteroid_count(round: u32) -> usize {
    match round { 1 => 6, 2 => 9, 3 => 12, _ => 15 }
}

fn asteroid_speed_scale(round: u32) -> f32 {
    1.0 + (round.saturating_sub(1) as f32) * 0.2
}

fn asteroid_max_radius(round: u32) -> f32 {
    (22.0 + (round.saturating_sub(1) as f32) * 2.0).min(30.0)
}

fn lcg(seed: u32) -> u32 {
    seed.wrapping_mul(1664525).wrapping_add(1013904223)
}

fn lcg_f(seed: u32) -> f32 {
    lcg(seed) as f32 / u32::MAX as f32
}

struct PlayerCountGuard(Arc<AtomicUsize>);
impl PlayerCountGuard {
    fn new(c: Arc<AtomicUsize>) -> Self { c.fetch_add(1, Ordering::Relaxed); Self(c) }
}
impl Drop for PlayerCountGuard {
    fn drop(&mut self) { self.0.fetch_sub(1, Ordering::Relaxed); }
}

// ── Asteroid ─────────────────────────────────────────────────────────────────

struct Asteroid {
    x: f32, y: f32,
    vx: f32, vy: f32,
    radius: f32,
    seed: u32,
}

fn spawn_asteroids(count: usize, round: u32, base_seed: u32, ship_x: f32, ship_y: f32) -> Vec<Asteroid> {
    let spd = asteroid_speed_scale(round);
    let max_r = asteroid_max_radius(round);
    let mut v = Vec::with_capacity(count);
    let mut seed = base_seed;

    for _ in 0..count {
        seed = lcg(seed);
        let x = lcg_f(seed) * CANVAS;
        seed = lcg(seed);
        let y = lcg_f(seed) * CANVAS;
        seed = lcg(seed);
        let angle = lcg_f(seed) * std::f32::consts::TAU;
        seed = lcg(seed);
        let speed_t = lcg_f(seed); // 0..1
        seed = lcg(seed);
        let radius_t = lcg_f(seed);
        seed = lcg(seed);
        let vis_seed = seed;

        // keep away from ship spawn
        let dx = x - ship_x;
        let dy = y - ship_y;
        let d = (dx*dx + dy*dy).sqrt();
        let (ax, ay) = if d < 70.0 {
            // place on opposite side
            let ox = ship_x + (if x < CENTER { -120.0 } else { 120.0 });
            let oy = ship_y + (if y < CENTER { -120.0 } else { 120.0 });
            (ox.clamp(20.0, CANVAS - 20.0), oy.clamp(20.0, CANVAS - 20.0))
        } else {
            (x, y)
        };

        let spd_actual = (0.5 + speed_t * 1.5) * spd;
        let vx = angle.cos() * spd_actual;
        let vy = angle.sin() * spd_actual;
        let radius = 8.0 + radius_t * (max_r - 8.0);

        v.push(Asteroid { x: ax, y: ay, vx, vy, radius, seed: vis_seed });
    }
    v
}

// ── Game struct ───────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum Phase { Playing, LapClear, GameOver }

struct Game {
    ship_x: f32, ship_y: f32,
    ship_vx: f32, ship_vy: f32,
    ship_angle: f32,
    hp: u8,
    invincible: u32,
    score: u32,
    round: u32,
    // current delivery target index (0..11)
    target_idx: usize,
    asteroids: Vec<Asteroid>,
    tick: u64,
    phase: Phase,
    event: Option<&'static str>,
    keys_up: bool, keys_down: bool, keys_left: bool, keys_right: bool,
}

impl Game {
    fn new() -> Self {
        let (sx, sy) = station_pos(90.0); // Aries
        let mut g = Game {
            ship_x: sx, ship_y: sy,
            ship_vx: 0.0, ship_vy: 0.0,
            ship_angle: 0.0,
            hp: SHIP_HP,
            invincible: 0,
            score: 0,
            round: 1,
            target_idx: 1, // first target is Taurus (index 1)
            asteroids: Vec::new(),
            tick: 0,
            phase: Phase::Playing,
            event: None,
            keys_up: false, keys_down: false, keys_left: false, keys_right: false,
        };
        g.asteroids = spawn_asteroids(asteroid_count(1), 1, 42, sx, sy);
        g
    }

    fn next_round(&mut self) {
        self.round += 1;
        self.hp = SHIP_HP; // full HP recovery
        self.target_idx = 1; // restart from Taurus
        let (sx, sy) = station_pos(90.0);
        self.ship_x = sx;
        self.ship_y = sy;
        self.ship_vx = 0.0;
        self.ship_vy = 0.0;
        self.asteroids = spawn_asteroids(
            asteroid_count(self.round),
            self.round,
            self.tick as u32,
            sx, sy,
        );
    }

    fn tick(&mut self) {
        self.tick += 1;
        if self.phase != Phase::Playing { return; }
        self.event = None;
        if self.invincible > 0 { self.invincible -= 1; }

        // Ship physics
        let ax = if self.keys_right { ACCEL } else if self.keys_left { -ACCEL } else { 0.0 };
        let ay = if self.keys_down  { ACCEL } else if self.keys_up   { -ACCEL } else { 0.0 };
        self.ship_vx = (self.ship_vx + ax) * DRAG;
        self.ship_vy = (self.ship_vy + ay) * DRAG;
        // clamp speed
        let spd = (self.ship_vx*self.ship_vx + self.ship_vy*self.ship_vy).sqrt();
        if spd > MAX_SPEED {
            self.ship_vx = self.ship_vx / spd * MAX_SPEED;
            self.ship_vy = self.ship_vy / spd * MAX_SPEED;
        }
        self.ship_x = (self.ship_x + self.ship_vx).rem_euclid(CANVAS);
        self.ship_y = (self.ship_y + self.ship_vy).rem_euclid(CANVAS);

        // Update visual angle
        if spd > 0.3 {
            self.ship_angle = self.ship_vy.atan2(self.ship_vx);
        }

        // Move asteroids (wrap-around)
        for a in &mut self.asteroids {
            a.x = (a.x + a.vx).rem_euclid(CANVAS);
            a.y = (a.y + a.vy).rem_euclid(CANVAS);
        }

        // Asteroid collision (toroidal distance)
        if self.invincible == 0 {
            for a in &self.asteroids {
                let dx = toroidal_diff(self.ship_x, a.x, CANVAS);
                let dy = toroidal_diff(self.ship_y, a.y, CANVAS);
                let dist = (dx*dx + dy*dy).sqrt();
                if dist < SHIP_R + a.radius {
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

        // Check station arrival
        let (tx, ty) = station_pos(STATIONS[self.target_idx].3);
        let dx = self.ship_x - tx;
        let dy = self.ship_y - ty;
        let dist = (dx*dx + dy*dy).sqrt();
        if dist < DOCK_R {
            // Delivery!
            let delivery_score = SCORE_DELIVERY * self.round;
            self.score += delivery_score;
            self.event = Some("delivery");

            self.target_idx += 1;
            if self.target_idx >= 12 {
                // LAP CLEAR — need to return to station 0 (Aries) first
                // Actually: after delivering to station 11 (Pisces, idx 11),
                // next target_idx would be 12 which means return to Aries (idx 0)
                // But plan says: deliver to all 12, then return to Aries = clear
                // We treat target_idx 12 as "return to Aries"
                // Actually let's make it: after station[11], target becomes 0 (Aries) for the return
                // and when they reach Aries again, it's LAP CLEAR
                self.target_idx = 12; // special value: return to Aries
            }
        }

        // Check return to Aries (lap clear)
        if self.target_idx == 12 {
            let (ax2, ay2) = station_pos(90.0); // Aries
            let dx = self.ship_x - ax2;
            let dy = self.ship_y - ay2;
            let dist = (dx*dx + dy*dy).sqrt();
            if dist < DOCK_R {
                // LAP CLEAR
                let hp_bonus = self.hp as u32 * SCORE_HP_BONUS * self.round;
                self.score += hp_bonus;
                self.event = Some("lap_clear");
                self.phase = Phase::LapClear;
            }
        }
    }

    fn to_json(&self) -> String {
        let phase_str = match self.phase {
            Phase::Playing  => "playing",
            Phase::LapClear => "lap_clear",
            Phase::GameOver => "gameover",
        };

        let target_display = if self.target_idx < 12 {
            let s = &STATIONS[self.target_idx];
            serde_json::json!({
                "idx": self.target_idx,
                "name_ja": s.0,
                "name_en": s.1,
                "symbol": s.2,
                "x": station_pos(s.3).0,
                "y": station_pos(s.3).1,
            })
        } else {
            // returning to Aries
            let s = &STATIONS[0];
            serde_json::json!({
                "idx": 0,
                "name_ja": s.0,
                "name_en": s.1,
                "symbol": s.2,
                "x": station_pos(s.3).0,
                "y": station_pos(s.3).1,
                "return": true,
            })
        };

        let asteroids: Vec<_> = self.asteroids.iter().map(|a| {
            serde_json::json!({
                "x": a.x, "y": a.y,
                "r": a.radius,
                "seed": a.seed,
            })
        }).collect();

        let stations_json: Vec<_> = STATIONS.iter().enumerate().map(|(i, s)| {
            let (x, y) = station_pos(s.3);
            serde_json::json!({
                "idx": i, "name_ja": s.0, "name_en": s.1, "symbol": s.2,
                "x": x, "y": y,
            })
        }).collect();

        serde_json::json!({
            "type": "state",
            "phase": phase_str,
            "ship": {
                "x": self.ship_x, "y": self.ship_y,
                "vx": self.ship_vx, "vy": self.ship_vy,
                "angle": self.ship_angle,
                "invincible": self.invincible > 0,
            },
            "hp": self.hp,
            "score": self.score,
            "round": self.round,
            "target": target_display,
            "stations": stations_json,
            "asteroids": asteroids,
            "event": self.event,
            "tick": self.tick,
        }).to_string()
    }
}

fn toroidal_diff(a: f32, b: f32, size: f32) -> f32 {
    let d = a - b;
    if d > size * 0.5 { d - size }
    else if d < -size * 0.5 { d + size }
    else { d }
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
    NextRound,
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

    // Send title state
    let _ = socket.send(Message::Text(
        serde_json::json!({
            "type": "state",
            "phase": "title",
            "scores": get_score_list(&scores),
        }).to_string().into()
    )).await;

    loop {
        // Wait for Start message at title
        let player_name = loop {
            match socket.recv().await {
                Some(Ok(Message::Text(txt))) => {
                    if let Ok(ClientMsg::Start { name }) = serde_json::from_str(&txt) {
                        break if name.trim().is_empty() { "???".to_string() } else { name.chars().take(20).collect() };
                    }
                }
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                _ => {}
            }
        };

        // Run game session
        let disconnected = game_session(&mut socket, &scores, &player_name).await;
        if disconnected { return; }
        // Otherwise loop back to title (game_session sends title state before returning)
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
                let json = game.to_json();
                if socket.send(Message::Text(json.into())).await.is_err() {
                    return true; // disconnected
                }

                match game.phase {
                    Phase::LapClear => {
                        // Brief pause, then next round
                        tokio::time::sleep(Duration::from_millis(2000)).await;
                        game.next_round();
                        game.phase = Phase::Playing;
                    }
                    Phase::GameOver => {
                        // Submit score
                        let rank = {
                            let mut sb = scores.lock().unwrap();
                            sb.add(player_name.to_string(), game.score)
                        };
                        let score_list = get_score_list(scores);
                        let _ = socket.send(Message::Text(
                            serde_json::json!({
                                "type": "state",
                                "phase": "gameover",
                                "score": game.score,
                                "round": game.round,
                                "rank": rank,
                                "scores": score_list,
                            }).to_string().into()
                        )).await;

                        // Wait for restart or disconnect
                        loop {
                            match socket.recv().await {
                                Some(Ok(Message::Text(txt))) => {
                                    if let Ok(ClientMsg::Restart) = serde_json::from_str(&txt) {
                                        // Send title
                                        let _ = socket.send(Message::Text(
                                            serde_json::json!({
                                                "type": "state",
                                                "phase": "title",
                                                "scores": get_score_list(scores),
                                            }).to_string().into()
                                        )).await;
                                        return false; // go back to title loop
                                    }
                                }
                                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                                _ => {}
                            }
                        }
                    }
                    Phase::Playing => {}
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
