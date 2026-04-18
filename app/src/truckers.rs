use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{interval, Duration, MissedTickBehavior};

use crate::scores::ScoreBoard;

const TICK_MS: u64 = 33;
const W: f32 = 540.0;
const H: f32 = 540.0;

const SHIP_MAX_SPEED: f32 = 5.5;
const SHIP_ACCEL: f32 = 0.35;
const SHIP_DRAG: f32 = 0.975;
const SHIP_R: f32 = 10.0;
const LAUNCH_TARGET_Y: f32 = H * 0.67;

const LAUNCH_TICKS: u64 = 90;
const CRUISE_TICKS: u64 = 1100;
const BOOSTER_TRIGGER_PCT: f32 = 0.14;
const BOOSTER_Y: f32 = 185.0;
const BOOSTER_X_PERFECT: f32 = 6.0;
const BOOSTER_X_GOOD: f32 = 14.0;
const BOOSTER_X_OK: f32 = 24.0;
const BOOSTER_SCROLL_BONUS_Y: f32 = 1.85;

const DOCK_Y_R: f32 = 35.0;  // 縦方向のドッキング許容範囲
const DOCK_X_PERFECT: f32 = 5.0;  // 完璧ドッキング
const DOCK_X_GOOD: f32    = 12.0;
const DOCK_X_OK: f32      = 22.0; // これ以上離れると不可
const AIRLOCK_Y: f32 = 50.0;
const DEPART_Y: f32 = H - 40.0;

const INVINCIBLE_TICKS: u32 = 60;
const SHIP_HP: u8 = 3;
const SCORE_DELIVERY: u32 = 500;
const SCORE_HP_BONUS: u32 = 200;
const SCORE_DOCK_PERFECT: u32 = 2000;
const SCORE_DOCK_GOOD: u32 = 1000;
const SCORE_DOCK_OK: u32 = 400;
const SCORE_BOOSTER_PERFECT: u32 = 1200;
const SCORE_BOOSTER_GOOD: u32 = 700;
const SCORE_BOOSTER_OK: u32 = 300;
const TOTAL_STATIONS: usize = 12;

// マニピュレーター
const MANIP_MAX_LEN: f32 = 44.0;
const MANIP_GROW: f32 = 3.5;
const MANIP_SHRINK: f32 = 5.5;

// 鉱石スポーン間隔
const MINERAL_INTERVAL: u64 = 60;
const MINERAL_R: f32 = 5.0;

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

fn lcg(s: u32) -> u32 { s.wrapping_mul(1664525).wrapping_add(1013904223) }
fn lcgf(s: u32) -> f32 { lcg(s) as f32 / u32::MAX as f32 }

// ── Asteroid ──────────────────────────────────────────────────────────────────

struct Asteroid { x: f32, y: f32, vx: f32, vy: f32, radius: f32, speed_tier: u8, seed: u32 }

const SPEED_TIERS: [f32; 5] = [1.4, 2.4, 3.6, 5.0, 7.0];
const SIZE_TIERS:  [f32; 5] = [5.0, 9.0, 14.0, 20.0, 27.0];

fn round_scale(round: u32) -> f32 { 1.0 + (round.saturating_sub(1) as f32) * 0.22 }

fn spawn_interval(round: u32, progress: f32) -> u64 {
    let base = 28u64;
    let rr = (round.saturating_sub(1) as u64) * 3;
    let pr = (progress * 16.0) as u64;
    base.saturating_sub(rr + pr).max(8)
}

fn spawn_interval_boosted(round: u32, progress: f32) -> u64 {
    spawn_interval(round, progress).saturating_sub(5).max(6)
}

fn spawn_asteroid(tick: u64, i: usize, round: u32) -> Asteroid {
    let s0 = lcg((tick as u32).wrapping_add(i as u32 * 6991).wrapping_add(round * 54321));
    let s1=lcg(s0); let s2=lcg(s1); let s3=lcg(s2); let s4=lcg(s3);
    let x  = 18.0 + lcgf(s0) * (W - 36.0);
    let vx = (lcgf(s1) - 0.5) * 1.8 * (1.0 + (round as f32 - 1.0) * 0.08);
    let st = (lcg(s2) % 5) as usize;
    let vy = SPEED_TIERS[st] * round_scale(round);
    let sz = (lcg(s3) % 5) as usize;
    Asteroid { x, y: -SIZE_TIERS[sz]-2.0, vx, vy, radius: SIZE_TIERS[sz], speed_tier: st as u8, seed: s4 }
}

// ── Mineral ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum MineralKind { Gold, Rare }

