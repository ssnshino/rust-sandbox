use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::{interval, Duration};

use crate::dungeon_gen::{generate_for_floor, COLS, ROWS};
use crate::scores::ScoreBoard;

pub const MAX_PLAYERS: usize = 16;
const MAX_HP: u8 = 5;
const BASE_EXP_NEXT: u32 = 30;
const POISON_TICKS: u32 = 180;
const POISON_DAMAGE_INTERVAL: u32 = 90;
const HEAL_AMOUNT: u8 = 1;
const DIG_TICKS: u32 = 5;
const FILL_TICKS: u32 = 5;
const HOLE_LIFE: u32 = 60;
const ESCAPE_TICKS: u32 = 30;
const INVINCIBLE_TICKS: u32 = 20;
const BASE_SPEED: u32 = 4;
const ANGRY_SPEED: u32 = 2;
const ANGRY_DIST: i32 = 6;
const TICK_MS: u64 = 150;
const INITIAL_ALIENS: usize = 4;
const RESPAWN_TICKS: u32 = 40;

// ── Misc ──────────────────────────────────────────────────────────────────────
struct PlayerCountGuard(Arc<AtomicUsize>);
impl PlayerCountGuard {
    fn new(c: Arc<AtomicUsize>) -> Self { c.fetch_add(1, Ordering::Relaxed); Self(c) }
}
impl Drop for PlayerCountGuard {
    fn drop(&mut self) { self.0.fetch_sub(1, Ordering::Relaxed); }
}

fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_nanos() as u64
}

// ── Direction ─────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir { Up, Down, Left, Right }
impl Dir {
    fn delta(self) -> (i32, i32) {
        match self { Dir::Up=>(0,-1), Dir::Down=>(0,1), Dir::Left=>(-1,0), Dir::Right=>(1,0) }
    }
    fn name(self) -> &'static str {
        match self { Dir::Up=>"up", Dir::Down=>"down", Dir::Left=>"left", Dir::Right=>"right" }
    }
    fn parse(s: &str) -> Option<Self> {
        match s { "up"=>Some(Dir::Up),"down"=>Some(Dir::Down),"left"=>Some(Dir::Left),"right"=>Some(Dir::Right),_=>None }
    }
}

// ── Player (individual stats) ─────────────────────────────────────────────────
struct Player {
    x: usize, y: usize,
    hp: u8, max_hp: u8,
    score: u32, level: u32, exp: u32, exp_next: u32,
    poison_ticks: u32, poison_tick_timer: u32,
    dir: Dir, invincible: u32,
}
impl Player {
    fn new(x: usize, y: usize) -> Self {
        Player {
            x, y, hp: MAX_HP, max_hp: MAX_HP,
            score: 0, level: 1, exp: 0, exp_next: BASE_EXP_NEXT,
            poison_ticks: 0, poison_tick_timer: POISON_DAMAGE_INTERVAL,
            dir: Dir::Down, invincible: 0,
        }
    }
}

struct PlayerProfile { user_id: Option<String>, name: String }

struct Inventory { green: u32, red: u32, blue: u32 }
impl Inventory { fn new() -> Self { Inventory { green: 0, red: 0, blue: 0 } } }

#[derive(PartialEq, Eq)]
enum Phase { Playing, GameOver }

struct PlayerSlot {
    session_id: u64,
    player: Player,
    profile: PlayerProfile,
    inventory: Inventory,
    phase: Phase,
    event: Option<&'static str>,
    kills: u32,
    gameover_sent: bool,
    tx: UnboundedSender<String>,
}

// ── Shared floor objects ──────────────────────────────────────────────────────
#[derive(Clone, PartialEq)]
enum HoleState {
    Digging(u32),
    Open(u32),
    Trapped { alien_id: usize, escape: u32 },
    Filling { ticks: u32, alien_id: Option<usize>, filler_id: Option<u64> },
}
struct Hole { x: usize, y: usize, state: HoleState, id: usize, digger_id: Option<u64> }

#[derive(Clone, Copy, PartialEq, Eq)]
enum ItemKind { Green, Red, Blue }
impl ItemKind {
    fn as_str(self) -> &'static str {
        match self { Self::Green=>"green", Self::Red=>"red", Self::Blue=>"blue" }
    }
}
struct Item  { x: usize, y: usize, kind: ItemKind, id: usize }
struct Chest { x: usize, y: usize, id: usize }

#[derive(Clone, Copy, PartialEq, Eq)]
enum AlienKind { Damage, Poison, Heal }
impl AlienKind {
    fn as_str(self) -> &'static str {
        match self { Self::Damage=>"damage", Self::Poison=>"poison", Self::Heal=>"heal" }
    }
}
#[derive(Clone, PartialEq)]
enum AlienState { Active, Trapped(usize), Dead(u32) }
struct Alien { x: usize, y: usize, kind: AlienKind, state: AlienState, move_timer: u32, id: usize }

