use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::{Mutex as AsyncMutex, Notify};
use tokio::time::{interval, Duration};

use crate::scores::ScoreBoard;

const TICK_MS: u64 = 33;
const GROUND_Y: f32 = 300.0;
const STAGE_W: f32 = 540.0;
const P_MAX_HP: u8 = 10;
const GRAVITY: f32 = 1.2;
const JUMP_VY: f32 = -18.0;
const DIAG_JUMP_VX: f32 = 4.0;
const WALK_SPD: f32 = 3.5;
const PUNCH_DMG: u8 = 1;
const KICK_DMG: u8 = 2;
const HURT_TICKS: u32 = 14;
const INVINCIBLE_TICKS: u32 = 22;
const ATTACK_TICKS: u32 = 18;
const ATTACK_HIT_START: u32 = 6;
const ATTACK_HIT_END: u32 = 12;
const MAX_STAGE: u32 = 5;

struct PlayerCountGuard(Arc<AtomicUsize>);
impl PlayerCountGuard {
    fn new(c: Arc<AtomicUsize>) -> Self { c.fetch_add(1, Ordering::Relaxed); Self(c) }
}
impl Drop for PlayerCountGuard {
    fn drop(&mut self) { self.0.fetch_sub(1, Ordering::Relaxed); }
}

// ── Fighter ───────────────────────────────────────────────────────────────────
#[derive(Clone, PartialEq, Debug)]
enum FightState {
    Idle, WalkF, WalkB,
    Punch, Kick, CrouchIdle, CrouchWalkF, CrouchWalkB, CrouchPunch, CrouchKick,
    Jump, JumpPunch, JumpKick,
    Guard,
    Hurt, Dead,
}

#[derive(Clone)]
struct Fighter {
    x: f32, y: f32, vy: f32, vx: f32,
    hp: u8, max_hp: u8,
    state: FightState,
    state_timer: u32,
    facing: i8,
    invincible: u32,
}

impl Fighter {
    fn new(x: f32, facing: i8, hp: u8) -> Self {
        Fighter { x, y: GROUND_Y, vy: 0.0, vx: 0.0, hp, max_hp: hp,
            state: FightState::Idle, state_timer: 0, facing, invincible: 0 }
    }
    fn is_dead(&self) -> bool { self.state == FightState::Dead }
    fn on_ground(&self) -> bool { self.y >= GROUND_Y - 0.1 }
    fn is_attacking(&self) -> bool {
        matches!(self.state, FightState::Punch | FightState::Kick |
                 FightState::CrouchPunch | FightState::CrouchKick |
                 FightState::JumpPunch | FightState::JumpKick)
    }
    fn is_crouching(&self) -> bool {
        matches!(self.state, FightState::CrouchIdle | FightState::CrouchWalkF |
                 FightState::CrouchWalkB | FightState::CrouchPunch | FightState::CrouchKick)
    }
    fn is_guarding(&self) -> bool { self.state == FightState::Guard }
    fn hitbox_cy(&self) -> f32 { if self.is_crouching() { self.y - 20.0 } else { self.y - 40.0 } }
    fn hitbox_h(&self)  -> f32 { if self.is_crouching() { 20.0 } else { 40.0 } }

    fn attack_box(&self) -> Option<(f32, f32, f32, f32)> { // x,y,w,h (top-left)
        let t = self.state_timer;
        if t < ATTACK_HIT_START || t > ATTACK_HIT_END { return None; }
        let (fwd, cy_off, w, h) = match self.state {
            FightState::Punch       => (55.0, -42.0, 30.0, 18.0),
            FightState::Kick        => (65.0, -22.0, 32.0, 20.0),
            FightState::CrouchPunch => (50.0, -18.0, 28.0, 16.0),
            FightState::CrouchKick  => (62.0, -14.0, 30.0, 18.0),
            FightState::JumpPunch   => (48.0, -50.0, 28.0, 18.0),
            FightState::JumpKick    => (60.0, -38.0, 30.0, 20.0),
            _ => return None,
        };
        let bx = if self.facing == 1 { self.x + fwd - w } else { self.x - fwd };
        Some((bx, self.y + cy_off, w, h))
    }

    fn attack_damage(&self) -> u8 {
        match self.state {
            FightState::Punch | FightState::CrouchPunch | FightState::JumpPunch => PUNCH_DMG,
            FightState::Kick  | FightState::CrouchKick  | FightState::JumpKick  => KICK_DMG,
            _ => 0,
        }
    }