struct Mineral {
    x: f32, y: f32,
    vx: f32, vy: f32,
    kind: MineralKind,
    seed: u32,
    id: u32,
}

fn spawn_mineral(tick: u64, id: u32, round: u32) -> Mineral {
    let s0 = lcg((tick as u32).wrapping_add(id * 8191).wrapping_add(round * 11111));
    let s1=lcg(s0); let s2=lcg(s1); let s3=lcg(s2);
    let x  = 24.0 + lcgf(s0) * (W - 48.0);
    let vx = (lcgf(s1) - 0.5) * 0.6;
    let vy = 0.4 + lcgf(s2) * 0.5; // ゆっくり落ちる
    let kind = if lcgf(s3) < 0.65 { MineralKind::Gold } else { MineralKind::Rare };
    Mineral { x, y: -MINERAL_R - 2.0, vx, vy, kind, seed: s0, id }
}

// ── Phase ─────────────────────────────────────────────────────────────────────
#[derive(PartialEq, Clone, Copy)]
enum Phase { Launching, Playing, BoosterDocking, Docking, StageClear, LapClear, GameOver }

// ── Game ──────────────────────────────────────────────────────────────────────
struct Game {
    sx: f32, sy: f32, svx: f32, svy: f32,
    hp: u8, invincible: u32,
    score: u32, round: u32, stage: usize, lap: u32,
    airlock_x: f32, depart_x: f32, booster_x: f32,
    asteroids: Vec<Asteroid>,
    minerals: Vec<Mineral>,
    mineral_next_id: u32,
    manip_len: f32,  // 0..MANIP_MAX_LEN
    tick: u64, stage_tick: u64,
    phase: Phase, phase_timer: u32,
    event: Option<&'static str>,
    dock_precision: u8,  // 0=perfect,1=good,2=ok (StageClear後に使う)
    booster_precision: u8,
    booster_enabled: bool,
    booster_attached: bool,
    booster_done: bool,
    keys_up: bool, keys_down: bool, keys_left: bool, keys_right: bool,
    keys_manip: bool,
}

impl Game {
    fn new() -> Self {
        Game {
            sx: W/2.0, sy: DEPART_Y, svx:0.0, svy:0.0,
            hp: SHIP_HP, invincible:0,
            score:0, round:1, stage:0, lap:1,
            airlock_x: pick_airlock_x(0,1), depart_x: W/2.0, booster_x: pick_booster_x(0,1),
            asteroids: Vec::new(), minerals: Vec::new(), mineral_next_id:0,
            manip_len:0.0,
            tick:0, stage_tick:0,
            phase: Phase::Launching, phase_timer:0,
            event: None, dock_precision:2, booster_precision:2, booster_enabled:false, booster_attached:false, booster_done:false,
            keys_up:false, keys_down:false, keys_left:false, keys_right:false, keys_manip:false,
        }
    }