// ── Floor instance ────────────────────────────────────────────────────────────
struct FloorInstance {
    floor_num: u32,
    grid: Vec<Vec<u8>>,
    aliens: Vec<Alien>,
    holes: Vec<Hole>,
    items: Vec<Item>,
    chests: Vec<Chest>,
    tick: u64,
    goal: (usize, usize),
    start: (usize, usize),
    next_hole_id: usize,
    next_item_id: usize,
    next_chest_id: usize,
    players: Vec<PlayerSlot>,
    scores: Arc<Mutex<ScoreBoard>>,
}

impl FloorInstance {
    fn new(floor_num: u32, seed: u64, scores: Arc<Mutex<ScoreBoard>>) -> Self {
        let grid  = generate_for_floor(seed, floor_num);
        let start = pick_start(&grid, seed);
        let goal  = bfs_farthest(&grid, start.0, start.1);
        let aliens = make_aliens(&grid, floor_num, start);
        let chests = make_chests(&grid, floor_num, start, goal);
        FloorInstance {
            floor_num, grid, aliens, holes: Vec::new(), items: Vec::new(), chests,
            tick: 0, goal, start, next_hole_id: 0, next_item_id: 0, next_chest_id: 1000,
            players: Vec::new(), scores,
        }
    }

    fn add_player(&mut self, session_id: u64, mut player: Player,
                  profile: PlayerProfile, inventory: Inventory,
                  kills: u32, tx: UnboundedSender<String>) {
        player.x = self.start.0;
        player.y = self.start.1;
        player.dir = Dir::Down;
        self.players.push(PlayerSlot {
            session_id, player, profile, inventory,
            phase: Phase::Playing, event: None, kills,
            gameover_sent: false, tx,
        });
    }

    fn remove_player(&mut self, session_id: u64) -> Option<PlayerSlot> {
        self.players.iter().position(|p| p.session_id == session_id)
            .map(|i| self.players.remove(i))
    }

    fn player_idx(&self, session_id: u64) -> Option<usize> {
        self.players.iter().position(|p| p.session_id == session_id)
    }

    fn is_path(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < COLS && (y as usize) < ROWS
            && self.grid[y as usize][x as usize] == 1
    }

    fn hole_at(&self, x: usize, y: usize) -> Option<usize> {
        self.holes.iter().position(|h| h.x == x && h.y == y)
    }

    fn bfs_next(&self, fx: usize, fy: usize, tx: usize, ty: usize) -> Option<(usize, usize)> {
        if fx == tx && fy == ty { return None; }
        let mut vis = vec![vec![false; COLS]; ROWS];
        let mut q: VecDeque<(usize, usize, usize, usize)> = VecDeque::new();
        vis[fy][fx] = true;
        for (dx, dy) in [(0i32,-1),(0,1),(-1,0),(1,0)] {
            let nx = fx as i32 + dx; let ny = fy as i32 + dy;
            if self.is_path(nx, ny) {
                let (nx, ny) = (nx as usize, ny as usize);
                let blocked = self.holes.iter().any(|h| h.x==nx && h.y==ny && matches!(h.state, HoleState::Trapped{..}));
                if !blocked && !vis[ny][nx] { vis[ny][nx]=true; q.push_back((nx,ny,nx,ny)); }
            }
        }
        while let Some((cx, cy, fx2, fy2)) = q.pop_front() {
            if cx == tx && cy == ty { return Some((fx2, fy2)); }
            for (dx, dy) in [(0i32,-1),(0,1),(-1,0),(1,0)] {
                let nx = cx as i32 + dx; let ny = cy as i32 + dy;
                if self.is_path(nx, ny) {
                    let (nx, ny) = (nx as usize, ny as usize);
                    let blocked = self.holes.iter().any(|h| h.x==nx && h.y==ny && matches!(h.state, HoleState::Trapped{..}));
                    if !blocked && !vis[ny][nx] { vis[ny][nx]=true; q.push_back((nx,ny,fx2,fy2)); }
                }
            }
        }
        None
    }

    fn nearest_player_to(&self, ax: usize, ay: usize) -> Option<(usize, usize)> {
        self.players.iter()
            .filter(|s| s.phase == Phase::Playing)
            .min_by_key(|s| {
                let dx = s.player.x as i32 - ax as i32;
                let dy = s.player.y as i32 - ay as i32;
                dx.abs() + dy.abs()
            })
            .map(|s| (s.player.x, s.player.y))
    }

    // ── Player actions ────────────────────────────────────────────────────────