    fn state_name(&self) -> &'static str {
        match self.state {
            FightState::Idle        => "idle",
            FightState::WalkF       => "walkf",
            FightState::WalkB       => "walkb",
            FightState::Punch       => "punch",
            FightState::Kick        => "kick",
            FightState::CrouchIdle  => "crouch",
            FightState::CrouchWalkF => "crouch_walkf",
            FightState::CrouchWalkB => "crouch_walkb",
            FightState::CrouchPunch => "crouch_punch",
            FightState::CrouchKick  => "crouch_kick",
            FightState::Jump        => "jump",
            FightState::JumpPunch   => "jump_punch",
            FightState::JumpKick    => "jump_kick",
            FightState::Guard       => "guard",
            FightState::Hurt        => "hurt",
            FightState::Dead        => "dead",
        }
    }

    fn step_physics(&mut self) {
        self.x += self.vx;
        self.y += self.vy;
        self.vy += GRAVITY;
        if self.y >= GROUND_Y {
            self.y = GROUND_Y; self.vy = 0.0; self.vx = 0.0;
            if matches!(self.state, FightState::Jump | FightState::JumpPunch | FightState::JumpKick) {
                self.state = FightState::Idle; self.state_timer = 0;
            }
        }
        if self.invincible > 0 { self.invincible -= 1; }
        match self.state {
            FightState::Hurt => {
                self.state_timer += 1;
                if self.state_timer >= HURT_TICKS {
                    self.state = if self.on_ground() { FightState::Idle } else { FightState::Jump };
                    self.state_timer = 0;
                }
            }
            FightState::Punch | FightState::Kick |
            FightState::CrouchPunch | FightState::CrouchKick |
            FightState::JumpPunch | FightState::JumpKick => {
                self.state_timer += 1;
                if self.state_timer >= ATTACK_TICKS {
                    self.state = if self.on_ground() { FightState::Idle } else { FightState::Jump };
                    self.state_timer = 0;
                }
            }
            _ => {}
        }
    }
}

fn rects_overlap(ax: f32, ay: f32, aw: f32, ah: f32, bx: f32, by: f32, bw: f32, bh: f32) -> bool {
    ax < bx+bw && ax+aw > bx && ay < by+bh && ay+ah > by
}

// ── Input ─────────────────────────────────────────────────────────────────────
#[derive(Clone, Default)]
struct Input { left: bool, right: bool, up: bool, down: bool, punch: bool, kick: bool, guard: bool }

// ── CPU AI ────────────────────────────────────────────────────────────────────
#[derive(Clone)]
enum CpuKind { WeakPuncher, Kicker, Jumper, Guarder, Boss }

#[derive(Clone)]
struct CpuAI {
    kind: CpuKind,
    wait: u32,
    guard: bool,
    guard_timer: u32,
    jump_atk: bool,
}

impl CpuAI {
    fn new(kind: CpuKind) -> Self {
        CpuAI { kind, wait: 0, guard: false, guard_timer: 0, jump_atk: false }
    }