    fn tick_game(&mut self) {
        self.tick += 1;
        self.event = None;

        match self.phase {
            Phase::StageClear | Phase::LapClear => {
                if self.phase_timer > 0 { self.phase_timer -= 1; }
                else { self.begin_next_stage(); }
                return;
            }
            Phase::GameOver => return,
            _ => {}
        }

        self.stage_tick += 1;
        if self.invincible > 0 { self.invincible -= 1; }

        // ── Launch ──
        if self.phase == Phase::Launching {
            let dist = self.sy - LAUNCH_TARGET_Y;
            if dist > 5.0 { self.svy -= SHIP_ACCEL * 0.6; }
            else { self.svy *= 0.80; }
            self.svx *= 0.85;
            self.svy *= SHIP_DRAG;
            let spd = (self.svx*self.svx+self.svy*self.svy).sqrt();
            if spd > SHIP_MAX_SPEED { self.svy = -SHIP_MAX_SPEED; self.svx = 0.0; }
            self.sx = (self.sx + self.svx).clamp(SHIP_R, W-SHIP_R);
            let ny = self.sy + self.svy;
            if ny < LAUNCH_TARGET_Y { self.sy = LAUNCH_TARGET_Y; self.svy = 0.0; }
            else { self.sy = ny.clamp(SHIP_R, H-SHIP_R); }
            if self.stage_tick >= LAUNCH_TICKS { self.phase = Phase::Playing; self.svy = 0.0; }
            return;
        }

        // ── Ship movement ──
        let ax = if self.keys_right { SHIP_ACCEL } else if self.keys_left { -SHIP_ACCEL } else { 0.0 };
        let ay = if self.keys_down  { SHIP_ACCEL } else if self.keys_up   { -SHIP_ACCEL } else { 0.0 };
        self.svx = (self.svx + ax) * SHIP_DRAG;
        self.svy = (self.svy + ay) * SHIP_DRAG;
        let spd = (self.svx*self.svx+self.svy*self.svy).sqrt();
        if spd > SHIP_MAX_SPEED { self.svx=self.svx/spd*SHIP_MAX_SPEED; self.svy=self.svy/spd*SHIP_MAX_SPEED; }
        self.sx = (self.sx + self.svx).clamp(SHIP_R, W-SHIP_R);
        self.sy = (self.sy + self.svy).clamp(SHIP_R, H-SHIP_R);

        let cruise_tick = self.stage_tick.saturating_sub(LAUNCH_TICKS);
        let progress = (cruise_tick as f32 / CRUISE_TICKS as f32).min(1.0);

        // ── Manip ──
        if self.keys_manip {
            self.manip_len = (self.manip_len + MANIP_GROW).min(MANIP_MAX_LEN);
        } else {
            self.manip_len = (self.manip_len - MANIP_SHRINK).max(0.0);
        }

        // ── Mineral collection ──
        if self.manip_len > 2.0 {
            let tip_x = self.sx;
            let tip_y = self.sy - SHIP_R - self.manip_len;
            let mut i = 0;
            while i < self.minerals.len() {
                let m = &self.minerals[i];
                let dx = tip_x - m.x;
                let dy = tip_y - m.y;
                if (dx*dx+dy*dy).sqrt() < MINERAL_R + 6.0 {
                    let bonus = match m.kind {
                        MineralKind::Gold => 300 * self.round,
                        MineralKind::Rare => 700 * self.round,
                    };
                    self.score += bonus;
                    let ev = match m.kind {
                        MineralKind::Gold => "mineral_gold",
                        MineralKind::Rare => "mineral_rare",
                    };
                    self.event = Some(ev);
                    self.minerals.swap_remove(i);
                } else { i += 1; }
            }
        }

        // ── Spawn asteroids (playing only) ──
        if self.phase == Phase::Playing {
            let interval = if self.booster_attached {
                spawn_interval_boosted(self.round, progress)
            } else {
                spawn_interval(self.round, progress)
            };
            if self.tick % interval == 0 {
                self.asteroids.push(spawn_asteroid(self.tick, self.asteroids.len(), self.round));
            }
            // Spawn minerals
            if self.tick % MINERAL_INTERVAL == 7 && cruise_tick > 30 {
                self.minerals.push(spawn_mineral(self.tick, self.mineral_next_id, self.round));
                self.mineral_next_id = self.mineral_next_id.wrapping_add(1);
            }
        }

        // ── Move asteroids ──
        let mut i = 0;
        while i < self.asteroids.len() {
            let a = &mut self.asteroids[i];
            a.x += a.vx;
            a.y += a.vy + if self.booster_attached { BOOSTER_SCROLL_BONUS_Y } else { 0.0 };
            if a.x < -a.radius-10.0 { a.x += W+a.radius*2.0; }
            if a.x > W+a.radius+10.0 { a.x -= W+a.radius*2.0; }
            if a.y > H+a.radius+20.0 { self.asteroids.swap_remove(i); } else { i+=1; }
        }

        // ── Move minerals ──
        let mut i = 0;
        while i < self.minerals.len() {
            let m = &mut self.minerals[i];
            m.x += m.vx;
            m.y += m.vy + if self.booster_attached { BOOSTER_SCROLL_BONUS_Y * 0.9 } else { 0.0 };
            if m.x < -MINERAL_R-10.0 { m.x += W+MINERAL_R*2.0; }
            if m.x > W+MINERAL_R+10.0 { m.x -= W+MINERAL_R*2.0; }
            if m.y > H+MINERAL_R+20.0 { self.minerals.swap_remove(i); } else { i+=1; }
        }

        // ── Collision (asteroids) ──
        if self.invincible == 0 && matches!(self.phase, Phase::Playing | Phase::BoosterDocking) {
            for a in &self.asteroids {
                let dx=self.sx-a.x; let dy=self.sy-a.y;
                if (dx*dx+dy*dy).sqrt() < SHIP_R+a.radius {
                    self.hp -= 1;
                    self.invincible = INVINCIBLE_TICKS;
                    self.event = Some("damage");
                    if self.hp == 0 { self.phase=Phase::GameOver; self.event=Some("gameover"); return; }
                    break;
                }
            }
        }

        // ── Enter booster docking phase ──
        if self.phase == Phase::Playing && self.booster_enabled && !self.booster_done && progress >= BOOSTER_TRIGGER_PCT {
            self.phase = Phase::BoosterDocking;
            self.asteroids.clear();
            self.minerals.clear();
            self.event = Some("booster_call");
        }

        // ── Check booster docking ──
        if self.phase == Phase::BoosterDocking {
            let dx = (self.sx - self.booster_x).abs();
            let dy = (self.sy - BOOSTER_Y).abs();
            if dy < DOCK_Y_R && dx < BOOSTER_X_OK {
                let (bonus, precision) = if dx < BOOSTER_X_PERFECT {
                    (SCORE_BOOSTER_PERFECT * self.round, 0u8)
                } else if dx < BOOSTER_X_GOOD {
                    (SCORE_BOOSTER_GOOD * self.round, 1u8)
                } else {
                    (SCORE_BOOSTER_OK * self.round, 2u8)
                };
                self.score += bonus;
                self.booster_precision = precision;
                self.booster_attached = true;
                self.booster_done = true;
                self.event = Some("booster_attach");
                self.phase = if progress >= 0.80 { Phase::Docking } else { Phase::Playing };
            }
        }

        // ── Enter docking phase ──
        if self.phase == Phase::Playing && progress >= 0.80 {
            self.phase = Phase::Docking;
            self.asteroids.clear();
        }

        // ── Check docking ──
        if self.phase == Phase::Docking {
            let dx = (self.sx - self.airlock_x).abs();
            let dy = (self.sy - AIRLOCK_Y).abs();
            if dy < DOCK_Y_R && dx < DOCK_X_OK {
                let (precision_bonus, precision) = if dx < DOCK_X_PERFECT {
                    (SCORE_DOCK_PERFECT * self.round, 0u8)
                } else if dx < DOCK_X_GOOD {
                    (SCORE_DOCK_GOOD * self.round, 1u8)
                } else {
                    (SCORE_DOCK_OK * self.round, 2u8)
                };
                self.score += SCORE_DELIVERY * self.round + precision_bonus;
                self.dock_precision = precision;
                let next = self.stage + 1;
                if next >= TOTAL_STATIONS {
                    let hp_bonus = self.hp as u32 * SCORE_HP_BONUS * self.round;
                    self.score += hp_bonus;
                    self.event = Some("lap_clear");
                    self.phase = Phase::LapClear;
                    self.phase_timer = 300;
                } else {
                    self.event = Some("delivery");
                    self.phase = Phase::StageClear;
                    self.phase_timer = 300;
                }
            }
        }
    }