    /// Returns Some(next_floor_num) when player reaches the goal.
    fn apply_move(&mut self, session_id: u64, dir_str: &str) -> Option<u32> {
        let pi = self.player_idx(session_id)?;
        if self.players[pi].phase != Phase::Playing { return None; }
        let dir = Dir::parse(dir_str)?;
        self.players[pi].player.dir = dir;
        let (dx, dy) = dir.delta();
        let nx = self.players[pi].player.x as i32 + dx;
        let ny = self.players[pi].player.y as i32 + dy;
        if !self.is_path(nx, ny) { return None; }
        let (nx, ny) = (nx as usize, ny as usize);
        if self.hole_at(nx, ny).is_some() { return None; }
        self.players[pi].player.x = nx;
        self.players[pi].player.y = ny;

        // Chest pickup
        if let Some(ci) = self.chests.iter().position(|ch| ch.x == nx && ch.y == ny) {
            let chest = self.chests.remove(ci);
            self.open_chest(pi, chest.id, chest.x, chest.y);
        }
        // Item pickup
        if let Some(ii) = self.items.iter().position(|it| it.x == nx && it.y == ny) {
            let kind = self.items[ii].kind;
            self.items.remove(ii);
            self.add_item_to_inventory(pi, kind);
        }

        // Goal check
        if (self.players[pi].player.x, self.players[pi].player.y) == self.goal {
            self.players[pi].event = Some("floor_complete");
            return Some(self.floor_num + 1);
        }
        None
    }

    fn apply_act(&mut self, session_id: u64) {
        let Some(pi) = self.player_idx(session_id) else { return; };
        if self.players[pi].phase != Phase::Playing { return; }
        let (dx, dy) = self.players[pi].player.dir.delta();
        let tx = self.players[pi].player.x as i32 + dx;
        let ty = self.players[pi].player.y as i32 + dy;
        if !self.is_path(tx, ty) { return; }
        let (tx, ty) = (tx as usize, ty as usize);
        if let Some(hi) = self.hole_at(tx, ty) {
            match self.holes[hi].state.clone() {
                HoleState::Open(_) => {
                    self.holes[hi].state = HoleState::Filling { ticks: FILL_TICKS, alien_id: None, filler_id: Some(session_id) };
                }
                HoleState::Trapped { alien_id, .. } => {
                    self.holes[hi].state = HoleState::Filling { ticks: FILL_TICKS, alien_id: Some(alien_id), filler_id: Some(session_id) };
                }
                _ => {}
            }
        } else {
            let id = self.next_hole_id; self.next_hole_id += 1;
            self.holes.push(Hole { x: tx, y: ty, state: HoleState::Digging(DIG_TICKS), id, digger_id: Some(session_id) });
        }
    }

    fn apply_use_item(&mut self, session_id: u64, kind: &str) {
        let Some(pi) = self.player_idx(session_id) else { return; };
        if self.players[pi].phase != Phase::Playing { return; }
        match kind {
            "green" => {
                if self.players[pi].inventory.green == 0 { return; }
                self.players[pi].inventory.green -= 1;
                self.players[pi].player.hp = self.players[pi].player.hp.saturating_add(HEAL_AMOUNT).min(self.players[pi].player.max_hp);
                if self.players[pi].event != Some("levelup") { self.players[pi].event = Some("use_green"); }
            }
            "red" => {
                if self.players[pi].inventory.red == 0 { return; }
                self.players[pi].inventory.red -= 1;
                self.players[pi].player.poison_ticks = 0;
                self.players[pi].player.poison_tick_timer = POISON_DAMAGE_INTERVAL;
                if self.players[pi].event != Some("levelup") { self.players[pi].event = Some("use_red"); }
            }
            "blue" => {
                if self.players[pi].inventory.blue == 0 { return; }
                self.players[pi].inventory.blue -= 1;
                self.players[pi].player.hp = self.players[pi].player.max_hp;
                self.players[pi].player.poison_ticks = 0;
                self.players[pi].player.poison_tick_timer = POISON_DAMAGE_INTERVAL;
                if self.players[pi].event != Some("levelup") { self.players[pi].event = Some("use_blue"); }
            }
            _ => {}
        }
    }

    // ── Item helpers ──────────────────────────────────────────────────────────
    fn roll_potion_kind(&self, salt: usize) -> ItemKind {
        let roll = (self.tick as usize + salt * 17 + self.floor_num as usize * 13) % 100;
        if roll < 10 { ItemKind::Blue } else if roll < 34 { ItemKind::Red } else { ItemKind::Green }
    }

    fn maybe_spawn_drop(&mut self, x: usize, y: usize, alien_id: usize) {
        let roll = (self.tick as usize + alien_id * 17 + x * 11 + y * 7 + self.floor_num as usize * 13) % 100;
        if roll >= 38 { return; }
        let kind = self.roll_potion_kind(alien_id + x + y);
        let id = self.next_item_id; self.next_item_id += 1;
        self.items.push(Item { x, y, kind, id });
    }

    fn open_chest(&mut self, pi: usize, chest_id: usize, x: usize, y: usize) {
        let kind = self.roll_potion_kind(chest_id + x + y);
        self.add_item_to_inventory(pi, kind);
        if self.players[pi].event != Some("levelup") {
            self.players[pi].event = Some(match kind {
                ItemKind::Green => "chest_green",
                ItemKind::Red   => "chest_red",
                ItemKind::Blue  => "chest_blue",
            });
        }
    }