    fn tick(&mut self, cpu: &mut Fighter, px: f32, py: f32, tick: u64) {
        if cpu.is_dead() || cpu.state == FightState::Hurt { return; }
        if cpu.is_attacking() { return; }

        let dx = px - cpu.x;
        let dist = dx.abs();
        cpu.facing = if dx >= 0.0 { 1 } else { -1 };

        if self.wait > 0 { self.wait -= 1; cpu.state = FightState::Idle; return; }

        match self.kind {
            CpuKind::WeakPuncher => {
                // Periodically retreat to avoid being predictable
                let retreat_phase = tick % 90 < 14;
                if dist > 88.0 {
                    cpu.state = FightState::WalkF;
                    cpu.x += cpu.facing as f32 * WALK_SPD * 0.7;
                } else if dist < 40.0 || (dist < 72.0 && retreat_phase) {
                    cpu.state = FightState::WalkB;
                    cpu.x -= cpu.facing as f32 * WALK_SPD * 0.55;
                } else if cpu.on_ground() && tick % 36 < 4 {
                    let atk = if tick % 4 == 0 { FightState::CrouchPunch } else { FightState::Punch };
                    cpu.state = atk; cpu.state_timer = 0;
                    self.wait = 14 + (tick % 8) as u32;
                } else if tick % 80 < 10 {
                    cpu.state = FightState::Guard;
                } else {
                    cpu.state = FightState::Idle;
                }
            }
            CpuKind::Kicker => {
                if dist > 100.0 {
                    cpu.state = FightState::WalkF;
                    cpu.x += cpu.facing as f32 * WALK_SPD;
                } else if dist < 52.0 {
                    cpu.state = FightState::WalkB;
                    cpu.x -= cpu.facing as f32 * WALK_SPD;
                } else if cpu.on_ground() && tick % 26 < 3 {
                    let atk = if tick % 3 != 0 { FightState::Kick } else { FightState::Punch };
                    cpu.state = atk; cpu.state_timer = 0;
                    self.wait = 10;
                } else if tick % 65 < 10 {
                    // Retreat briefly then counter
                    cpu.state = FightState::WalkB;
                    cpu.x -= cpu.facing as f32 * WALK_SPD * 0.8;
                } else if tick % 100 < 8 {
                    cpu.state = FightState::Guard;
                } else {
                    cpu.state = FightState::Idle;
                }
            }
            CpuKind::Jumper => {
                if !cpu.on_ground() {
                    // In air: attack when close enough
                    if self.jump_atk && cpu.y < GROUND_Y - 28.0 && dist < 120.0 {
                        cpu.state = FightState::JumpKick; cpu.state_timer = 0;
                        self.jump_atk = false;
                    }
                    return;
                }
                // Just landed — quick ground kick if close
                if !self.jump_atk && dist < 85.0 && tick % 10 < 3 {
                    cpu.state = FightState::Kick; cpu.state_timer = 0;
                    self.wait = 10; return;
                }
                if dist > 105.0 {
                    cpu.state = FightState::WalkF;
                    cpu.x += cpu.facing as f32 * WALK_SPD;
                } else if dist < 55.0 {
                    cpu.state = FightState::WalkB;
                    cpu.x -= cpu.facing as f32 * WALK_SPD * 0.8;
                } else if tick % 36 < 4 {
                    if dist > 130.0 { cpu.vx = cpu.facing as f32 * DIAG_JUMP_VX; }
                    cpu.vy = JUMP_VY;
                    cpu.state = FightState::Jump;
                    self.jump_atk = true;
                } else if tick % 18 < 2 {
                    cpu.state = FightState::Punch; cpu.state_timer = 0;
                } else {
                    cpu.state = FightState::Idle;
                }
            }
            CpuKind::Guarder => {
                if self.guard_timer > 0 {
                    self.guard_timer -= 1;
                    if self.guard_timer > 0 {
                        self.guard = true;
                        cpu.state = FightState::Guard;
                    } else {
                        self.guard = false;
                        // Counter-attack immediately after guard ends
                        if dist < 88.0 && cpu.on_ground() {
                            let atk = if tick % 2 == 0 { FightState::Kick } else { FightState::Punch };
                            cpu.state = atk; cpu.state_timer = 0;
                            self.wait = 20;
                        }
                    }
                    return;
                }
                if dist < 72.0 {
                    if tick % 28 < 3 {
                        let atk = if tick % 3 != 0 { FightState::Kick } else { FightState::Punch };
                        cpu.state = atk; cpu.state_timer = 0; self.wait = 20;
                    } else if tick % 48 < 20 {
                        self.guard_timer = 22; self.guard = true;
                        cpu.state = FightState::Guard;
                    } else {
                        cpu.state = FightState::Idle;
                    }
                } else if dist > 92.0 {
                    cpu.state = FightState::WalkF;
                    cpu.x += cpu.facing as f32 * WALK_SPD * 0.85;
                } else {
                    cpu.state = FightState::Idle;
                }
            }
            CpuKind::Boss => {
                let phase2 = cpu.hp <= cpu.max_hp / 2;
                let spd = if phase2 { WALK_SPD * 1.4 } else { WALK_SPD };
                let freq = if phase2 { 14u64 } else { 22 };

                if !cpu.on_ground() {
                    // Boss attacks in air when close
                    if cpu.y < GROUND_Y - 28.0 && dist < 110.0 {
                        let air_atk = if tick % 2 == 0 { FightState::JumpKick } else { FightState::JumpPunch };
                        cpu.state = air_atk; cpu.state_timer = 0;
                    }
                    return;
                }

                if dist > 92.0 {
                    cpu.state = FightState::WalkF;
                    cpu.x += cpu.facing as f32 * spd;
                } else if dist < 42.0 {
                    cpu.state = FightState::WalkB;
                    cpu.x -= cpu.facing as f32 * spd * 0.8;
                } else if cpu.on_ground() && tick % freq < 3 {
                    let atk = match tick % 7 {
                        0 => FightState::Punch,
                        1 => FightState::Kick,
                        2 => { cpu.vy = JUMP_VY; FightState::Jump }
                        3 => FightState::CrouchKick,
                        4 => FightState::CrouchPunch,
                        5 => { cpu.vy = JUMP_VY; cpu.vx = cpu.facing as f32 * DIAG_JUMP_VX; FightState::Jump }
                        _ => FightState::Kick,
                    };
                    cpu.state = atk; cpu.state_timer = 0;
                    if !matches!(cpu.state, FightState::Jump) {
                        self.wait = if phase2 { 8 } else { 14 };
                    }
                } else if tick % 55 < 8 && !phase2 {
                    cpu.state = FightState::Guard;
                } else {
                    cpu.state = FightState::Idle;
                }
            }
        }
        // Clamp
        cpu.x = cpu.x.clamp(30.0, STAGE_W - 30.0);
    }
}

// ── Stage config ──────────────────────────────────────────────────────────────
struct StageCfg { cpu_hp: u8, kind: CpuKind, name: &'static str, name_en: &'static str }
fn stage_cfg(s: u32) -> StageCfg {
    match s {
        1 => StageCfg { cpu_hp: 6,  kind: CpuKind::WeakPuncher, name: "ヨワヨワくん", name_en: "Weakling" },
        2 => StageCfg { cpu_hp: 8,  kind: CpuKind::Kicker,      name: "キッカー",     name_en: "Kicker" },
        3 => StageCfg { cpu_hp: 9,  kind: CpuKind::Jumper,      name: "ジャンパー",   name_en: "Jumper" },
        4 => StageCfg { cpu_hp: 10, kind: CpuKind::Guarder,     name: "ガードマン",   name_en: "Guardsman" },
        _ => StageCfg { cpu_hp: 14, kind: CpuKind::Boss,        name: "ボス",         name_en: "Boss" },
    }
}

// ── Game ──────────────────────────────────────────────────────────────────────
#[derive(PartialEq)]
enum Phase { Title, Playing, StageClear, GameOver }

