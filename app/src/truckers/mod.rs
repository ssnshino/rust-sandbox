use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration, MissedTickBehavior};

use crate::scores::ScoreBoard;

const TICK_MS: u64 = 33;
//const W: f32 = 540.0;
//const H: f32 = 540.0;
// @@@ 20260418 change. 540 -> 667 -> 600
const W: f32 = 375.0;
const H: f32 = 500.0;

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
const FUEL_STAND_TRIGGER_PCT: f32 = 0.50;
const FUEL_STAND_Y: f32 = 250.0;
const FUEL_STAND_X_PERFECT: f32 = 7.0;
const FUEL_STAND_X_GOOD: f32 = 16.0;
const FUEL_STAND_X_OK: f32 = 26.0;

const DOCK_Y_R: f32 = 35.0; // 縦方向のドッキング許容範囲
const DOCK_X_PERFECT: f32 = 5.0; // 完璧ドッキング
const DOCK_X_GOOD: f32 = 12.0;
const DOCK_X_OK: f32 = 22.0; // これ以上離れると不可
const AIRLOCK_Y: f32 = 90.0;
const DEPART_Y: f32 = H - 40.0;

const SHIP_HP_MAX: u16 = 100;
const SHIP_FUEL_MAX: f32 = 100.0;
const SCORE_DELIVERY: u32 = 500;
const SCORE_INTEGRITY_BONUS: u32 = 20;
const SCORE_DOCK_PERFECT: u32 = 2000;
const SCORE_DOCK_GOOD: u32 = 1000;
const SCORE_DOCK_OK: u32 = 400;
const SCORE_BOOSTER_PERFECT: u32 = 1200;
const SCORE_BOOSTER_GOOD: u32 = 700;
const SCORE_BOOSTER_OK: u32 = 300;
const MONEY_GOLD: u32 = 500;
const MONEY_RARE: u32 = 100;
const BOOSTER_COST: u32 = 1000;
const LATE_FINE: u32 = 200;
const REPAIR_COST_PER_POINT: u32 = 6;
const FUEL_COST_PER_POINT: u32 = 4;
const TOTAL_STATIONS: usize = 12;

// マニピュレーター
const MANIP_MAX_LEN: f32 = 44.0;

// 鉱石スポーン間隔
const MINERAL_INTERVAL: u64 = 60;
const MINERAL_R: f32 = 5.0;

const STATIONS: [(&str, &str, &str); 12] = [
    ("おひつじ", "Aries", "♈"),
    ("おうし", "Taurus", "♉"),
    ("ふたご", "Gemini", "♊"),
    ("かに", "Cancer", "♋"),
    ("しし", "Leo", "♌"),
    ("おとめ", "Virgo", "♍"),
    ("てんびん", "Libra", "♎"),
    ("さそり", "Scorpio", "♏"),
    ("いて", "Sagittarius", "♐"),
    ("やぎ", "Capricorn", "♑"),
    ("みずがめ", "Aquarius", "♒"),
    ("うお", "Pisces", "♓"),
];

struct PlayerCountGuard(Arc<AtomicUsize>);
impl PlayerCountGuard {
    fn new(c: Arc<AtomicUsize>) -> Self {
        c.fetch_add(1, Ordering::Relaxed);
        Self(c)
    }
}
impl Drop for PlayerCountGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

fn lcg(s: u32) -> u32 {
    s.wrapping_mul(1664525).wrapping_add(1013904223)
}
fn lcgf(s: u32) -> f32 {
    lcg(s) as f32 / u32::MAX as f32
}