    fn add_item_to_inventory(&mut self, pi: usize, kind: ItemKind) {
        match kind {
            ItemKind::Green => { self.players[pi].inventory.green += 1; if self.players[pi].event != Some("levelup") { self.players[pi].event = Some("item_green"); } }
            ItemKind::Red   => { self.players[pi].inventory.red   += 1; if self.players[pi].event != Some("levelup") { self.players[pi].event = Some("item_red"); } }
            ItemKind::Blue  => { self.players[pi].inventory.blue  += 1; if self.players[pi].event != Some("levelup") { self.players[pi].event = Some("item_blue"); } }
        }
    }

    fn gain_exp(&mut self, pi: usize, amount: u32) {
        self.players[pi].player.exp += amount;
        while self.players[pi].player.exp >= self.players[pi].player.exp_next {
            self.players[pi].player.exp -= self.players[pi].player.exp_next;
            self.players[pi].player.level += 1;
            self.players[pi].player.max_hp = self.players[pi].player.max_hp.saturating_add(1).min(9);
            self.players[pi].player.hp = self.players[pi].player.max_hp;
            self.players[pi].player.exp_next += 15;
            self.players[pi].event = Some("levelup");
        }
    }

    fn apply_contact_effect_to(&mut self, pi: usize, kind: AlienKind) {
        match kind {
            AlienKind::Damage => {
                self.players[pi].player.hp = self.players[pi].player.hp.saturating_sub(1);
                self.players[pi].player.invincible = INVINCIBLE_TICKS;
                self.players[pi].event = Some("dmg");
            }
            AlienKind::Poison => {
                self.players[pi].player.poison_ticks = POISON_TICKS;
                self.players[pi].player.poison_tick_timer = POISON_DAMAGE_INTERVAL;
                self.players[pi].player.invincible = INVINCIBLE_TICKS;
                self.players[pi].event = Some("poison");
            }
            AlienKind::Heal => {
                self.players[pi].player.hp = self.players[pi].player.hp.saturating_add(HEAL_AMOUNT).min(self.players[pi].player.max_hp);
                self.players[pi].player.invincible = INVINCIBLE_TICKS / 2;
                if self.players[pi].event != Some("levelup") { self.players[pi].event = Some("heal"); }
            }
        }
        if self.players[pi].player.hp == 0 {
            self.players[pi].phase = Phase::GameOver;
        }
    }