struct FightGame {
    phase: Phase,
    stage: u32,
    tick: u64,
    score: u32,
    event: Option<&'static str>,
    player: Fighter,
    cpu: Fighter,
    cpu_ai: CpuAI,
    input: Input,
}

impl FightGame {
    fn new() -> Self {
        let cfg = stage_cfg(1);
        FightGame {
            phase: Phase::Title, stage: 1, tick: 0, score: 0, event: None,
            player: Fighter::new(100.0,  1, P_MAX_HP),
            cpu:    Fighter::new(420.0, -1, cfg.cpu_hp),
            cpu_ai: CpuAI::new(cfg.kind),
            input: Input::default(),
        }
    }

    fn start(&mut self, _name: &str) {
        self.score = 0;
        self.load_stage(1);
        self.phase = Phase::Playing;
    }

    fn load_stage(&mut self, stage: u32) {
        let cfg = stage_cfg(stage);
        self.player = Fighter::new(100.0,  1, P_MAX_HP);
        self.cpu    = Fighter::new(420.0, -1, cfg.cpu_hp);
        self.cpu_ai = CpuAI::new(cfg.kind);
        self.stage  = stage;
        self.tick   = 0;
        self.event  = Some("stage_start");
    }

    fn tick_game(&mut self) {
        if self.phase != Phase::Playing { return; }
        self.tick += 1;
        self.event = None;

        // CPU AI
        let px = self.player.x;
        let py = self.player.y;
        let tick = self.tick;
        self.cpu_ai.tick(&mut self.cpu, px, py, tick);

        // Player input → state
        self.apply_input();

        // Physics
        self.player.step_physics();
        self.cpu.step_physics();

        // Keep facing opponent (player only, when not busy)
        if !self.player.is_attacking() && self.player.state != FightState::Hurt && !self.player.is_dead() {
            let dx = self.cpu.x - self.player.x;
            self.player.facing = if dx >= 0.0 { 1 } else { -1 };
        }

        // Hit detection: player → cpu
        if let Some((ax, ay, aw, ah)) = self.player.attack_box() {
            let (bx, by, bw, bh) = (self.cpu.x - 20.0, self.cpu.hitbox_cy(), 40.0, self.cpu.hitbox_h());
            if self.cpu.invincible == 0 && rects_overlap(ax, ay, aw, ah, bx, by, bw, bh) {
                let guarded = matches!(self.cpu_ai.kind, CpuKind::Guarder) && self.cpu_ai.guard;
                if !guarded {
                    let dmg = self.player.attack_damage();
                    self.cpu.hp = self.cpu.hp.saturating_sub(dmg);
                    self.cpu.invincible = INVINCIBLE_TICKS;
                    self.score += 50;
                    if self.cpu.hp == 0 {
                        self.cpu.state = FightState::Dead;
                    } else {
                        self.cpu.state = FightState::Hurt; self.cpu.state_timer = 0;
                    }
                    self.event = Some("player_hit");
                    if matches!(self.cpu_ai.kind, CpuKind::Guarder) { self.cpu_ai.guard_timer = 20; }
                }
            }
        }

        // Hit detection: cpu → player
        if let Some((ax, ay, aw, ah)) = self.cpu.attack_box() {
            let (bx, by, bw, bh) = (self.player.x - 20.0, self.player.hitbox_cy(), 40.0, self.player.hitbox_h());
            if self.player.invincible == 0 && rects_overlap(ax, ay, aw, ah, bx, by, bw, bh) {
                // ガード中はダメージ0（ノックバックのみ）
                if self.player.is_guarding() {
                    self.player.invincible = INVINCIBLE_TICKS / 2;
                    self.event = Some("guard_success");
                    // ガードのけぞり（少し後退）
                    self.player.x -= self.player.facing as f32 * 6.0;
                    return;  // ← ここで終了
                }
                let dmg = self.cpu.attack_damage();
                self.player.hp = self.player.hp.saturating_sub(dmg);
                self.player.invincible = INVINCIBLE_TICKS;
                if self.player.hp == 0 {
                    self.player.state = FightState::Dead;
                    self.event = Some("player_dead");
                } else {
                    self.player.state = FightState::Hurt; self.player.state_timer = 0;
                    self.event = if self.player.hp <= self.player.max_hp / 3 {
                        Some("player_low_hp")
                    } else {
                        Some("cpu_hit")
                    };
                }
            }
        }

        // Clamp x
        self.player.x = self.player.x.clamp(30.0, STAGE_W - 30.0);
        self.cpu.x    = self.cpu.x.clamp(30.0, STAGE_W - 30.0);

        // Win / loss
        if self.cpu.is_dead() {
            let bonus = 500 * self.stage + self.player.hp as u32 * 100;
            self.score += bonus;
            if self.stage >= MAX_STAGE {
                self.event = Some("all_clear");
                self.phase = Phase::GameOver;
            } else {
                self.event = Some("stage_clear");
                self.phase = Phase::StageClear;
            }
        } else if self.player.is_dead() {
            self.event = Some("gameover");
            self.phase = Phase::GameOver;
        }
    }