// 小数点一桁に丸める @@@ 20260420 add.
fn r1(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

/*
fn r1(v: f32) -> i32 {
    (v * 10.0).round() / 10.0 as i32
}
*/
// ── Asteroid ──────────────────────────────────────────────────────────────────

struct Asteroid {
    x: f32,
    vx: f32,
    vy: f32,
    radius: f32,
    speed_tier: u8,
    seed: u32,
}

const SPEED_TIERS: [f32; 5] = [1.4, 2.4, 3.6, 5.0, 7.0];
const SIZE_TIERS: [f32; 5] = [5.0, 9.0, 14.0, 20.0, 27.0];

fn round_scale(round: u32) -> f32 {
    1.0 + (round.saturating_sub(1) as f32) * 0.22
}

fn spawn_interval(round: u32, progress: f32) -> u64 {
    let base = 28u64;
    let rr = (round.saturating_sub(1) as u64) * 3;
    let pr = (progress * 16.0) as u64;
    base.saturating_sub(rr + pr).max(8)
}

fn spawn_asteroid(tick: u64, i: usize, round: u32) -> Asteroid {
    let s0 = lcg((tick as u32)
        .wrapping_add(i as u32 * 6991)
        .wrapping_add(round * 54321));
    let s1 = lcg(s0);
    let s2 = lcg(s1);
    let s3 = lcg(s2);
    let s4 = lcg(s3);
    let x = 18.0 + lcgf(s0) * (W - 36.0);
    let vx = (lcgf(s1) - 0.5) * 1.8 * (1.0 + (round as f32 - 1.0) * 0.08);
    let st = (lcg(s2) % 5) as usize;
    let vy = SPEED_TIERS[st] * round_scale(round);
    let sz = (lcg(s3) % 5) as usize;
    Asteroid {
        x,
        vx,
        vy,
        radius: SIZE_TIERS[sz],
        speed_tier: st as u8,
        seed: s4,
    }
}

fn build_asteroid_plan(stage: usize, round: u32) -> Vec<serde_json::Value> {
    let mut route_tick = 0u64;
    let mut index = 0usize;
    let mut plan = Vec::new();
    while route_tick <= CRUISE_TICKS + 120 {
        let progress = (route_tick as f32 / CRUISE_TICKS as f32).min(1.0);
        let mut interval = spawn_interval(round, progress);
        if is_dense_route(stage) {
            interval = (interval / 2).max(4);
        }
        route_tick += interval;
        let seed_tick = route_tick
            .wrapping_mul(37)
            .wrapping_add(index as u64 * 101)
            .wrapping_add(stage as u64 * 503);
        let jitter = (lcg(seed_tick as u32) % interval.max(1) as u32) as u64;
        let spawn_at = route_tick.saturating_add(jitter / 2);
        let asteroid = spawn_asteroid(seed_tick, index, round);
        plan.push(serde_json::json!({
            "spawn_at": spawn_at,
            "x": asteroid.x,
            "vx": r1(asteroid.vx),
            "vy": r1(asteroid.vy),
            "r": asteroid.radius,
            "tier": asteroid.speed_tier,
            "seed": asteroid.seed,
        }));
        index += 1;
    }
    plan
}

fn build_mineral_plan(round: u32) -> Vec<serde_json::Value> {
    let mut plan = Vec::new();
    let mut route_tick = 37u64;
    let mut id = 0u32;
    while route_tick <= CRUISE_TICKS + 120 {
        let mineral = spawn_mineral(route_tick, id, round);
        plan.push(serde_json::json!({
            "spawn_at": route_tick,
            "x": mineral.x,
            "vx": r1(mineral.vx),
            "vy": r1(mineral.vy),
            "kind": if mineral.kind==MineralKind::Gold {"gold"} else {"rare"},
            "seed": mineral.seed,
            "id": id,
        }));
        route_tick += MINERAL_INTERVAL;
        id = id.wrapping_add(1);
    }
    plan
}

// ── Mineral ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum MineralKind {
    Gold,
    Rare,
}

struct Mineral {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    kind: MineralKind,
    seed: u32,
    id: u32,
}

fn spawn_mineral(tick: u64, id: u32, round: u32) -> Mineral {
    let s0 = lcg((tick as u32)
        .wrapping_add(id * 8191)
        .wrapping_add(round * 11111));
    let s1 = lcg(s0);
    let s2 = lcg(s1);
    let s3 = lcg(s2);
    let x = 24.0 + lcgf(s0) * (W - 48.0);
    let vx = (lcgf(s1) - 0.5) * 0.6;
    let vy = 0.4 + lcgf(s2) * 0.5; // ゆっくり落ちる
    let kind = if lcgf(s3) < 0.65 {
        MineralKind::Gold
    } else {
        MineralKind::Rare
    };
    Mineral {
        x,
        y: -MINERAL_R - 2.0,
        vx,
        vy,
        kind,
        seed: s0,
        id,
    }
}

// ── Phase ─────────────────────────────────────────────────────────────────────
#[derive(Debug, PartialEq, Clone, Copy)]
enum Phase {
    Launching,
    Playing,
    BoosterDocking,
    FuelDocking,
    Docking,
    StageClear,
    LapClear,
    GameOver,
}