    // ── Tick ─────────────────────────────────────────────────────────────────
    fn tick_all(&mut self) {
        if self.players.is_empty() { return; }
        self.tick += 1;

        // Per-player: event reset, invincible, poison
        for slot in &mut self.players {
            if slot.phase != Phase::Playing { continue; }
            slot.event = None;
            if slot.player.invincible > 0 { slot.player.invincible -= 1; }
            if slot.player.poison_ticks > 0 {
                slot.player.poison_ticks -= 1;
                if slot.player.poison_tick_timer > 0 { slot.player.poison_tick_timer -= 1; }
                if slot.player.poison_tick_timer == 0 {
                    slot.player.poison_tick_timer = POISON_DAMAGE_INTERVAL;
                    slot.player.hp = slot.player.hp.saturating_sub(1);
                    slot.event = Some("poison_tick");
                    if slot.player.hp == 0 { slot.phase = Phase::GameOver; }
                }
            }
        }

        // Holes
        let mut i = 0;
        while i < self.holes.len() {
            let remove = match self.holes[i].state.clone() {
                HoleState::Digging(t) => {
                    if t <= 1 { self.holes[i].state = HoleState::Open(HOLE_LIFE); }
                    else { self.holes[i].state = HoleState::Digging(t - 1); }
                    false
                }
                HoleState::Open(t) => {
                    if t <= 1 { true } else { self.holes[i].state = HoleState::Open(t - 1); false }
                }
                HoleState::Trapped { alien_id, escape } => {
                    if escape <= 1 {
                        let (hx, hy) = (self.holes[i].x, self.holes[i].y);
                        let ep = [(0i32,-1),(0,1),(-1,0),(1,0)].iter()
                            .filter_map(|&(dx, dy)| {
                                let nx = hx as i32 + dx; let ny = hy as i32 + dy;
                                if self.is_path(nx, ny) { Some((nx as usize, ny as usize)) } else { None }
                            }).next();
                        if let Some(a) = self.aliens.iter_mut().find(|a| a.id == alien_id) {
                            a.state = AlienState::Active;
                            if let Some((nx, ny)) = ep { a.x = nx; a.y = ny; }
                        }
                        true
                    } else {
                        self.holes[i].state = HoleState::Trapped { alien_id, escape: escape - 1 };
                        false
                    }
                }
                HoleState::Filling { ticks, alien_id: aid, filler_id } => {
                    if ticks <= 1 {
                        if let Some(alien_id) = aid {
                            let mut drop_pos = None;
                            if let Some(a) = self.aliens.iter_mut().find(|a| a.id == alien_id) {
                                a.state = AlienState::Dead(RESPAWN_TICKS);
                                drop_pos = Some((a.x, a.y, a.id));
                            }
                            if let Some((ax, ay, aid2)) = drop_pos {
                                // Kill credit to filler
                                if let Some(fid) = filler_id {
                                    if let Some(pi) = self.players.iter().position(|p| p.session_id == fid) {
                                        self.players[pi].kills += 1;
                                        let kills = self.players[pi].kills;
                                        self.players[pi].player.score += 10 + kills * 5;
                                        self.gain_exp(pi, 8);
                                        if self.players[pi].event != Some("levelup") {
                                            self.players[pi].event = Some("kill");
                                        }
                                    }
                                }
                                self.maybe_spawn_drop(ax, ay, aid2);
                            }
                        }
                        true
                    } else {
                        self.holes[i].state = HoleState::Filling { ticks: ticks - 1, alien_id: aid, filler_id };
                        false
                    }
                }
            };
            if remove { self.holes.remove(i); } else { i += 1; }
        }

        // Aliens: BFS to nearest player
        let count = self.aliens.len();
        let pending: Vec<Option<(usize, usize)>> = (0..count).map(|ai| {
            let (ax, ay) = (self.aliens[ai].x, self.aliens[ai].y);
            if self.aliens[ai].state == AlienState::Active && self.aliens[ai].move_timer == 0 {
                if let Some((tx, ty)) = self.nearest_player_to(ax, ay) {
                    self.bfs_next(ax, ay, tx, ty)
                } else { None }
            } else { None }
        }).collect();

        for ai in 0..count {
            match self.aliens[ai].state.clone() {
                AlienState::Dead(t) => {
                    if t <= 1 {
                        // Respawn far from any living player
                        let any_left = self.players.iter().any(|s| s.phase == Phase::Playing && s.player.x < COLS/2);
                        let rx = if any_left { COLS - 2 } else { 1 };
                        let any_top = self.players.iter().any(|s| s.phase == Phase::Playing && s.player.y < ROWS/2);
                        let ry = if any_top { ROWS - 2 } else { 1 };
                        self.aliens[ai].state = AlienState::Active;
                        self.aliens[ai].x = rx; self.aliens[ai].y = ry;
                        self.aliens[ai].move_timer = BASE_SPEED;
                    } else { self.aliens[ai].state = AlienState::Dead(t - 1); }
                }
                AlienState::Trapped(_) => {}
                AlienState::Active => {
                    if self.aliens[ai].move_timer > 0 {
                        self.aliens[ai].move_timer -= 1;
                    } else {
                        let nearest_dist = self.players.iter()
                            .filter(|s| s.phase == Phase::Playing)
                            .map(|s| {
                                let dx = self.aliens[ai].x as i32 - s.player.x as i32;
                                let dy = self.aliens[ai].y as i32 - s.player.y as i32;
                                dx.abs() + dy.abs()
                            })
                            .min().unwrap_or(999);
                        let (base_spd, angry_spd) = match self.floor_num {
                            1 => (10u32, 7u32),
                            2 => (6u32, 4u32),
                            _ => (BASE_SPEED, ANGRY_SPEED),
                        };
                        let speed = if nearest_dist <= ANGRY_DIST { angry_spd } else { base_spd };
                        self.aliens[ai].move_timer = speed;
                        if let Some((nx, ny)) = pending[ai] {
                            let hi = self.holes.iter().position(|h| h.x == nx && h.y == ny && matches!(h.state, HoleState::Open(_)));
                            if let Some(hi) = hi {
                                let aid = self.aliens[ai].id;
                                let hid = self.holes[hi].id;
                                self.aliens[ai].state = AlienState::Trapped(hid);
                                self.aliens[ai].x = nx; self.aliens[ai].y = ny;
                                self.holes[hi].state = HoleState::Trapped { alien_id: aid, escape: ESCAPE_TICKS };
                                // Notify the hole digger
                                let digger = self.holes[hi].digger_id;
                                if let Some(did) = digger {
                                    if let Some(pi) = self.players.iter().position(|p| p.session_id == did) {
                                        if self.players[pi].phase == Phase::Playing && self.players[pi].event.is_none() {
                                            self.players[pi].event = Some("trapped");
                                        }
                                    }
                                }
                            } else {
                                self.aliens[ai].x = nx; self.aliens[ai].y = ny;
                            }
                        }
                    }
                    // Collision with each player
                    if matches!(self.aliens[ai].state, AlienState::Active) {
                        let axi = self.aliens[ai].x;
                        let ayi = self.aliens[ai].y;
                        let kind = self.aliens[ai].kind;
                        let contacts: Vec<usize> = self.players.iter().enumerate()
                            .filter(|(_, s)| s.phase == Phase::Playing && s.player.invincible == 0
                                && s.player.x == axi && s.player.y == ayi)
                            .map(|(i, _)| i)
                            .collect();
                        let mut kill_heal = false;
                        for pi in contacts {
                            self.apply_contact_effect_to(pi, kind);
                            if kind == AlienKind::Heal { kill_heal = true; }
                        }
                        if kill_heal { self.aliens[ai].state = AlienState::Dead(RESPAWN_TICKS * 2); }
                    }
                }
            }
        }

        // Send state to all players
        let n = self.players.len();
        for i in 0..n {
            let msg = if self.players[i].phase == Phase::GameOver && !self.players[i].gameover_sent {
                self.players[i].gameover_sent = true;
                let score = self.players[i].player.score;
                let event = self.players[i].event;
                let sb = self.scores.lock().unwrap();
                serde_json::json!({
                    "type": "gameover",
                    "score": score,
                    "event": event,
                    "qualifies": sb.qualifies(score),
                    "scores": sb.list(),
                    "min_score": sb.min_score(),
                }).to_string()
            } else if self.players[i].phase == Phase::Playing {
                self.to_json_for(i)
            } else {
                continue;
            };
            let _ = self.players[i].tx.send(msg);
        }
    }

