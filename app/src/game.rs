use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering},
    Arc,
};
use tokio::sync::{mpsc, watch, Mutex, Notify};
use tokio::time::{interval, Duration, MissedTickBehavior};

const W: f64 = 800.0;
const H: f64 = 600.0;
const BALL_R: f64 = 8.0;
const PADDLE_W: f64 = 12.0;
const PADDLE_H: f64 = 80.0;
const PADDLE_SPEED: f64 = 12.0;
const BALL_SPEED: f64 = 5.0;
const BALL_SPEED_FAST: f64 = BALL_SPEED * 1.6;
const CPU_SPEED: f64 = 3.8;
const PLAYER_X: f64 = 20.0;
const CPU_X: f64 = W - PLAYER_X - PADDLE_W;
const SERVE_SLOW: f64 = BALL_SPEED * 0.6;
const SERVE_NORMAL: f64 = BALL_SPEED;
const SERVE_FAST: f64 = BALL_SPEED * 1.4;

const NO_TARGET: u64 = u64::MAX;

struct PlayerCountGuard(Arc<AtomicUsize>);
impl PlayerCountGuard {
    fn new(counter: Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::Relaxed);
        Self(counter)
    }
}
impl Drop for PlayerCountGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

fn pack_target(v: Option<f64>) -> u64 {
    v.map(|f| f.to_bits()).unwrap_or(NO_TARGET)
}
fn unpack_target(v: u64) -> Option<f64> {
    if v == NO_TARGET { None } else { Some(f64::from_bits(v)) }
}

// ── プロトコル ──────────────────────────────────────────

#[derive(Deserialize)]
struct Input {
    #[serde(default)] dir: i32,
    #[serde(default)] restart: bool,
    #[serde(default)] pause: bool,
    target_y: Option<f64>,
}

#[derive(Serialize)]
struct GameState {
    ball_x: f64, ball_y: f64,
    player_y: f64, cpu_y: f64,
    point_player: String, point_cpu: String,
    games_player: u32, games_cpu: u32,
    phase: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")] winner: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")] event: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")] role: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")] countdown: Option<u32>,
}

// ── 2P マッチメイキング状態 ───────────────────────────────

pub struct AppState {
    waiting: Mutex<Option<WaitingPlayer>>,
}

struct WaitingPlayer {
    p1_dir:      Arc<AtomicI32>,
    p1_restart:  Arc<AtomicBool>,
    p1_pause:    Arc<AtomicBool>,
    p1_target_y: Arc<AtomicU64>,
    state_tx:    mpsc::UnboundedSender<String>,
    notify:      Arc<Notify>,
}

impl AppState {
    pub fn new() -> Self { Self { waiting: Mutex::new(None) } }
    pub async fn waiting_count(&self) -> usize {
        let waiting = self.waiting.lock().await;
        usize::from(waiting.is_some())
    }
}

// ── ゲームロジック ─────────────────────────────────────

#[derive(PartialEq)]
enum Phase {
    Starting { ticks_left: u32 },
    Playing,
    Paused,
    GameOver { winner: &'static str },
}

struct Game {
    ball_x: f64, ball_y: f64, ball_vx: f64, ball_vy: f64,
    player_y: f64, cpu_y: f64,
    pt_player: i32, pt_cpu: i32,
    games_player: u32, games_cpu: u32,
    player_dir: i32, p2_dir: i32,
    player_target_y: Option<f64>,
    p2_target_y:     Option<f64>,
    event: Option<&'static str>,
    cpu_target_offset: f64, tick_count: u64,
    phase: Phase, two_player: bool,
}

fn pseudo_rand(seed: u64) -> f64 {
    let x = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let x = x ^ (x >> 33);
    let x = x.wrapping_mul(0xff51afd7ed558ccd);
    let x = x ^ (x >> 33);
    (x as f64) / (u64::MAX as f64)
}

fn zone_speed(rel: f64) -> f64 {
    if rel.abs() > 0.5 { BALL_SPEED_FAST } else { BALL_SPEED }
}

fn point_label(pt: i32, opp: i32) -> String {
    if pt == 3 && opp == 3 { return "Deuce".into(); }
    if pt == 4 { return "Adv".into(); }
    match pt { 0 => "Love".into(), 1 => "15".into(), 2 => "30".into(), _ => "40".into() }
}

fn take_bool(a: &AtomicBool) -> bool {
    a.compare_exchange(true, false, Ordering::AcqRel, Ordering::Relaxed).is_ok()
}

impl Game {
    fn new(two_player: bool) -> Self {
        let mut g = Self {
            ball_x: W/2.0, ball_y: H/2.0,
            ball_vx: BALL_SPEED, ball_vy: BALL_SPEED * 0.6,
            player_y: H/2.0, cpu_y: H/2.0,
            pt_player: 0, pt_cpu: 0,
            games_player: 0, games_cpu: 0,
            player_dir: 0, p2_dir: 0,
            player_target_y: None, p2_target_y: None,
            event: None, cpu_target_offset: 0.0, tick_count: 0,
            phase: Phase::Playing, two_player,
        };
        g.start_serve(1.0);
        g
    }