impl Phase {
    fn as_str(self) -> &'static str {
        match self {
            Phase::Launching => "launching",
            Phase::Playing => "playing",
            Phase::BoosterDocking => "booster_docking",
            Phase::FuelDocking => "fuel_docking",
            Phase::Docking => "docking",
            Phase::StageClear => "stage_clear",
            Phase::LapClear => "lap_clear",
            Phase::GameOver => "gameover",
        }
    }
}

// ── Game ──────────────────────────────────────────────────────────────────────
struct Game {
    sx: f32,
    sy: f32,
    svx: f32,
    svy: f32,
    hp: u16,
    fuel: f32,
    invincible: u32,
    score: u32,
    money: u32,
    round: u32,
    stage: usize,
    lap: u32,
    airlock_x: f32,
    depart_x: f32,
    booster_x: f32,
    fuel_stand_x: f32,
    asteroids: Vec<Asteroid>,
    minerals: Vec<Mineral>,
    manip_len: f32, // 0..MANIP_MAX_LEN
    tick: u64,
    stage_tick: u64,
    phase: Phase,
    phase_timer: u32,
    event: Option<&'static str>,
    dock_precision: u8, // 0=perfect,1=good,2=ok (StageClear後に使う)
    booster_precision: u8,
    booster_enabled: bool,
    booster_attached: bool,
    booster_done: bool,
    booster_done_tick: u64,
    fuel_stand_enabled: bool,
    fuel_stand_done: bool,
    fuel_stand_done_tick: u64,
    late_fined: bool,
    gold_count: u32,
    rare_count: u32,
    last_delivery_score: u32,
    last_dock_bonus: u32,
    last_cargo_bonus: u32,
    last_hp_bonus: u32,
    last_late_fine: u32,
    last_booster_cost: u32,
    last_repair_cost: u32,
    last_fuel_cost: u32,
    last_fuel_stand_cost: u32,
}

impl Game {
    fn apply_service_costs(&mut self) {
        let repair_needed = SHIP_HP_MAX.saturating_sub(self.hp) as u32;
        let fuel_needed = (SHIP_FUEL_MAX - self.fuel).max(0.0).ceil() as u32;

        let repair_points = repair_needed.min(self.money / REPAIR_COST_PER_POINT);
        let repair_cost = repair_points * REPAIR_COST_PER_POINT;
        self.money = self.money.saturating_sub(repair_cost);
        self.hp = (self.hp as u32 + repair_points).min(SHIP_HP_MAX as u32) as u16;
        self.last_repair_cost = repair_cost;

        let fuel_points = fuel_needed.min(self.money / FUEL_COST_PER_POINT);
        let fuel_cost = fuel_points * FUEL_COST_PER_POINT;
        self.money = self.money.saturating_sub(fuel_cost);
        self.fuel = (self.fuel + fuel_points as f32).min(SHIP_FUEL_MAX);
        self.last_fuel_cost = fuel_cost;
    }

    fn apply_client_state(&mut self, ship: ClientShip, hp: u16, fuel: f32, manip_len: f32) {
        // Accept browser simulation only while the ship is controllable.
        // This keeps stale packets from changing title, clear or game-over state.
        if !matches!(
            self.phase,
            Phase::Playing | Phase::BoosterDocking | Phase::FuelDocking | Phase::Docking
        ) {
            return;
        }

        self.sx = ship.x.clamp(SHIP_R, W - SHIP_R);
        self.sy = ship.y.clamp(SHIP_R, H - SHIP_R);
        self.svx = ship.vx.clamp(-SHIP_MAX_SPEED, SHIP_MAX_SPEED);
        self.svy = ship.vy.clamp(-SHIP_MAX_SPEED, SHIP_MAX_SPEED);
        self.hp = hp.min(SHIP_HP_MAX);
        self.fuel = fuel.clamp(0.0, SHIP_FUEL_MAX);
        self.manip_len = manip_len.clamp(0.0, MANIP_MAX_LEN);

        if self.hp == 0 {
            self.phase = Phase::GameOver;
            self.event = Some("gameover");
        } else if self.fuel <= 0.0 {
            self.phase = Phase::GameOver;
            self.event = Some("fuel_empty");
        }
    }

