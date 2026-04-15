use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
    Arc,
};
use tokio::sync::watch;
use tokio::time::{interval, Duration, MissedTickBehavior};

const W: f64 = 800.0;
const H: f64 = 600.0;
const BALL_R: f64 = 8.0;
const PADDLE_W: f64 = 100.0;
const PADDLE_H: f64 = 12.0;
const PADDLE_Y: f64 = H - 40.0;
const PADDLE_SPEED: f64 = 8.0;
const BALL_SPEED: f64 = 5.5;
const BRICK_COLS: usize = 10;
const BRICK_ROWS: usize = 6;
const BRICK_W: f64 = 72.0;
const BRICK_H: f64 = 22.0;
const BRICK_GAP_X: f64 = 8.0;
const BRICK_GAP_Y: f64 = 8.0;
const BRICK_START_X: f64 = 4.0;
const BRICK_START_Y: f64 = 50.0;
const LIVES: u32 = 3;
const TOTAL_BRICKS: usize = BRICK_COLS * BRICK_ROWS;
const NO_TARGET: u64 = u64::MAX;

fn pack_f64(v: Option<f64>) -> u64 {
    v.map(|f| f.to_bits()).unwrap_or(NO_TARGET)
}
fn unpack_f64(v: u64) -> Option<f64> {
    if v == NO_TARGET { None } else { Some(f64::from_bits(v)) }
}

fn pseudo_rand(seed: u64) -> f64 {
    let x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let x = x ^ (x >> 33);
    let x = x.wrapping_mul(0xff51afd7ed558ccd);
    let x = x ^ (x >> 33);
    (x as f64) / (u64::MAX as f64)
}

fn brick_rect(idx: usize) -> (f64, f64) {
    let col = idx % BRICK_COLS;
    let row = idx / BRICK_COLS;
    let x = BRICK_START_X + col as f64 * (BRICK_W + BRICK_GAP_X);
    let y = BRICK_START_Y + row as f64 * (BRICK_H + BRICK_GAP_Y);
    (x, y)
}

// ── プロトコル ──────────────────────────────────────────

#[derive(Deserialize)]
struct Input {
    #[serde(default)] dir: i32,
    #[serde(default)] launch: bool,
    paddle_x: Option<f64>,
}

#[derive(Serialize)]
struct GameState {
    ball_x: f64,
    ball_y: f64,
    paddle_x: f64,
    bricks: Vec<u8>,
    score: u32,
    lives: u32,
    phase: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")] event: Option<&'static str>,
}

// ── ゲームロジック ──────────────────────────────────────

#[derive(PartialEq)]
enum Phase {
    Ready,
    Playing,
    GameOver,
    Clear,
}

struct Game {
    ball_x: f64,
    ball_y: f64,
    ball_vx: f64,
    ball_vy: f64,
    paddle_x: f64,
    bricks: Vec<bool>,
    score: u32,
    lives: u32,
    dir: i32,
    target_x: Option<f64>,
    event: Option<&'static str>,
    tick_count: u64,
    phase: Phase,
}

impl Game {
    fn new() -> Self {
        Self {
            ball_x: W / 2.0,
            ball_y: PADDLE_Y - PADDLE_H / 2.0 - BALL_R,
            ball_vx: 0.0,
            ball_vy: 0.0,
            paddle_x: W / 2.0,
            bricks: vec![true; TOTAL_BRICKS],
            score: 0,
            lives: LIVES,
            dir: 0,
            target_x: None,
            event: None,
            tick_count: 0,
            phase: Phase::Ready,
        }
    }

    fn launch(&mut self) {
        // ランダムな角度で上方向に発射（速度ベクトルの大きさは一定）
        let r = pseudo_rand(self.tick_count);
        let vx = (r - 0.5) * BALL_SPEED * 1.2;
        let vx = vx.clamp(-BALL_SPEED * 0.8, BALL_SPEED * 0.8);
        let vy = -((BALL_SPEED * BALL_SPEED - vx * vx).max(0.0)).sqrt();
        self.ball_vx = vx;
        self.ball_vy = vy;
        self.phase = Phase::Playing;
    }

    fn reset_ball(&mut self) {
        self.ball_x = self.paddle_x;
        self.ball_y = PADDLE_Y - PADDLE_H / 2.0 - BALL_R;
        self.ball_vx = 0.0;
        self.ball_vy = 0.0;
        self.phase = Phase::Ready;
    }