    // ── JSON ──────────────────────────────────────────────────────────────────
    fn to_json_for(&self, pi: usize) -> String {
        let slot = &self.players[pi];
        let p    = &slot.player;
        let aliens: Vec<serde_json::Value> = self.aliens.iter()
            .filter(|a| !matches!(a.state, AlienState::Dead(_)))
            .map(|a| serde_json::json!({"x":a.x,"y":a.y,"trapped":matches!(a.state,AlienState::Trapped(_)),"id":a.id,"kind":a.kind.as_str()}))
            .collect();
        let holes: Vec<serde_json::Value> = self.holes.iter().map(|h| {
            let (st, t, total) = match &h.state {
                HoleState::Digging(t)           => ("dig",     *t, DIG_TICKS),
                HoleState::Open(t)              => ("open",    *t, HOLE_LIFE),
                HoleState::Trapped{escape,..}   => ("trapped", *escape, ESCAPE_TICKS),
                HoleState::Filling{ticks,..}    => ("fill",    *ticks,  FILL_TICKS),
            };
            serde_json::json!({"x":h.x,"y":h.y,"st":st,"t":t,"total":total})
        }).collect();
        let items:  Vec<serde_json::Value> = self.items.iter().map(|it| serde_json::json!({"x":it.x,"y":it.y,"kind":it.kind.as_str(),"id":it.id})).collect();
        let chests: Vec<serde_json::Value> = self.chests.iter().map(|ch| serde_json::json!({"x":ch.x,"y":ch.y,"id":ch.id})).collect();
        let others: Vec<serde_json::Value> = self.players.iter().enumerate()
            .filter(|(i, s)| *i != pi && s.phase == Phase::Playing)
            .map(|(_, s)| serde_json::json!({
                "x": s.player.x, "y": s.player.y,
                "dir": s.player.dir.name(),
                "name": s.profile.name,
                "hp": s.player.hp, "max_hp": s.player.max_hp,
                "poisoned": s.player.poison_ticks > 0,
            }))
            .collect();
        serde_json::json!({
            "type": "state",
            "phase": if slot.phase == Phase::Playing { "playing" } else { "gameover" },
            "profile": {"name": slot.profile.name, "player_id": slot.profile.user_id},
            "inventory": {"green": slot.inventory.green, "red": slot.inventory.red, "blue": slot.inventory.blue},
            "player": {
                "x": p.x, "y": p.y, "hp": p.hp, "max_hp": p.max_hp,
                "score": p.score, "level": p.level, "exp": p.exp, "exp_next": p.exp_next,
                "poisoned": p.poison_ticks > 0, "poison_ticks": p.poison_ticks,
                "dir": p.dir.name(), "inv": p.invincible > 0,
            },
            "others": others,
            "aliens": aliens, "holes": holes, "items": items, "chests": chests,
            "event": slot.event, "tick": self.tick,
            "floor": self.floor_num, "goal": {"x": self.goal.0, "y": self.goal.1},
        }).to_string()
    }

    fn grid_json_with_bonus(&self, cleared_floor: u32, bonus: u32) -> String {
        serde_json::json!({
            "type": "grid",
            "grid": self.grid,
            "floor": self.floor_num,
            "goal_x": self.goal.0, "goal_y": self.goal.1,
            "bonus": bonus,
            "cleared_floor": cleared_floor,
        }).to_string()
    }
}

// ── Maze helpers ──────────────────────────────────────────────────────────────
fn bfs_farthest(grid: &Vec<Vec<u8>>, sx: usize, sy: usize) -> (usize, usize) {
    let mut vis = vec![vec![false; COLS]; ROWS];
    let mut q: VecDeque<(usize, usize)> = VecDeque::new();
    let mut last = (sx, sy);
    vis[sy][sx] = true;
    q.push_back((sx, sy));
    while let Some((x, y)) = q.pop_front() {
        last = (x, y);
        for (dx, dy) in [(0i32,-1),(0,1),(-1,0),(1,0)] {
            let nx = x as i32 + dx; let ny = y as i32 + dy;
            if nx>=0 && ny>=0 && (nx as usize)<COLS && (ny as usize)<ROWS {
                let (nx, ny) = (nx as usize, ny as usize);
                if grid[ny][nx]==1 && !vis[ny][nx] { vis[ny][nx]=true; q.push_back((nx,ny)); }
            }
        }
    }
    last
}