    fn start_serve(&mut self, dir: f64) {
        self.reset_rally(dir);
        self.phase = Phase::Starting { ticks_left: 90 };
    }

    fn refresh_cpu_offset(&mut self) {
        let r = pseudo_rand(self.tick_count ^ (self.pt_player as u64 * 7 + self.pt_cpu as u64 * 13));
        self.cpu_target_offset = (r - 0.5) * PADDLE_H * 0.9;
    }

    fn reset_rally(&mut self, dir: f64) {
        self.ball_x = W / 2.0;
        self.ball_y = H / 2.0;
        let r_speed = pseudo_rand(self.tick_count ^ 0xdeadbeef);
        let speed = if r_speed < 0.333 { SERVE_SLOW }
                    else if r_speed < 0.666 { SERVE_NORMAL }
                    else { SERVE_FAST };
        let r_sign  = pseudo_rand(self.tick_count ^ 0xcafebabe);
        let r_angle = pseudo_rand(self.tick_count ^ 0xf00dface);
        let vy_sign  = if r_sign < 0.5 { 1.0 } else { -1.0 };
        let vy_ratio = 0.3 + r_angle * 0.6;
        self.ball_vx = speed * dir;
        self.ball_vy = speed * vy_ratio * vy_sign;
        self.refresh_cpu_offset();
    }

    fn award_point(&mut self, player_scored: bool) -> bool {
        if player_scored {
            if self.pt_cpu == 4 { self.pt_cpu = 3; return false; }
            self.pt_player += 1;
            if self.pt_player >= 4 && self.pt_player - self.pt_cpu >= 2 {
                self.games_player += 1; return true;
            }
        } else {
            if self.pt_player == 4 { self.pt_player = 3; return false; }
            self.pt_cpu += 1;
            if self.pt_cpu >= 4 && self.pt_cpu - self.pt_player >= 2 {
                self.games_cpu += 1; return true;
            }
        }
        false
    }

    fn reset_points(&mut self) { self.pt_player = 0; self.pt_cpu = 0; }

    fn apply_input_atomics(
        &mut self,
        dir_a: &AtomicI32,
        restart_a: &AtomicBool,
        pause_a: &AtomicBool,
        target_y_a: &AtomicU64,
    ) {
        self.player_dir      = dir_a.load(Ordering::Relaxed);
        self.player_target_y = unpack_target(target_y_a.load(Ordering::Relaxed));

        if take_bool(restart_a) {
            match self.phase {
                Phase::GameOver { .. } | Phase::Paused => {
                    self.reset_points();
                    self.start_serve(1.0);
                }
                _ => {}
            }
        }
        if take_bool(pause_a) {
            self.phase = match self.phase {
                Phase::Playing | Phase::Starting { .. } => Phase::Paused,
                Phase::Paused  => Phase::Playing,
                Phase::GameOver { winner } => Phase::GameOver { winner },
            };
        }
    }