    fn apply_input(&mut self) {
        let p = &mut self.player;
        if p.is_dead() || p.state == FightState::Hurt { return; }
        if p.is_attacking() { return; }

        let inp = self.input.clone();
        let on_ground = p.on_ground();

        // ── Guard (ground only, can't attack while guarding) ──
        if inp.guard && on_ground && !inp.punch && !inp.kick {
            p.state = FightState::Guard;
            return;
        }

        // ── Jump ──
        if inp.up && on_ground {
            p.vy = JUMP_VY;
            // 斜めジャンプ
            if inp.right      { p.vx =  DIAG_JUMP_VX; }
            else if inp.left  { p.vx = -DIAG_JUMP_VX; }
            p.state = FightState::Jump;
            return;
        }

        // ── In air ──
        if !on_ground {
            if inp.punch { p.state = FightState::JumpPunch; p.state_timer = 0; }
            else if inp.kick { p.state = FightState::JumpKick; p.state_timer = 0; }
            return;
        }

        // ── Crouch ──
        if inp.down {
            if inp.punch     { p.state = FightState::CrouchPunch; p.state_timer = 0; return; }
            if inp.kick      { p.state = FightState::CrouchKick;  p.state_timer = 0; return; }
            // しゃがみ歩き
            if inp.left  { p.x -= WALK_SPD * 0.6; p.state = if p.facing == -1 { FightState::CrouchWalkF } else { FightState::CrouchWalkB }; return; }
            if inp.right { p.x += WALK_SPD * 0.6; p.state = if p.facing ==  1 { FightState::CrouchWalkF } else { FightState::CrouchWalkB }; return; }
            p.state = FightState::CrouchIdle;
            return;
        }

        // ── Standing attacks ──
        if inp.punch { p.state = FightState::Punch; p.state_timer = 0; return; }
        if inp.kick  { p.state = FightState::Kick;  p.state_timer = 0; return; }

        // ── Walk ──
        if inp.left  { p.x -= WALK_SPD; p.state = if p.facing == -1 { FightState::WalkF } else { FightState::WalkB }; }
        else if inp.right { p.x += WALK_SPD; p.state = if p.facing == 1 { FightState::WalkF } else { FightState::WalkB }; }
        else { p.state = FightState::Idle; }
    }

    fn to_json(&self) -> String {
        let cfg = stage_cfg(self.stage);
        serde_json::json!({
            "type": "state",
            "phase": match self.phase {
                Phase::Title      => "title",
                Phase::Playing    => "playing",
                Phase::StageClear => "stage_clear",
                Phase::GameOver   => "gameover",
            },
            "stage": self.stage, "score": self.score, "tick": self.tick,
            "event": self.event,
            "cpu_name": cfg.name, "cpu_name_en": cfg.name_en,
            "player": {
                "x": self.player.x as i32, "y": self.player.y as i32,
                "hp": self.player.hp, "max_hp": self.player.max_hp,
                "state": self.player.state_name(), "facing": self.player.facing,
                "inv": self.player.invincible > 0,
            },
            "cpu": {
                "x": self.cpu.x as i32, "y": self.cpu.y as i32,
                "hp": self.cpu.hp, "max_hp": self.cpu.max_hp,
                "state": self.cpu.state_name(), "facing": self.cpu.facing,
                "inv": self.cpu.invincible > 0,
            },
        }).to_string()
    }
}

// ── WebSocket messages ────────────────────────────────────────────────────────
#[derive(Deserialize)]
struct StartProfile { name: String }

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientMsg {
    Start   { profile: StartProfile },
    Input   { left: bool, right: bool, up: bool, down: bool, punch: bool, kick: bool, guard: bool },
    Next,
    Submit  { name: String, score: u32 },
    Restart,
}