    fn collect_client_mineral(&mut self, mineral_id: u32, mineral_kind: Option<String>) {
        if !matches!(
            self.phase,
            Phase::Playing | Phase::BoosterDocking | Phase::FuelDocking | Phase::Docking
        ) {
            return;
        }

        let kind = if let Some(index) = self.minerals.iter().position(|m| m.id == mineral_id) {
            self.minerals.swap_remove(index).kind
        } else if mineral_kind.as_deref() == Some("rare") {
            MineralKind::Rare
        } else {
            MineralKind::Gold
        };
        let bonus = match kind {
            MineralKind::Gold => 300 * self.round,
            MineralKind::Rare => 700 * self.round,
        };
        self.score += bonus;
        match kind {
            MineralKind::Gold => {
                self.money += MONEY_GOLD;
                self.gold_count += 1;
                self.event = Some("mineral_gold");
            }
            MineralKind::Rare => {
                self.money += MONEY_RARE;
                self.rare_count += 1;
                self.event = Some("mineral_rare");
            }
        }
    }

    fn new() -> Self {
        Game {
            sx: W / 2.0,
            sy: DEPART_Y,
            svx: 0.0,
            svy: 0.0,
            hp: SHIP_HP_MAX,
            fuel: SHIP_FUEL_MAX,
            invincible: 0,
            score: 0,
            money: 0,
            round: 1,
            stage: 0,
            lap: 1,
            airlock_x: pick_airlock_x(0, 1),
            depart_x: W / 2.0,
            booster_x: pick_booster_x(0, 1),
            fuel_stand_x: pick_fuel_stand_x(0, 1),
            asteroids: Vec::new(),
            minerals: Vec::new(),
            manip_len: 0.0,
            tick: 0,
            stage_tick: 0,
            phase: Phase::Launching,
            phase_timer: 0,
            event: None,
            dock_precision: 2,
            booster_precision: 2,
            booster_enabled: false,
            booster_attached: false,
            booster_done: false,
            booster_done_tick: 0,
            fuel_stand_enabled: is_dense_route(0),
            fuel_stand_done: false,
            fuel_stand_done_tick: 0,
            late_fined: false,
            gold_count: 0,
            rare_count: 0,
            last_delivery_score: 0,
            last_dock_bonus: 0,
            last_cargo_bonus: 0,
            last_hp_bonus: 0,
            last_late_fine: 0,
            last_booster_cost: 0,
            last_repair_cost: 0,
            last_fuel_cost: 0,
            last_fuel_stand_cost: 0,
        }
    }