    fn begin_next_stage(&mut self) {
        if self.stage + 1 >= TOTAL_STATIONS {
            self.round += 1; self.stage = 0; self.lap += 1; self.hp = SHIP_HP;
        } else {
            self.stage += 1;
        }
        self.depart_x  = self.airlock_x;
        self.airlock_x = pick_airlock_x(self.stage as u32, self.round);
        self.booster_x = pick_booster_x(self.stage as u32, self.round);
        self.sx = self.depart_x; self.sy = DEPART_Y;
        self.svx = 0.0; self.svy = 0.0;
        self.asteroids.clear(); self.minerals.clear();
        self.manip_len = 0.0; self.invincible = 0; self.stage_tick = 0;
        self.phase = Phase::Launching; self.event = None;
        self.booster_enabled = should_booster_stage(self.stage);
        self.booster_attached = false;
        self.booster_done = false;
        self.booster_precision = 2;
    }

    fn to_json(&self) -> String {
        let phase_str = match self.phase {
            Phase::Launching  => "launching",
            Phase::Playing    => "playing",
            Phase::BoosterDocking => "booster_docking",
            Phase::Docking    => "docking",
            Phase::StageClear => "stage_clear",
            Phase::LapClear   => "lap_clear",
            Phase::GameOver   => "gameover",
        };
        let ct = self.stage_tick.saturating_sub(LAUNCH_TICKS);
        let progress = (ct as f32 / CRUISE_TICKS as f32 * 100.0).min(100.0);
        let from_st = &STATIONS[self.stage % TOTAL_STATIONS];
        let to_st   = &STATIONS[(self.stage + 1) % TOTAL_STATIONS];
        let next_stage_booster = self.phase == Phase::StageClear && should_booster_stage(self.stage + 1);

        let asteroids: Vec<_> = self.asteroids.iter().map(|a|
            serde_json::json!({"x":a.x,"y":a.y,"r":a.radius,"tier":a.speed_tier,"seed":a.seed})
        ).collect();

        let minerals: Vec<_> = self.minerals.iter().map(|m|
            serde_json::json!({
                "x":m.x,"y":m.y,
                "kind": if m.kind==MineralKind::Gold {"gold"} else {"rare"},
                "seed":m.seed, "id":m.id,
            })
        ).collect();

        serde_json::json!({
            "type":"state","phase":phase_str,
            "ship":{"x":self.sx,"y":self.sy,"vx":self.svx,"vy":self.svy,"invincible":self.invincible>0},
            "manip_len": self.manip_len,
            "hp":self.hp,"score":self.score,"round":self.round,"stage":self.stage,"lap":self.lap,
            "progress":progress,"stage_tick":self.stage_tick,
            "airlock_x":self.airlock_x,"depart_x":self.depart_x,"booster_x":self.booster_x,
            "from":{"name_ja":from_st.0,"name_en":from_st.1,"symbol":from_st.2},
            "to":  {"name_ja":to_st.0,  "name_en":to_st.1,  "symbol":to_st.2},
            "asteroids":asteroids,"minerals":minerals,
            "event":self.event,"tick":self.tick,
            "dock_precision":self.dock_precision,
            "booster_precision":self.booster_precision,
            "booster_enabled":self.booster_enabled,
            "booster_attached":self.booster_attached,
            "fast_scroll":self.booster_attached,
            "next_stage_booster":next_stage_booster,
            "dock_x_ok": DOCK_X_OK,
            "booster_x_ok": BOOSTER_X_OK,
        }).to_string()
    }
}