// ── run ───────────────────────────────────────────────────────────────────────
pub async fn run(
    mut socket: WebSocket,
    scores: Arc<Mutex<ScoreBoard>>,
    player_count: Arc<AtomicUsize>,
) {
    let _guard = PlayerCountGuard::new(player_count);
    let (tx, mut rx) = unbounded_channel::<String>();
    let game_arc = Arc::new(Mutex::new(FightGame::new()));

    // Send title scores
    {
        let sb = scores.lock().unwrap();
        let _ = tx.send(serde_json::json!({
            "type": "title", "scores": sb.list(), "min_score": sb.min_score()
        }).to_string());
    }

    // Tick loop
    let game2 = Arc::clone(&game_arc);
    let tx2   = tx.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(TICK_MS));
        loop {
            ticker.tick().await;
            let mut g = game2.lock().unwrap();
            if g.phase == Phase::Playing {
                g.tick_game();
                let _ = tx2.send(g.to_json());
            }
        }
    });

    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                if socket.send(Message::Text(msg)).await.is_err() { break; }
            }
            msg = socket.recv() => {
                let Some(Ok(message)) = msg else { break; };
                let txt = match message {
                    Message::Text(t)   => t,
                    Message::Ping(p)   => { let _ = socket.send(Message::Pong(p)).await; continue; }
                    Message::Pong(_) | Message::Binary(_) => continue,
                    Message::Close(_)  => break,
                };
                let Ok(cm) = serde_json::from_str::<ClientMsg>(&txt) else { continue; };
                match cm {
                    ClientMsg::Start { profile } => {
                        let name = profile.name.trim().to_string();
                        if name.is_empty() { continue; }
                        {
                            let mut g = game_arc.lock().unwrap();
                            g.start(&name);
                        }
                        let msg = game_arc.lock().unwrap().to_json();
                        let _ = tx.send(msg);
                    }
                    ClientMsg::Input { left, right, up, down, punch, kick, guard } => {
                        game_arc.lock().unwrap().input = Input { left, right, up, down, punch, kick, guard };
                    }
                    ClientMsg::Next => {
                        let msg = {
                            let mut g = game_arc.lock().unwrap();
                            if g.phase == Phase::StageClear {
                                let next = g.stage + 1;
                                g.load_stage(next);
                                g.phase = Phase::Playing;
                                Some(g.to_json())
                            } else { None }
                        };
                        if let Some(m) = msg { let _ = tx.send(m); }
                    }
                    ClientMsg::Submit { name, score } => {
                        let msg = {
                            let mut sb = scores.lock().unwrap();
                            let rank = sb.add(name, score);
                            serde_json::json!({"type":"submitted","rank":rank,"scores":sb.list(),"min_score":sb.min_score()}).to_string()
                        };
                        let _ = tx.send(msg);
                    }
                    ClientMsg::Restart => {
                        *game_arc.lock().unwrap() = FightGame::new();
                        let sb = scores.lock().unwrap();
                        let _ = tx.send(serde_json::json!({
                            "type": "title", "scores": sb.list(), "min_score": sb.min_score()
                        }).to_string());
                    }
                }
            }
        }
    }
}

// ── 2P FightRoom ──────────────────────────────────────────────────────────────
struct WaitingFighter {
    name:     String,
    input_rx: tokio::sync::mpsc::UnboundedReceiver<Input>, // game loop reads P1 inputs
    state_tx: UnboundedSender<String>,                     // game loop → P1 socket
    notify:   Arc<Notify>,
}

pub struct FightRoom {
    waiting: AsyncMutex<Option<WaitingFighter>>,
}
pub type SharedFightRoom = Arc<FightRoom>;

impl FightRoom {
    pub fn new() -> Self { FightRoom { waiting: AsyncMutex::new(None) } }
    pub async fn waiting_count(&self) -> usize {
        usize::from(self.waiting.lock().await.is_some())
    }
}