    fn tick_game(&mut self) {
        self.tick += 1;
        self.event = None;

        match self.phase {
            Phase::StageClear | Phase::LapClear => {
                if self.phase_timer > 0 {
                    self.phase_timer -= 1;
                }
                return;
            }
            Phase::GameOver => return,
            _ => {}
        }

        let route_time_paused = matches!(self.phase, Phase::BoosterDocking | Phase::FuelDocking);
        if !route_time_paused {
            self.stage_tick += 1;
        }
        if self.invincible > 0 {
            self.invincible -= 1;
        }

        // ── Launch ──
        if self.phase == Phase::Launching {
            let dist = self.sy - LAUNCH_TARGET_Y;
            if dist > 5.0 {
                self.svy -= SHIP_ACCEL * 0.6;
            } else {
                self.svy *= 0.80;
            }
            self.svx *= 0.85;
            self.svy *= SHIP_DRAG;
            let spd = (self.svx * self.svx + self.svy * self.svy).sqrt();
            if spd > SHIP_MAX_SPEED {
                self.svy = -SHIP_MAX_SPEED;
                self.svx = 0.0;
            }
            self.sx = (self.sx + self.svx).clamp(SHIP_R, W - SHIP_R);
            let ny = self.sy + self.svy;
            if ny < LAUNCH_TARGET_Y {
                self.sy = LAUNCH_TARGET_Y;
                self.svy = 0.0;
            } else {
                self.sy = ny.clamp(SHIP_R, H - SHIP_R);
            }
            if self.stage_tick >= LAUNCH_TICKS {
                self.phase = Phase::Playing;
                self.svy = 0.0;
            }
            return;
        }

        let cruise_tick = self.stage_tick.saturating_sub(LAUNCH_TICKS);
        let progress = (cruise_tick as f32 / CRUISE_TICKS as f32).min(1.0);

        // ── Mineral collection ──
        if self.manip_len > 2.0 {
            let tip_x = self.sx;
            let tip_y = self.sy - SHIP_R - self.manip_len;
            let mut i = 0;
            while i < self.minerals.len() {
                let m = &self.minerals[i];
                let dx = tip_x - m.x;
                let dy = tip_y - m.y;
                if (dx * dx + dy * dy).sqrt() < MINERAL_R + 6.0 {
                    let bonus = match m.kind {
                        MineralKind::Gold => 300 * self.round,
                        MineralKind::Rare => 700 * self.round,
                    };
                    self.score += bonus;
                    match m.kind {
                        MineralKind::Gold => {
                            self.money += MONEY_GOLD;
                            self.gold_count += 1;
                        }
                        MineralKind::Rare => {
                            self.money += MONEY_RARE;
                            self.rare_count += 1;
                        }
                    }
                    let ev = match m.kind {
                        MineralKind::Gold => "mineral_gold",
                        MineralKind::Rare => "mineral_rare",
                    };
                    self.event = Some(ev);
                    self.minerals.swap_remove(i);
                } else {
                    i += 1;
                }
            }
        }

        // ── Enter booster docking phase ──
        if self.phase == Phase::Playing
            && self.booster_enabled
            && !self.booster_done
            && progress >= BOOSTER_TRIGGER_PCT
        {
            self.phase = Phase::BoosterDocking;
            self.asteroids.clear();
            self.event = Some("booster_call");
        }

        // ── Check booster docking ──
        if self.phase == Phase::BoosterDocking {
            let dx = (self.sx - self.booster_x).abs();
            let dy = (self.sy - BOOSTER_Y).abs();
            if dy < DOCK_Y_R && dx < BOOSTER_X_OK {
                if self.money < BOOSTER_COST {
                    self.event = Some("booster_fee_short");
                    self.booster_done = true;
                    self.booster_done_tick = self.tick;
                    self.booster_enabled = false;
                    self.phase = if progress >= 0.80 {
                        Phase::Docking
                    } else {
                        Phase::Playing
                    };
                    return;
                }
                let (bonus, precision) = if dx < BOOSTER_X_PERFECT {
                    (SCORE_BOOSTER_PERFECT * self.round, 0u8)
                } else if dx < BOOSTER_X_GOOD {
                    (SCORE_BOOSTER_GOOD * self.round, 1u8)
                } else {
                    (SCORE_BOOSTER_OK * self.round, 2u8)
                };
                self.score += bonus;
                self.money = self.money.saturating_sub(BOOSTER_COST);
                self.booster_precision = precision;
                self.last_booster_cost = BOOSTER_COST;
                self.booster_attached = true;
                self.booster_done = true;
                self.booster_done_tick = self.tick;
                self.event = Some("booster_attach");
                self.phase = if progress >= 0.80 {
                    Phase::Docking
                } else {
                    Phase::Playing
                };
            }
        }

        // ── Enter fuel stand docking phase on dense asteroid routes ──
        if self.phase == Phase::Playing
            && self.fuel_stand_enabled
            && !self.fuel_stand_done
            && progress >= FUEL_STAND_TRIGGER_PCT
        {
            self.phase = Phase::FuelDocking;
            self.asteroids.clear();
            self.event = Some("fuel_stand_call");
        }

        // ── Check fuel stand docking ──
        if self.phase == Phase::FuelDocking {
            let dx = (self.sx - self.fuel_stand_x).abs();
            let dy = (self.sy - FUEL_STAND_Y).abs();
            if dy < DOCK_Y_R && dx < FUEL_STAND_X_OK {
                let fuel_needed = (SHIP_FUEL_MAX - self.fuel).max(0.0).ceil() as u32;
                let fuel_points = fuel_needed.min(self.money / FUEL_COST_PER_POINT);
                if fuel_needed > 0 && fuel_points == 0 {
                    self.event = Some("fuel_stand_fee_short");
                    self.last_fuel_stand_cost = 0;
                } else {
                    let cost = fuel_points * FUEL_COST_PER_POINT;
                    self.money = self.money.saturating_sub(cost);
                    self.fuel = (self.fuel + fuel_points as f32).min(SHIP_FUEL_MAX);
                    self.last_fuel_stand_cost = cost;
                    let (bonus, _precision) = if dx < FUEL_STAND_X_PERFECT {
                        (300 * self.round, 0u8)
                    } else if dx < FUEL_STAND_X_GOOD {
                        (150 * self.round, 1u8)
                    } else {
                        (50 * self.round, 2u8)
                    };
                    self.score += bonus;
                    self.event = Some("fuel_stand_refuel");
                }
                self.fuel_stand_done = true;
                self.fuel_stand_done_tick = self.tick;
                self.phase = if progress >= 0.80 {
                    Phase::Docking
                } else {
                    Phase::Playing
                };
                return;
            }
        }

        // ── Enter docking phase ──
        if self.phase == Phase::Playing && progress >= 0.80 {
            self.phase = Phase::Docking;
            self.asteroids.clear();
        }

        if self.phase == Phase::Docking && progress >= 1.0 && !self.late_fined {
            self.money = self.money.saturating_sub(LATE_FINE);
            self.last_late_fine = LATE_FINE;
            self.late_fined = true;
            self.event = Some("late_fine");
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
                let delivery_score = SCORE_DELIVERY * self.round;
                let cargo_bonus = self.gold_count * MONEY_GOLD + self.rare_count * MONEY_RARE;
                self.last_delivery_score = delivery_score;
                self.last_dock_bonus = precision_bonus;
                self.last_cargo_bonus = cargo_bonus;
                self.dock_precision = precision;
                self.score += delivery_score + precision_bonus + cargo_bonus;
                self.apply_service_costs();
                let next = self.stage + 1;
                if next >= TOTAL_STATIONS {
                    let hp_bonus = self.hp as u32 * SCORE_INTEGRITY_BONUS * self.round;
                    self.last_hp_bonus = hp_bonus;
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
            self.round += 1;
            self.stage = 0;
            self.lap += 1;
        } else {
            self.stage += 1;
        }
        self.depart_x = self.airlock_x;
        self.airlock_x = pick_airlock_x(self.stage as u32, self.round);
        self.booster_x = pick_booster_x(self.stage as u32, self.round);
        self.fuel_stand_x = pick_fuel_stand_x(self.stage as u32, self.round);
        self.sx = self.depart_x;
        self.sy = DEPART_Y;
        self.svx = 0.0;
        self.svy = 0.0;
        self.asteroids.clear();
        self.minerals.clear();
        self.manip_len = 0.0;
        self.invincible = 0;
        self.stage_tick = 0;
        self.phase = Phase::Launching;
        self.event = None;
        self.late_fined = false;
        self.booster_enabled = should_booster_stage(self.stage);
        self.booster_attached = false;
        self.booster_done = false;
        self.booster_done_tick = 0;
        self.booster_precision = 2;
        self.fuel_stand_enabled = is_dense_route(self.stage);
        self.fuel_stand_done = false;
        self.fuel_stand_done_tick = 0;
        self.gold_count = 0;
        self.rare_count = 0;
        self.last_delivery_score = 0;
        self.last_dock_bonus = 0;
        self.last_cargo_bonus = 0;
        self.last_hp_bonus = 0;
        self.last_late_fine = 0;
        self.last_booster_cost = 0;
        self.last_repair_cost = 0;
        self.last_fuel_cost = 0;
        self.last_fuel_stand_cost = 0;
    }

    fn to_json(&self) -> String {
        let phase_str = self.phase.as_str();
        let ct = self.stage_tick.saturating_sub(LAUNCH_TICKS);
        let progress = (ct as f32 / CRUISE_TICKS as f32 * 100.0).min(100.0);
        let from_st = &STATIONS[self.stage % TOTAL_STATIONS];
        let to_st = &STATIONS[(self.stage + 1) % TOTAL_STATIONS];
        let next_stage_booster =
            self.phase == Phase::StageClear && should_booster_stage(self.stage + 1);

        let asteroids: Vec<serde_json::Value> = Vec::new();
        let asteroid_plan = if self.phase == Phase::Launching && self.stage_tick <= 10 {
            build_asteroid_plan(self.stage, self.round)
        } else {
            Vec::new()
        };
        let mineral_plan = if self.phase == Phase::Launching && self.stage_tick <= 10 {
            build_mineral_plan(self.round)
        } else {
            Vec::new()
        };
        let route_config = if self.phase == Phase::Launching && self.stage_tick <= 10 {
            route_config_json()
        } else {
            serde_json::Value::Null
        };

        let minerals: Vec<serde_json::Value> = Vec::new();

        serde_json::json!({
            "type":"state","phase":phase_str,
            "ship":{
                "x":r1(self.sx),
                "y":r1(self.sy),
                "vx":r1(self.svx),
                "vy":r1(self.svy),
                "invincible":self.invincible>0
            },
            "manip_len": self.manip_len,
            "hp":self.hp,"fuel":((self.fuel * 10.0).round() / 10.0),"score":self.score,"money":self.money,"round":self.round,"stage":self.stage,"lap":self.lap,
            "progress":progress,"stage_tick":self.stage_tick,
            "route_paused":matches!(self.phase, Phase::BoosterDocking | Phase::FuelDocking | Phase::Docking),
            "asteroid_plan":asteroid_plan,
            "mineral_plan":mineral_plan,
            "route_config":route_config,
            "airlock_x":r1(self.airlock_x),
            "depart_x":r1(self.depart_x),
            "booster_x":r1(self.booster_x),
            "fuel_stand_x":r1(self.fuel_stand_x),
            "from":{"name_ja":from_st.0,"name_en":from_st.1,"symbol":from_st.2},
            "to":  {"name_ja":to_st.0,  "name_en":to_st.1,  "symbol":to_st.2},
            "asteroids":asteroids,"minerals":minerals,
            "event":self.event,"tick":self.tick,
            "gold_count": self.gold_count,
            "rare_count": self.rare_count,
            "delivery_score": self.last_delivery_score,
            "dock_bonus": self.last_dock_bonus,
            "cargo_bonus": self.last_cargo_bonus,
            "hp_bonus": self.last_hp_bonus,
            "late_fine": self.last_late_fine,
            "booster_cost": self.last_booster_cost,
            "repair_cost": self.last_repair_cost,
            "fuel_cost": self.last_fuel_cost,
            "fuel_stand_cost": self.last_fuel_stand_cost,
            "dock_precision":self.dock_precision,
            "booster_precision":self.booster_precision,
            "booster_enabled":self.booster_enabled,
            "booster_attached":self.booster_attached,
            "booster_done_tick":self.booster_done_tick,
            "fuel_stand_enabled":self.fuel_stand_enabled,
            "fuel_stand_done":self.fuel_stand_done,
            "fuel_stand_done_tick":self.fuel_stand_done_tick,
            "dense_route":self.fuel_stand_enabled,
            "fast_scroll":self.booster_attached,
            "next_stage_booster":next_stage_booster,
        }).to_string()
    }
}

fn pick_airlock_x(stage: u32, round: u32) -> f32 {
    let seed = lcg(stage.wrapping_mul(1009).wrapping_add(round * 997));
    90.0 + lcgf(seed) * (W - 180.0)
}

fn route_config_json() -> serde_json::Value {
    serde_json::json!({
        "dock_y": AIRLOCK_Y,
        "dock_x_ok": DOCK_X_OK,
        "booster_y": BOOSTER_Y,
        "booster_x_ok": BOOSTER_X_OK,
        "fuel_stand_y": FUEL_STAND_Y,
        "fuel_stand_x_ok": FUEL_STAND_X_OK,
    })
}

fn pick_booster_x(stage: u32, round: u32) -> f32 {
    let seed = lcg(stage
        .wrapping_mul(1619)
        .wrapping_add(round * 1237)
        .wrapping_add(77));
    100.0 + lcgf(seed) * (W - 200.0)
}

fn should_booster_stage(stage: usize) -> bool {
    stage > 0 && (stage + 1) % 3 == 1
}

fn is_dense_route(stage: usize) -> bool {
    let destination_station_no = ((stage + 1) % TOTAL_STATIONS) + 1;
    matches!(destination_station_no, 4 | 8 | 12)
}

fn pick_fuel_stand_x(stage: u32, round: u32) -> f32 {
    let seed = lcg(stage
        .wrapping_mul(2179)
        .wrapping_add(round * 1423)
        .wrapping_add(191));
    88.0 + lcgf(seed) * (W - 176.0)
}

// ── Input ─────────────────────────────────────────────────────────────────────
#[derive(Deserialize)]
struct ClientShip {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

// The browser owns moment-to-moment ship simulation.
// It notifies the server only when a meaningful gameplay event happens.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Start {
        #[serde(default)]
        name: String,
    },
    ClientEvent {
        ship: ClientShip,
        hp: u16,
        fuel: f32,
        manip_len: f32,
        #[serde(default)]
        mineral_id: Option<u32>,
        #[serde(default)]
        mineral_kind: Option<String>,
    },
    Restart {
        #[serde(default)]
        name: String,
    },
    Continue,
}