    fn move_paddles(&mut self) {
        // P1 (左パドル)
        if let Some(ty) = self.player_target_y {
            self.player_y = ty;
        } else {
            self.player_y += self.player_dir as f64 * PADDLE_SPEED;
        }
        self.player_y = self.player_y.clamp(PADDLE_H/2.0, H - PADDLE_H/2.0);

        // P2/CPU (右パドル)
        if self.two_player {
            if let Some(ty) = self.p2_target_y {
                self.cpu_y = ty;
            } else {
                self.cpu_y += self.p2_dir as f64 * PADDLE_SPEED;
            }
            self.cpu_y = self.cpu_y.clamp(PADDLE_H/2.0, H - PADDLE_H/2.0);
        } else {
            let diff = self.ball_y + self.cpu_target_offset - self.cpu_y;
            if diff.abs() > 5.0 { self.cpu_y += diff.signum() * CPU_SPEED; }
            self.cpu_y = self.cpu_y.clamp(PADDLE_H/2.0, H - PADDLE_H/2.0);
        }
    }

    fn tick(&mut self) {
        self.event = None;
        self.tick_count += 1;

        // Starting カウントダウン中: パドルは動かせるがボールは静止
        if let Phase::Starting { ref mut ticks_left } = self.phase {
            *ticks_left = ticks_left.saturating_sub(1);
            if *ticks_left == 0 { self.phase = Phase::Playing; }
            self.move_paddles();
            return;
        }

        if self.phase != Phase::Playing { return; }

        self.move_paddles();

        self.ball_x += self.ball_vx;
        self.ball_y += self.ball_vy;

        if self.ball_y - BALL_R <= 0.0 {
            self.ball_y = BALL_R; self.ball_vy = self.ball_vy.abs(); self.event = Some("hit");
        } else if self.ball_y + BALL_R >= H {
            self.ball_y = H - BALL_R; self.ball_vy = -self.ball_vy.abs(); self.event = Some("hit");
        }

        if self.ball_vx < 0.0
            && self.ball_x - BALL_R <= PLAYER_X + PADDLE_W
            && self.ball_x + BALL_R >= PLAYER_X
            && (self.ball_y - self.player_y).abs() <= PADDLE_H/2.0
        {
            self.ball_x = PLAYER_X + PADDLE_W + BALL_R;
            let rel = (self.ball_y - self.player_y) / (PADDLE_H/2.0);
            let speed = zone_speed(rel);
            self.ball_vx = speed; self.ball_vy = rel * speed;
            self.event = Some("hit"); self.refresh_cpu_offset();
        }

        if self.ball_vx > 0.0
            && self.ball_x + BALL_R >= CPU_X
            && self.ball_x - BALL_R <= CPU_X + PADDLE_W
            && (self.ball_y - self.cpu_y).abs() <= PADDLE_H/2.0
        {
            self.ball_x = CPU_X - BALL_R;
            let rel = (self.ball_y - self.cpu_y) / (PADDLE_H/2.0);
            let speed = zone_speed(rel);
            self.ball_vx = -speed; self.ball_vy = rel * speed;
            self.event = Some("hit");
        }

        if self.ball_x + BALL_R < 0.0 {
            self.event = Some("score");
            if self.award_point(false) {
                self.phase = Phase::GameOver {
                    winner: if self.two_player { "p2" } else { "cpu" },
                };
            } else { self.start_serve(-1.0); }
        } else if self.ball_x - BALL_R > W {
            self.event = Some("score");
            if self.award_point(true) {
                self.phase = Phase::GameOver {
                    winner: if self.two_player { "p1" } else { "player" },
                };
            } else { self.start_serve(1.0); }
        }
    }