// ── 2P game loop (best-of-3) ──────────────────────────────────────────────────
async fn game_loop_2p(
    mut p1_rx: tokio::sync::mpsc::UnboundedReceiver<Input>,
    mut p2_rx: tokio::sync::mpsc::UnboundedReceiver<Input>,
    p1_tx: UnboundedSender<String>,
    p2_tx: UnboundedSender<String>,
    p1_name: String,
    p2_name: String,
) {
    let mut p1_wins: u8 = 0;
    let mut p2_wins: u8 = 0;

    'match_loop: loop {
        // Reset fighters for each round
        let mut f1 = Fighter::new(100.0,  1, P_MAX_HP);
        let mut f2 = Fighter::new(440.0, -1, P_MAX_HP);
        let mut i1 = Input::default();
        let mut i2 = Input::default();
        let mut tick: u64 = 0;
        let mut ticker = interval(Duration::from_millis(TICK_MS));
        let mut first = true;

        'round_loop: loop {
            ticker.tick().await;
            tick += 1;

            // Drain latest inputs
            while let Ok(inp) = p1_rx.try_recv() { i1 = inp; }
            while let Ok(inp) = p2_rx.try_recv() { i2 = inp; }

            let event = if first { first = false; Some("stage_start") } else { None };

            // Apply inputs
            let event = apply_input_2p(&mut f1, &i1, event);
            let event = apply_input_2p(&mut f2, &i2, event);

            // Face opponent
            if !f1.is_attacking() && f1.state != FightState::Hurt && !f1.is_dead() {
                f1.facing = if f2.x >= f1.x { 1 } else { -1 };
            }
            if !f2.is_attacking() && f2.state != FightState::Hurt && !f2.is_dead() {
                f2.facing = if f1.x >= f2.x { 1 } else { -1 };
            }

            f1.step_physics(); f1.x = f1.x.clamp(30.0, STAGE_W - 30.0);
            f2.step_physics(); f2.x = f2.x.clamp(30.0, STAGE_W - 30.0);

            // Hit detection
            let event = check_hit_2p(&mut f1, &mut f2, event);
            let event = check_hit_2p_rev(&mut f1, &mut f2, event);

            if f1.is_dead() || f2.is_dead() {
                let p1_won_round = f2.is_dead();
                if p1_won_round { p1_wins += 1; } else { p2_wins += 1; }

                let match_over = p1_wins >= 2 || p2_wins >= 2;

                let (phase1, phase2, ev1, ev2) = if match_over {
                    if p1_wins >= 2 { ("win", "lose", "all_clear", "gameover") }
                    else            { ("lose", "win", "gameover", "all_clear") }
                } else if p1_won_round {
                    ("round_win", "round_lose", "stage_clear", "gameover")
                } else {
                    ("round_lose", "round_win", "gameover", "stage_clear")
                };

                let j1 = make_2p_json(&f1, &f2, &p1_name, &p2_name, phase1, ev1, tick, p1_wins, p2_wins, "p1");
                let j2 = make_2p_json(&f2, &f1, &p2_name, &p1_name, phase2, ev2, tick, p2_wins, p1_wins, "p2");

                if p1_tx.send(j1).is_err() {
                    let _ = p2_tx.send(r#"{"type":"state","phase":"opponent_left"}"#.to_string());
                    return;
                }
                if p2_tx.send(j2).is_err() {
                    let _ = p1_tx.send(r#"{"type":"state","phase":"opponent_left"}"#.to_string());
                    return;
                }

                if match_over { return; }

                // 2.5 second pause between rounds
                tokio::time::sleep(Duration::from_millis(2500)).await;
                break 'round_loop;
            } else {
                let ev_str = event.unwrap_or("");
                let j1 = make_2p_json(&f1, &f2, &p1_name, &p2_name, "playing", ev_str, tick, p1_wins, p2_wins, "p1");
                let j2 = make_2p_json(&f2, &f1, &p2_name, &p1_name, "playing", ev_str, tick, p2_wins, p1_wins, "p2");

                if p1_tx.send(j1).is_err() {
                    let _ = p2_tx.send(r#"{"type":"state","phase":"opponent_left"}"#.to_string());
                    return;
                }
                if p2_tx.send(j2).is_err() {
                    let _ = p1_tx.send(r#"{"type":"state","phase":"opponent_left"}"#.to_string());
                    return;
                }
            }
        }
        // continues to next round
        let _ = p1_wins; // suppress lint; match_over already handles termination
    }
}

fn apply_input_2p<'a>(f: &mut Fighter, inp: &Input, ev: Option<&'a str>) -> Option<&'a str> {
    if f.is_dead() || f.state == FightState::Hurt { return ev; }
    if f.is_attacking() { return ev; }
    let on_ground = f.on_ground();
    if inp.guard && on_ground && !inp.punch && !inp.kick { f.state = FightState::Guard; return ev; }
    if inp.up && on_ground {
        f.vy = JUMP_VY;
        if inp.right      { f.vx =  DIAG_JUMP_VX; }
        else if inp.left  { f.vx = -DIAG_JUMP_VX; }
        f.state = FightState::Jump; return ev;
    }
    if !on_ground {
        if inp.punch { f.state = FightState::JumpPunch; f.state_timer = 0; }
        else if inp.kick { f.state = FightState::JumpKick; f.state_timer = 0; }
        return ev;
    }
    if inp.down {
        if inp.punch     { f.state = FightState::CrouchPunch; f.state_timer = 0; return ev; }
        if inp.kick      { f.state = FightState::CrouchKick;  f.state_timer = 0; return ev; }
        if inp.left  { f.x -= WALK_SPD * 0.6; f.state = if f.facing == -1 { FightState::CrouchWalkF } else { FightState::CrouchWalkB }; return ev; }
        if inp.right { f.x += WALK_SPD * 0.6; f.state = if f.facing ==  1 { FightState::CrouchWalkF } else { FightState::CrouchWalkB }; return ev; }
        f.state = FightState::CrouchIdle; return ev;
    }
    if inp.punch { f.state = FightState::Punch; f.state_timer = 0; return ev; }
    if inp.kick  { f.state = FightState::Kick;  f.state_timer = 0; return ev; }
    if inp.left  { f.x -= WALK_SPD; f.state = if f.facing == -1 { FightState::WalkF } else { FightState::WalkB }; }
    else if inp.right { f.x += WALK_SPD; f.state = if f.facing == 1 { FightState::WalkF } else { FightState::WalkB }; }
    else { f.state = FightState::Idle; }
    ev
}

fn check_hit_2p<'a>(attacker: &mut Fighter, defender: &mut Fighter, ev: Option<&'a str>) -> Option<&'a str> {
    let Some((ax,ay,aw,ah)) = attacker.attack_box() else { return ev; };
    let (bx,by,bw,bh) = (defender.x-20.0, defender.hitbox_cy(), 40.0, defender.hitbox_h());
    if defender.invincible > 0 || !rects_overlap(ax,ay,aw,ah,bx,by,bw,bh) { return ev; }
    if defender.is_guarding() {
        defender.invincible = INVINCIBLE_TICKS / 2;
        defender.x -= defender.facing as f32 * 6.0;
        return Some("guard_success");
    }
    let dmg = attacker.attack_damage();
    defender.hp = defender.hp.saturating_sub(dmg);
    defender.invincible = INVINCIBLE_TICKS;
    if defender.hp == 0 { defender.state = FightState::Dead; }
    else { defender.state = FightState::Hurt; defender.state_timer = 0; }
    Some("player_hit")
}