fn pick_airlock_x(stage: u32, round: u32) -> f32 {
    let seed = lcg(stage.wrapping_mul(1009).wrapping_add(round * 997));
    90.0 + lcgf(seed) * (W - 180.0)
}

fn pick_booster_x(stage: u32, round: u32) -> f32 {
    let seed = lcg(stage.wrapping_mul(1619).wrapping_add(round * 1237).wrapping_add(77));
    100.0 + lcgf(seed) * (W - 200.0)
}

fn should_booster_stage(stage: usize) -> bool {
    stage > 0 && (stage + 1) % 3 == 1
}

// ── Input ─────────────────────────────────────────────────────────────────────
#[derive(Deserialize, Default)]
struct Keys {
    #[serde(default)] up: bool, #[serde(default)] down: bool,
    #[serde(default)] left: bool, #[serde(default)] right: bool,
    #[serde(default)] manip: bool,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Input { keys: Keys },
    Start { #[serde(default)] name: String },
    Restart,
    Continue,
}

fn score_list(scores: &Arc<Mutex<ScoreBoard>>) -> serde_json::Value {
    serde_json::json!(scores.lock().unwrap().list())
}

pub async fn run(mut socket: WebSocket, scores: Arc<Mutex<ScoreBoard>>, players: Arc<AtomicUsize>) {
    let _guard = PlayerCountGuard::new(players);
    let _ = socket.send(Message::Text(
        serde_json::json!({"type":"state","phase":"title","scores":score_list(&scores)})
        .to_string().into()
    )).await;

    loop {
        let name = loop {
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
        if game_session(&mut socket, &scores, &name).await { return; }
    }
}

async fn game_session(socket: &mut WebSocket, scores: &Arc<Mutex<ScoreBoard>>, player_name: &str) -> bool {
    let mut game = Game::new();
    let mut ticker = interval(Duration::from_millis(TICK_MS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                game.tick_game();
                if socket.send(Message::Text(game.to_json().into())).await.is_err() { return true; }
                if game.phase == Phase::GameOver {
                    let rank = { let mut sb=scores.lock().unwrap(); sb.add(player_name.to_string(), game.score) };
                    let _ = socket.send(Message::Text(
                        serde_json::json!({
                            "type":"state","phase":"gameover",
                            "score":game.score,"round":game.round,"lap":game.lap,
                            "rank":rank,"scores":score_list(scores),
                        }).to_string().into()
                    )).await;
                    loop {
                        match socket.recv().await {
                            Some(Ok(Message::Text(txt))) => {
                                if let Ok(ClientMsg::Restart) = serde_json::from_str(&txt) {
                                    let _ = socket.send(Message::Text(
                                        serde_json::json!({"type":"state","phase":"title","scores":score_list(scores)})
                                        .to_string().into()
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
                            game.keys_manip = keys.manip;
                        }
                        if let Ok(ClientMsg::Continue) = serde_json::from_str(&txt) {
                            if matches!(game.phase, Phase::StageClear | Phase::LapClear) {
                                game.phase_timer = 0;
                            }
                        }
                    }
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                    _ => {}
                }
            }
        }
    }
}