fn pick_start(grid: &Vec<Vec<u8>>, seed: u64) -> (usize, usize) {
    let cells: Vec<(usize, usize)> = (1..ROWS).step_by(2)
        .flat_map(|y| (1..COLS).step_by(2)
            .filter_map(move |x| if grid[y][x]==1 { Some((x,y)) } else { None }))
        .collect();
    if cells.is_empty() { return (1, 1); }
    cells[(seed as usize).wrapping_mul(2654435761) % cells.len()]
}

fn make_chests(grid: &Vec<Vec<u8>>, floor: u32, start: (usize,usize), goal: (usize,usize)) -> Vec<Chest> {
    let count = if floor <= 2 { 1 } else { 2 };
    let mut cells: Vec<(usize,usize,usize)> = (1..ROWS-1)
        .flat_map(|y| (1..COLS-1).filter_map(move |x| {
            if grid[y][x] != 1 || (x,y)==start || (x,y)==goal { return None; }
            let dist = x.abs_diff(start.0) + y.abs_diff(start.1);
            if dist < 8 { return None; }
            Some((x, y, dist))
        })).collect();
    cells.sort_by(|a, b| b.2.cmp(&a.2));
    let mut out = Vec::new();
    for (i, (x, y, _)) in cells.into_iter().enumerate() {
        if out.iter().all(|ch: &Chest| ch.x.abs_diff(x) + ch.y.abs_diff(y) >= 6) {
            out.push(Chest { x, y, id: 2000 + i + floor as usize * 10 });
            if out.len() >= count { break; }
        }
    }
    out
}

fn make_aliens(grid: &Vec<Vec<u8>>, floor: u32, start: (usize,usize)) -> Vec<Alien> {
    let extra = (floor / 3) as usize;
    let count = (INITIAL_ALIENS + extra).min(8);
    let mut cells: Vec<(usize,usize,usize)> = (1..ROWS-1)
        .flat_map(|y| (1..COLS-1).filter_map(move |x| {
            if grid[y][x] != 1 || (x,y)==start { return None; }
            Some((x, y, x.abs_diff(start.0) + y.abs_diff(start.1)))
        })).collect();
    cells.sort_by(|a, b| b.2.cmp(&a.2));
    let mut picked: Vec<(usize,usize)> = Vec::new();
    for (x, y, _) in cells {
        if picked.iter().all(|&(px,py)| px.abs_diff(x) + py.abs_diff(y) >= 6) {
            picked.push((x, y));
            if picked.len() >= count { break; }
        }
    }
    if picked.is_empty() { picked.push(start); }
    while picked.len() < count { picked.push(*picked.last().unwrap()); }
    picked.into_iter().enumerate().map(|(i, (ax, ay))| {
        let kind = if floor >= 4 && i == count.saturating_sub(1) && floor % 3 == 1 {
            AlienKind::Heal
        } else if i % 3 == 2 { AlienKind::Poison } else { AlienKind::Damage };
        Alien { x: ax, y: ay, kind, state: AlienState::Active, move_timer: BASE_SPEED + i as u32 * 3, id: i }
    }).collect()
}

// ── Room ──────────────────────────────────────────────────────────────────────
pub type SharedRoom = Arc<Mutex<Room>>;

pub struct Room {
    floors: HashMap<u32, Arc<Mutex<FloorInstance>>>,
    scores: Arc<Mutex<ScoreBoard>>,
}

impl Room {
    pub fn new(scores: Arc<Mutex<ScoreBoard>>) -> Self {
        Room { floors: HashMap::new(), scores }
    }

    fn get_or_create_floor(&mut self, floor_num: u32, seed: u64) -> Arc<Mutex<FloorInstance>> {
        if let Some(f) = self.floors.get(&floor_num) {
            return Arc::clone(f);
        }
        let floor = Arc::new(Mutex::new(FloorInstance::new(floor_num, seed, Arc::clone(&self.scores))));
        self.floors.insert(floor_num, Arc::clone(&floor));
        let f2 = Arc::clone(&floor);
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(TICK_MS));
            loop {
                ticker.tick().await;
                f2.lock().unwrap().tick_all();
            }
        });
        floor
    }
}