fn check_hit_2p_rev<'a>(f1: &mut Fighter, f2: &mut Fighter, ev: Option<&'a str>) -> Option<&'a str> {
    // f2 attacks f1
    let Some((ax,ay,aw,ah)) = f2.attack_box() else { return ev; };
    let (bx,by,bw,bh) = (f1.x-20.0, f1.hitbox_cy(), 40.0, f1.hitbox_h());
    if f1.invincible > 0 || !rects_overlap(ax,ay,aw,ah,bx,by,bw,bh) { return ev; }
    if f1.is_guarding() {
        f1.invincible = INVINCIBLE_TICKS / 2;
        f1.x -= f1.facing as f32 * 6.0;
        return Some("guard_success");
    }
    let dmg = f2.attack_damage();
    f1.hp = f1.hp.saturating_sub(dmg);
    f1.invincible = INVINCIBLE_TICKS;
    if f1.hp == 0 { f1.state = FightState::Dead; }
    else { f1.state = FightState::Hurt; f1.state_timer = 0; }
    Some("cpu_hit")
}

fn make_2p_json(me: &Fighter, opp: &Fighter, my_name: &str, opp_name: &str,
                phase: &str, event: &str, tick: u64, my_wins: u8, opp_wins: u8, role: &str) -> String {
    serde_json::json!({
        "type": "state",
        "phase": phase,
        "stage": 1,
        "score": 0,
        "tick": tick,
        "event": if event.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(event.to_string()) },
        "mode": "2p",
        "role": role,
        "my_wins": my_wins,
        "opp_wins": opp_wins,
        "cpu_name": opp_name,
        "cpu_name_en": opp_name,
        "player": {
            "x": me.x as i32, "y": me.y as i32,
            "hp": me.hp, "max_hp": me.max_hp,
            "state": me.state_name(), "facing": me.facing,
            "inv": me.invincible > 0,
        },
        "cpu": {
            "x": opp.x as i32, "y": opp.y as i32,
            "hp": opp.hp, "max_hp": opp.max_hp,
            "state": opp.state_name(), "facing": opp.facing,
            "inv": opp.invincible > 0,
        },
    }).to_string()
}

// ── run_2p ────────────────────────────────────────────────────────────────────
pub async fn run_2p(
    mut socket: WebSocket,
    scores: Arc<Mutex<ScoreBoard>>,
    player_count: Arc<AtomicUsize>,
    room: SharedFightRoom,
) {
    let _guard = PlayerCountGuard::new(player_count);
    let (state_tx, mut state_rx) = unbounded_channel::<String>();
    let (input_tx, input_rx)     = unbounded_channel::<Input>();
    let notify = Arc::new(Notify::new());

    // Send waiting message until we get the name
    let _ = state_tx.send(r#"{"type":"state","phase":"name_input"}"#.to_string());

    // Wait for Start message to get the player name
    let name = loop {
        match socket.recv().await {
            Some(Ok(Message::Text(txt))) => {
                if let Ok(ClientMsg::Start { profile }) = serde_json::from_str(&txt) {
                    let n = profile.name.trim().to_string();
                    if !n.is_empty() { break n; }
                }
            }
            _ => return,
        }
    };

    // Matchmaking
    let role = {
        let mut waiting = room.waiting.lock().await;
        if let Some(p1) = waiting.take() {
            let p1_name = p1.name.clone();
            let p2_name = name.clone();
            tokio::spawn(game_loop_2p(
                p1.input_rx, input_rx,
                p1.state_tx, state_tx.clone(),
                p1_name, p2_name,
            ));
            p1.notify.notify_one();
            "p2"
        } else {
            *waiting = Some(WaitingFighter {
                name: name.clone(),
                input_rx,
                state_tx: state_tx.clone(),
                notify: notify.clone(),
            });
            "p1"
        }
    };

    // P1 waits for P2
    if role == "p1" {
        let _ = state_tx.send(r#"{"type":"state","phase":"waiting"}"#.to_string());
        loop {
            tokio::select! {
                _ = notify.notified() => break,
                // state_rx も消費してソケットに送る（waiting メッセージを届ける）
                Some(msg) = state_rx.recv() => {
                    if socket.send(Message::Text(msg)).await.is_err() { return; }
                }
                msg = socket.recv() => {
                    if !matches!(msg, Some(Ok(_))) {
                        let mut w = room.waiting.lock().await;
                        *w = None;
                        return;
                    }
                }
            }
        }
    }

    // Send title scores
    {
        let sb = scores.lock().unwrap();
        let _ = state_tx.send(serde_json::json!({
            "type": "title", "scores": sb.list(), "min_score": sb.min_score()
        }).to_string());
    }

    // Main loop
    loop {
        tokio::select! {
            Some(msg) = state_rx.recv() => {
                if socket.send(Message::Text(msg)).await.is_err() { break; }
            }
            msg = socket.recv() => {
                let Some(Ok(message)) = msg else { break; };
                let txt = match message {
                    Message::Text(t)   => t,
                    Message::Ping(p)   => { let _ = socket.send(Message::Pong(p)).await; continue; }
                    Message::Pong(_) | Message::Binary(_) => continue,
                    Message::Close(_)  => break,
                };
                if let Ok(ClientMsg::Input { left, right, up, down, punch, kick, guard }) =
                    serde_json::from_str::<ClientMsg>(&txt)
                {
                    let _ = input_tx.send(Input { left, right, up, down, punch, kick, guard });
                }
            }
        }
    }
}