fn score_list(scores: &Arc<Mutex<ScoreBoard>>) -> serde_json::Value {
    serde_json::json!(scores.lock().unwrap().list())
}

fn title_state(scores: &Arc<Mutex<ScoreBoard>>) -> String {
    serde_json::json!({"type":"state","phase":"title","scores":score_list(scores)}).to_string()
}

fn should_send_state(game: &Game, last_sent_phase: Phase) -> bool {
    game.event.is_some()
        || (game.phase == Phase::Launching && game.stage_tick <= 3)
        || game.phase != last_sent_phase
        || matches!(game.phase, Phase::GameOver)
}

async fn send_state(socket: &mut WebSocket, game: &Game) -> bool {
    socket
        .send(Message::Text(game.to_json().into()))
        .await
        .is_err()
}

async fn send_gameover_state(
    socket: &mut WebSocket,
    game: &Game,
) -> bool {
    socket
        .send(Message::Text(
            serde_json::json!({
                "type":"state","phase":"gameover",
                "score":game.score,"round":game.round,"lap":game.lap,
                "rank":null,"scores":[],
            })
            .to_string()
            .into(),
        ))
        .await
        .is_err()
}

fn register_score(scores: &Arc<Mutex<ScoreBoard>>, name: String, score: u32) {
    let final_name = if name.trim().is_empty() {
        "野郎".to_string()
    } else {
        name.chars().take(20).collect()
    };
    let mut sb = scores.lock().unwrap();
    let _ = sb.add(final_name, score);
}