// ── Client messages ───────────────────────────────────────────────────────────
#[derive(Deserialize)]
struct StartProfile {
    #[serde(default)]
    player_id: Option<String>,
    name: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientMsg {
    Start   { profile: StartProfile },
    Move    { dir: String },
    Act,
    Useitem { kind: String },
    Submit  { name: String, score: u32 },
    Restart,
}

// ── WebSocket handler ─────────────────────────────────────────────────────────
pub async fn run(
    mut socket: WebSocket,
    scores: Arc<Mutex<ScoreBoard>>,
    player_count: Arc<AtomicUsize>,
    room: SharedRoom,
) {
    let _guard = PlayerCountGuard::new(player_count);
    let session_id = now_ns();
    eprintln!("[dungeon] session_open id={}", session_id);

    let (tx, mut rx) = unbounded_channel::<String>();
    let mut current_floor: Option<Arc<Mutex<FloorInstance>>> = None;

    // Send title immediately
    {
        let sb  = scores.lock().unwrap();
        let msg = serde_json::json!({"type":"title","scores":sb.list(),"min_score":sb.min_score()}).to_string();
        let _   = tx.send(msg);
    }

    loop {
        tokio::select! {
            // Game → Client
            Some(msg) = rx.recv() => {
                if socket.send(Message::Text(msg)).await.is_err() { break; }
            }
            // Client → Game
            msg = socket.recv() => {
                let Some(Ok(message)) = msg else {
                    eprintln!("[dungeon] session_recv_end id={}", session_id);
                    break;
                };
                let txt = match message {
                    Message::Text(t)    => t,
                    Message::Ping(p)    => { let _ = socket.send(Message::Pong(p)).await; continue; }
                    Message::Pong(_)    => continue,
                    Message::Binary(_)  => continue,
                    Message::Close(_)   => break,
                };
                let Ok(cm) = serde_json::from_str::<ClientMsg>(&txt) else { continue; };
                match cm {
                    ClientMsg::Start { profile } => {
                        let name = profile.name.trim().to_string();
                        if name.is_empty() { continue; }

                        // Leave current floor
                        if let Some(ref fa) = current_floor {
                            fa.lock().unwrap().remove_player(session_id);
                        }

                        // Join floor 1
                        let floor_arc = {
                            let mut r = room.lock().unwrap();
                            r.get_or_create_floor(1, now_ns())
                        };
                        {
                            let mut f = floor_arc.lock().unwrap();
                            let player = Player::new(f.start.0, f.start.1);
                            f.add_player(session_id, player,
                                PlayerProfile { user_id: profile.player_id, name },
                                Inventory::new(), 0, tx.clone());
                            let pi      = f.player_idx(session_id).unwrap();
                            let grid_m  = f.grid_json_with_bonus(0, 0);
                            let state_m = f.to_json_for(pi);
                            let _ = tx.send(grid_m);
                            let _ = tx.send(state_m);
                        }
                        current_floor = Some(floor_arc);
                    }

                    ClientMsg::Move { dir } => {
                        let Some(ref fa) = current_floor else { continue; };
                        let next_floor_opt = {
                            let mut f = fa.lock().unwrap();
                            let result = f.apply_move(session_id, &dir);
                            if let Some(pi) = f.player_idx(session_id) {
                                let _ = tx.send(f.to_json_for(pi));
                            }
                            result
                        };
                        if let Some(next_floor_num) = next_floor_opt {
                            let (cleared_floor, slot) = {
                                let mut f = fa.lock().unwrap();
                                (f.floor_num, f.remove_player(session_id))
                            };
                            if let Some(mut s) = slot {
                                let bonus = 100 * cleared_floor;
                                s.player.score += bonus;
                                let next_arc = {
                                    let mut r = room.lock().unwrap();
                                    r.get_or_create_floor(next_floor_num, now_ns())
                                };
                                {
                                    let mut nf = next_arc.lock().unwrap();
                                    nf.add_player(session_id, s.player, s.profile, s.inventory, s.kills, tx.clone());
                                    let pi      = nf.player_idx(session_id).unwrap();
                                    let grid_m  = nf.grid_json_with_bonus(cleared_floor, bonus);
                                    let state_m = nf.to_json_for(pi);
                                    let _ = tx.send(grid_m);
                                    let _ = tx.send(state_m);
                                }
                                current_floor = Some(next_arc);
                            }
                        }
                    }

                    ClientMsg::Act => {
                        let Some(ref fa) = current_floor else { continue; };
                        fa.lock().unwrap().apply_act(session_id);
                    }

                    ClientMsg::Useitem { kind } => {
                        let Some(ref fa) = current_floor else { continue; };
                        let msg = {
                            let mut f = fa.lock().unwrap();
                            f.apply_use_item(session_id, &kind);
                            f.player_idx(session_id).map(|pi| f.to_json_for(pi))
                        };
                        if let Some(m) = msg { let _ = tx.send(m); }
                    }

                    ClientMsg::Submit { name, score } => {
                        let msg = {
                            let mut sb = scores.lock().unwrap();
                            let rank   = sb.add(name, score);
                            serde_json::json!({"type":"submitted","rank":rank,"scores":sb.list(),"min_score":sb.min_score()}).to_string()
                        };
                        let _ = tx.send(msg);
                    }

                    ClientMsg::Restart => {
                        if let Some(ref fa) = current_floor {
                            fa.lock().unwrap().remove_player(session_id);
                        }
                        current_floor = None;
                        let msg = {
                            let sb = scores.lock().unwrap();
                            serde_json::json!({"type":"title","scores":sb.list(),"min_score":sb.min_score()}).to_string()
                        };
                        let _ = tx.send(msg);
                    }
                }
            }
        }
    }

    // Cleanup on disconnect
    eprintln!("[dungeon] session_close id={}", session_id);
    if let Some(fa) = current_floor {
        fa.lock().unwrap().remove_player(session_id);
    }
}