    fn tick(&mut self) {
        self.event = None;
        self.tick_count += 1;

        // パドル移動
        if let Some(tx) = self.target_x {
            self.paddle_x = tx.clamp(PADDLE_W / 2.0, W - PADDLE_W / 2.0);
        } else {
            self.paddle_x += self.dir as f64 * PADDLE_SPEED;
            self.paddle_x = self.paddle_x.clamp(PADDLE_W / 2.0, W - PADDLE_W / 2.0);
        }

        if self.phase == Phase::Ready {
            // ボールがパドルの上に乗り続ける
            self.ball_x = self.paddle_x;
            return;
        }

        if self.phase != Phase::Playing { return; }

        // ボール移動
        self.ball_x += self.ball_vx;
        self.ball_y += self.ball_vy;

        // 左右壁
        if self.ball_x - BALL_R <= 0.0 {
            self.ball_x = BALL_R;
            self.ball_vx = self.ball_vx.abs();
            self.event = Some("hit_wall");
        } else if self.ball_x + BALL_R >= W {
            self.ball_x = W - BALL_R;
            self.ball_vx = -self.ball_vx.abs();
            self.event = Some("hit_wall");
        }

        // 天井
        if self.ball_y - BALL_R <= 0.0 {
            self.ball_y = BALL_R;
            self.ball_vy = self.ball_vy.abs();
            self.event = Some("hit_wall");
        }

        // パドル衝突
        let pl = self.paddle_x - PADDLE_W / 2.0;
        let pr = self.paddle_x + PADDLE_W / 2.0;
        let pt = PADDLE_Y - PADDLE_H / 2.0;

        if self.ball_vy > 0.0
            && self.ball_y + BALL_R >= pt
            && self.ball_y - BALL_R <= pt + PADDLE_H
            && self.ball_x >= pl
            && self.ball_x <= pr
        {
            self.ball_y = pt - BALL_R;
            let rel = (self.ball_x - self.paddle_x) / (PADDLE_W / 2.0); // -1.0 〜 1.0
            // 端ほど水平寄りに、中央ほど真上寄りに
            let speed = BALL_SPEED + rel.abs() * 1.0;
            self.ball_vx = speed * rel * 0.85;
            self.ball_vy = -((speed * speed - self.ball_vx * self.ball_vx).max(1.0)).sqrt();
            self.event = Some("hit_paddle");
        }

        // ブロック衝突（1フレーム1ブロックまで）
        'brick_loop: for i in 0..TOTAL_BRICKS {
            if !self.bricks[i] { continue; }
            let (bx, by) = brick_rect(i);
            let bx2 = bx + BRICK_W;
            let by2 = by + BRICK_H;

            let closest_x = self.ball_x.clamp(bx, bx2);
            let closest_y = self.ball_y.clamp(by, by2);
            let dx = self.ball_x - closest_x;
            let dy = self.ball_y - closest_y;

            if dx * dx + dy * dy < BALL_R * BALL_R {
                self.bricks[i] = false;
                let row = i / BRICK_COLS;
                self.score += (BRICK_ROWS - row) as u32 * 10;

                // どちらの面に当たったか判定して反射
                let overlap_x = if self.ball_x < bx {
                    self.ball_x + BALL_R - bx
                } else if self.ball_x > bx2 {
                    bx2 + BALL_R - (self.ball_x - BALL_R)
                } else {
                    BALL_R * 2.0
                };
                let overlap_y = if self.ball_y < by {
                    self.ball_y + BALL_R - by
                } else if self.ball_y > by2 {
                    by2 + BALL_R - (self.ball_y - BALL_R)
                } else {
                    BALL_R * 2.0
                };

                if overlap_x < overlap_y {
                    self.ball_vx = -self.ball_vx;
                } else {
                    self.ball_vy = -self.ball_vy;
                }

                self.event = Some("hit_brick");
                break 'brick_loop;
            }
        }

        // ボールが画面下に落ちた
        if self.ball_y - BALL_R > H {
            self.event = Some("miss");
            self.lives = self.lives.saturating_sub(1);
            if self.lives == 0 {
                self.phase = Phase::GameOver;
            } else {
                self.reset_ball();
            }
        }

        // 全ブロック破壊でクリア
        if self.phase == Phase::Playing && self.bricks.iter().all(|&b| !b) {
            self.phase = Phase::Clear;
        }
    }

    fn to_state(&self) -> GameState {
        let phase_str = match self.phase {
            Phase::Ready    => "ready",
            Phase::Playing  => "playing",
            Phase::GameOver => "game_over",
            Phase::Clear    => "clear",
        };
        GameState {
            ball_x: self.ball_x,
            ball_y: self.ball_y,
            paddle_x: self.paddle_x,
            bricks: self.bricks.iter().map(|&b| b as u8).collect(),
            score: self.score,
            lives: self.lives,
            phase: phase_str,
            event: self.event,
        }
    }
}

// ── WebSocket ハンドラ ─────────────────────────────────

pub async fn run(mut socket: WebSocket) {
    let dir_a      = Arc::new(AtomicI32::new(0));
    let launch_a   = Arc::new(AtomicBool::new(false));
    let target_x_a = Arc::new(AtomicU64::new(NO_TARGET));

    let (state_tx, mut state_rx) = watch::channel(String::new());

    {
        let d = dir_a.clone();
        let l = launch_a.clone();
        let t = target_x_a.clone();
        tokio::spawn(async move {
            let mut game = Game::new();
            let mut ticker = interval(Duration::from_millis(16));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                game.dir = d.load(Ordering::Relaxed);
                game.target_x = unpack_f64(t.load(Ordering::Relaxed));

                if l.compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed).is_ok() {
                    match game.phase {
                        Phase::Ready => game.launch(),
                        Phase::GameOver | Phase::Clear => { game = Game::new(); }
                        _ => {}
                    }
                }

                game.tick();
                let json = serde_json::to_string(&game.to_state()).unwrap();
                if state_tx.send(json).is_err() { break; }
            }
        });
    }

    loop {
        tokio::select! {
            res = state_rx.changed() => {
                if res.is_err() { break; }
                let json = state_rx.borrow().clone();
                if socket.send(Message::Text(json)).await.is_err() { break; }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(inp) = serde_json::from_str::<Input>(&text) {
                            dir_a.store(inp.dir, Ordering::Relaxed);
                            if inp.launch { launch_a.store(true, Ordering::Relaxed); }
                            target_x_a.store(
                                pack_f64(inp.paddle_x.map(|x|
                                    x.clamp(PADDLE_W / 2.0, W - PADDLE_W / 2.0))),
                                Ordering::Relaxed,
                            );
                        }
                    }
                    _ => break,
                }
            }
        }
    }
}