    fn to_state(&self, role: Option<&'static str>) -> GameState {
        let (phase_str, winner, countdown) = match &self.phase {
            Phase::Starting { ticks_left } => ("starting", None, Some(*ticks_left)),
            Phase::Playing  => ("playing",   None, None),
            Phase::Paused   => ("paused",    None, None),
            Phase::GameOver { winner } => ("game_over", Some(*winner), None),
        };
        GameState {
            ball_x: self.ball_x, ball_y: self.ball_y,
            player_y: self.player_y, cpu_y: self.cpu_y,
            point_player: point_label(self.pt_player, self.pt_cpu),
            point_cpu:    point_label(self.pt_cpu, self.pt_player),
            games_player: self.games_player, games_cpu: self.games_cpu,
            phase: phase_str, winner, event: self.event, role, countdown,
        }
    }
}

// ── 1P: ゲームループとI/Oを分離 ────────────────────────
//
// ゲームループタスク: ソケットに依存しない。watch チャンネルに最新フレームを書く。
// I/O タスク: watch から最新フレームを送信 & ソケットから入力を受信。
//
// → socket.send() の遅延がゲームループを止めない。プチフリーズ解消。

pub async fn run_1p(mut socket: WebSocket, player_count: Arc<AtomicUsize>) {
    let _player_count_guard = PlayerCountGuard::new(player_count);
    let dir_a      = Arc::new(AtomicI32::new(0));
    let restart_a  = Arc::new(AtomicBool::new(false));
    let pause_a    = Arc::new(AtomicBool::new(false));
    let target_y_a = Arc::new(AtomicU64::new(NO_TARGET));

    // watch チャンネル: 常に最新フレームだけ保持 (送信遅れでもフレームが溜まらない)
    let (state_tx, mut state_rx) = watch::channel(String::new());

    // ゲームループタスク (I/O なし、純粋なロジック)
    {
        let d = dir_a.clone();
        let r = restart_a.clone();
        let p = pause_a.clone();
        let t = target_y_a.clone();
        tokio::spawn(async move {
            let mut game = Game::new(false);
            let mut ticker = interval(Duration::from_millis(16));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                game.apply_input_atomics(&d, &r, &p, &t);
                game.tick();
                let json = serde_json::to_string(&game.to_state(None)).unwrap();
                if state_tx.send(json).is_err() { break; }
            }
        });
    }

    // I/O ループ: 送信と受信を並行処理
    loop {
        tokio::select! {
            res = state_rx.changed() => {
                if res.is_err() { break; } // ゲームループ終了
                let json = state_rx.borrow().clone();
                if socket.send(Message::Text(json)).await.is_err() { break; }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(inp) = serde_json::from_str::<Input>(&text) {
                            dir_a.store(inp.dir, Ordering::Relaxed);
                            if inp.restart { restart_a.store(true, Ordering::Relaxed); }
                            if inp.pause   { pause_a.store(true, Ordering::Relaxed); }
                            target_y_a.store(
                                pack_target(inp.target_y.map(|y|
                                    y.clamp(PADDLE_H/2.0, H - PADDLE_H/2.0))),
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

// ── 2P マッチメイキング ────────────────────────────────

pub async fn handle_2p(socket: WebSocket, state: Arc<AppState>, player_count: Arc<AtomicUsize>) {
    let _player_count_guard = PlayerCountGuard::new(player_count);
    let my_dir      = Arc::new(AtomicI32::new(0));
    let my_restart  = Arc::new(AtomicBool::new(false));
    let my_pause    = Arc::new(AtomicBool::new(false));
    let my_target_y = Arc::new(AtomicU64::new(NO_TARGET));
    let (state_tx, state_rx) = mpsc::unbounded_channel::<String>();
    let notify = Arc::new(Notify::new());

    let role: &'static str = {
        let mut waiting = state.waiting.lock().await;
        if let Some(p1) = waiting.take() {
            tokio::spawn(game_loop_2p(
                p1.p1_dir,      my_dir.clone(),
                p1.p1_restart,  my_restart.clone(),
                p1.p1_pause,    my_pause.clone(),
                p1.p1_target_y, my_target_y.clone(),
                p1.state_tx,    state_tx.clone(),
            ));
            p1.notify.notify_one();
            "p2"
        } else {
            *waiting = Some(WaitingPlayer {
                p1_dir:      my_dir.clone(),
                p1_restart:  my_restart.clone(),
                p1_pause:    my_pause.clone(),
                p1_target_y: my_target_y.clone(),
                state_tx:    state_tx.clone(),
                notify:      notify.clone(),
            });
            "p1"
        }
    };

    socket_io(socket, role, my_dir, my_restart, my_pause, my_target_y, state_rx, notify, state).await;
}

async fn socket_io(
    mut socket: WebSocket,
    role: &'static str,
    dir_a:        Arc<AtomicI32>,
    restart_a:    Arc<AtomicBool>,
    pause_a:      Arc<AtomicBool>,
    target_y_a:   Arc<AtomicU64>,
    mut state_rx: mpsc::UnboundedReceiver<String>,
    notify:       Arc<Notify>,
    app_state:    Arc<AppState>,
) {
    if role == "p1" {
        let waiting_json = r#"{"phase":"waiting"}"#.to_string();
        if socket.send(Message::Text(waiting_json)).await.is_err() {
            let mut w = app_state.waiting.lock().await;
            *w = None;
            return;
        }
        loop {
            tokio::select! {
                _ = notify.notified() => break,
                msg = socket.recv() => {
                    match msg {
                        Some(Ok(_)) => {}
                        _ => {
                            let mut w = app_state.waiting.lock().await;
                            *w = None;
                            return;
                        }
                    }
                }
            }
        }
    }

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(inp) = serde_json::from_str::<Input>(&text) {
                            dir_a.store(inp.dir, Ordering::Relaxed);
                            if inp.restart { restart_a.store(true, Ordering::Relaxed); }
                            if inp.pause   { pause_a.store(true, Ordering::Relaxed); }
                            target_y_a.store(
                                pack_target(inp.target_y.map(|y|
                                    y.clamp(PADDLE_H/2.0, H - PADDLE_H/2.0))),
                                Ordering::Relaxed,
                            );
                        }
                    }
                    _ => break,
                }
            }
            json = state_rx.recv() => {
                match json {
                    Some(j) => { if socket.send(Message::Text(j)).await.is_err() { break; } }
                    None => break,
                }
            }
        }
    }
}