pub async fn run(mut socket: WebSocket, scores: Arc<Mutex<ScoreBoard>>, players: Arc<AtomicUsize>) {
    let _guard = PlayerCountGuard::new(players);
    let _ = socket
        .send(Message::Text(title_state(&scores).into()))
        .await;

    loop {
        loop {
            match socket.recv().await {
                Some(Ok(Message::Text(txt))) => {
                    if let Ok(ClientMsg::Start { .. }) = serde_json::from_str(&txt) {
                        break;
                    }
                }
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return,
                _ => {}
            }
        };
        if game_session(&mut socket, &scores).await {
            return;
        }
    }
}

async fn game_session(
    socket: &mut WebSocket,
    scores: &Arc<Mutex<ScoreBoard>>,
) -> bool {
    let mut game = Game::new();
    let mut last_sent_phase = game.phase;
    let mut ticker = interval(Duration::from_millis(TICK_MS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                game.tick_game();
                if should_send_state(&game, last_sent_phase) {
                    last_sent_phase = game.phase;
                    if send_state(socket, &game).await { return true; }
                }
                if game.phase == Phase::GameOver {
                    let _ = send_gameover_state(socket, &game).await;
                    loop {
                        match socket.recv().await {
                            Some(Ok(Message::Text(txt))) => {
                                if let Ok(ClientMsg::Restart { name }) = serde_json::from_str(&txt) {
                                    register_score(scores, name, game.score);
                                    let _ = socket.send(Message::Text(
                                        title_state(scores).into()
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
                        if let Ok(ClientMsg::ClientEvent { ship, hp, fuel, manip_len, mineral_id, mineral_kind }) = serde_json::from_str(&txt) {
                            game.apply_client_state(ship, hp, fuel, manip_len);
                            if let Some(mineral_id) = mineral_id {
                                game.collect_client_mineral(mineral_id, mineral_kind);
                            }
                            if game.event.is_some() || matches!(game.phase, Phase::GameOver) {
                                if send_state(socket, &game).await { return true; }
                            }
                        }
                        if let Ok(ClientMsg::Continue) = serde_json::from_str(&txt) {
                            if matches!(game.phase, Phase::StageClear | Phase::LapClear) {
                                game.begin_next_stage();
                            }
                        }
                        if let Ok(ClientMsg::Start { .. }) = serde_json::from_str(&txt) {
                            if matches!(game.phase, Phase::StageClear | Phase::LapClear) {
                                game.begin_next_stage();
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