async fn game_loop_2p(
    p1_dir_a:      Arc<AtomicI32>,
    p2_dir_a:      Arc<AtomicI32>,
    p1_restart_a:  Arc<AtomicBool>,
    p2_restart_a:  Arc<AtomicBool>,
    p1_pause_a:    Arc<AtomicBool>,
    p2_pause_a:    Arc<AtomicBool>,
    p1_target_y_a: Arc<AtomicU64>,
    p2_target_y_a: Arc<AtomicU64>,
    p1_tx: mpsc::UnboundedSender<String>,
    p2_tx: mpsc::UnboundedSender<String>,
) {
    let mut game = Game::new(true);
    let mut ticker = interval(Duration::from_millis(16));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;

        game.apply_input_atomics(&p1_dir_a, &p1_restart_a, &p1_pause_a, &p1_target_y_a);
        game.p2_dir      = p2_dir_a.load(Ordering::Relaxed);
        game.p2_target_y = unpack_target(p2_target_y_a.load(Ordering::Relaxed));

        // P2 のリスタート・ポーズも処理
        if take_bool(&p2_restart_a) {
            match game.phase {
                Phase::GameOver { .. } | Phase::Paused => {
                    game.reset_points();
                    game.start_serve(1.0);
                }
                _ => {}
            }
        }
        if take_bool(&p2_pause_a) {
            game.phase = match game.phase {
                Phase::Playing | Phase::Starting { .. } => Phase::Paused,
                Phase::Paused  => Phase::Playing,
                Phase::GameOver { winner } => Phase::GameOver { winner },
            };
        }

        game.tick();

        let j1 = serde_json::to_string(&game.to_state(Some("p1"))).unwrap();
        let j2 = serde_json::to_string(&game.to_state(Some("p2"))).unwrap();

        if p1_tx.send(j1).is_err() {
            let _ = p2_tx.send(r#"{"phase":"opponent_left"}"#.to_string());
            break;
        }
        if p2_tx.send(j2).is_err() {
            let _ = p1_tx.send(r#"{"phase":"opponent_left"}"#.to_string());
            break;
        }
    }
}
