
use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};

use crate::dungeon_gen::{generate_for_floor, COLS, ROWS};
use crate::scores::ScoreBoard;

const MAX_HP: u8 = 5;
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

struct Player { x:usize, y:usize, hp:u8, score:u32, dir:Dir, invincible:u32 }

#[derive(Clone, PartialEq)]
enum HoleState {
    Digging(u32), Open(u32),
    Trapped { alien_id:usize, escape:u32 },
    Filling { ticks:u32, alien_id:Option<usize> },
}
struct Hole { x:usize, y:usize, state:HoleState, id:usize }

#[derive(Clone, PartialEq)]
enum AlienState { Active, Trapped(usize), Dead(u32) }
struct Alien { x:usize, y:usize, state:AlienState, move_timer:u32, id:usize }

#[derive(PartialEq, Eq)]
enum Phase { Playing, GameOver }

struct Game {
    grid: Vec<Vec<u8>>,
    player: Player,
    aliens: Vec<Alien>,
    holes: Vec<Hole>,
    tick: u64,
    next_hole_id: usize,
    phase: Phase,
    event: Option<&'static str>,
    kills: u32,
    floor: u32,
    goal: (usize, usize),
    start: (usize, usize),
    new_grid_ready: bool,
    floor_bonus: u32,
    cleared_floor: u32,
}

// ── Maze helpers ──────────────────────────────────────────────────────────────
fn bfs_farthest(grid: &Vec<Vec<u8>>, sx: usize, sy: usize) -> (usize, usize) {
    let mut vis = vec![vec![false;COLS];ROWS];
    let mut q: VecDeque<(usize,usize)> = VecDeque::new();
    let mut last = (sx, sy);
    vis[sy][sx] = true;
    q.push_back((sx, sy));
    while let Some((x,y)) = q.pop_front() {
        last = (x,y);
        for (dx,dy) in [(0i32,-1),(0,1),(-1,0),(1,0)] {
            let nx=x as i32+dx; let ny=y as i32+dy;
            if nx>=0&&ny>=0&&(nx as usize)<COLS&&(ny as usize)<ROWS {
                let (nx,ny)=(nx as usize,ny as usize);
                if grid[ny][nx]==1&&!vis[ny][nx] { vis[ny][nx]=true; q.push_back((nx,ny)); }
            }
        }
    }
    last
}

fn pick_start(grid: &Vec<Vec<u8>>, seed: u64) -> (usize, usize) {
    let cells: Vec<(usize,usize)> = (1..ROWS).step_by(2)
        .flat_map(|y| (1..COLS).step_by(2)
            .filter_map(move |x| if grid[y][x]==1 { Some((x,y)) } else { None }))
        .collect();
    if cells.is_empty() { return (1,1); }
    cells[(seed as usize).wrapping_mul(2654435761) % cells.len()]
}

fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_nanos() as u64
}

// ── Game ──────────────────────────────────────────────────────────────────────
impl Game {
    fn new(seed: u64) -> Self {
        let grid = generate_for_floor(seed, 1);
        let start = pick_start(&grid, seed);
        let goal  = bfs_farthest(&grid, start.0, start.1);
        let player = Player { x:start.0,y:start.1,hp:MAX_HP,score:0,dir:Dir::Down,invincible:0 };
        let aliens = make_aliens(&grid, 1, start);
        Game { grid,player,aliens,holes:Vec::new(),tick:0,next_hole_id:0,
               phase:Phase::Playing,event:None,kills:0,
               floor:1,goal,start,new_grid_ready:false,floor_bonus:0,cleared_floor:0 }
    }

    fn advance_floor(&mut self) {
        self.cleared_floor = self.floor;           // save the floor that was just cleared
        self.floor_bonus = 100 * self.cleared_floor; // bonus based on cleared floor
        self.player.score += self.floor_bonus;
        self.floor += 1;                           // then advance
        let seed = now_ns() ^ (self.floor as u64).wrapping_mul(6364136223846793005);
        self.grid = generate_for_floor(seed, self.floor);
        self.start = pick_start(&self.grid, seed.wrapping_add(1));
        self.goal  = bfs_farthest(&self.grid, self.start.0, self.start.1);
        self.player.x = self.start.0;
        self.player.y = self.start.1;
        self.player.dir = Dir::Down;
        self.holes.clear();
        self.aliens = make_aliens(&self.grid, self.floor, self.start);
        self.new_grid_ready = true;
    }

    fn is_path(&self, x:i32, y:i32) -> bool {
        x>=0&&y>=0&&(x as usize)<COLS&&(y as usize)<ROWS&&self.grid[y as usize][x as usize]==1
    }
    fn hole_at(&self, x:usize, y:usize) -> Option<usize> {
        self.holes.iter().position(|h| h.x==x&&h.y==y)
    }

    fn bfs_next(&self, fx:usize, fy:usize, tx:usize, ty:usize) -> Option<(usize,usize)> {
        if fx==tx&&fy==ty { return None; }
        let mut vis = vec![vec![false;COLS];ROWS];
        let mut q: VecDeque<(usize,usize,usize,usize)> = VecDeque::new();
        vis[fy][fx] = true;
        for (dx,dy) in [(0i32,-1),(0,1),(-1,0),(1,0)] {
            let nx=fx as i32+dx; let ny=fy as i32+dy;
            if self.is_path(nx,ny) {
                let (nx,ny)=(nx as usize,ny as usize);
                let blocked=self.holes.iter().any(|h|h.x==nx&&h.y==ny&&matches!(h.state,HoleState::Trapped{..}));
                if !blocked&&!vis[ny][nx] { vis[ny][nx]=true; q.push_back((nx,ny,nx,ny)); }
            }
        }
        while let Some((cx,cy,fx2,fy2))=q.pop_front() {
            if cx==tx&&cy==ty { return Some((fx2,fy2)); }
            for (dx,dy) in [(0i32,-1),(0,1),(-1,0),(1,0)] {
                let nx=cx as i32+dx; let ny=cy as i32+dy;
                if self.is_path(nx,ny) {
                    let (nx,ny)=(nx as usize,ny as usize);
                    let blocked=self.holes.iter().any(|h|h.x==nx&&h.y==ny&&matches!(h.state,HoleState::Trapped{..}));
                    if !blocked&&!vis[ny][nx] { vis[ny][nx]=true; q.push_back((nx,ny,fx2,fy2)); }
                }
            }
        }
        None
    }

    fn apply_move(&mut self, dir_str:&str) {
        let Some(dir) = Dir::parse(dir_str) else { return };
        self.player.dir = dir;
        let (dx,dy) = dir.delta();
        let nx=self.player.x as i32+dx; let ny=self.player.y as i32+dy;
        if !self.is_path(nx,ny) { return; }
        let (nx,ny)=(nx as usize,ny as usize);
        if self.hole_at(nx,ny).is_some() { return; }
        self.player.x=nx; self.player.y=ny;
        // goal check
        if (self.player.x, self.player.y) == self.goal {
            self.event = Some("floor_complete");
            self.advance_floor();
        }
    }

    fn apply_act(&mut self) {
        let (dx,dy)=self.player.dir.delta();
        let tx=self.player.x as i32+dx; let ty=self.player.y as i32+dy;
        if !self.is_path(tx,ty) { return; }
        let (tx,ty)=(tx as usize,ty as usize);
        if let Some(hi)=self.hole_at(tx,ty) {
            match self.holes[hi].state.clone() {
                HoleState::Open(_) => {
                    self.holes[hi].state=HoleState::Filling{ticks:FILL_TICKS,alien_id:None};
                }
                HoleState::Trapped{alien_id,..} => {
                    self.holes[hi].state=HoleState::Filling{ticks:FILL_TICKS,alien_id:Some(alien_id)};
                }
                _ => {}
            }
        } else {
            let id=self.next_hole_id; self.next_hole_id+=1;
            self.holes.push(Hole { x:tx,y:ty,state:HoleState::Digging(DIG_TICKS),id });
        }
    }

    fn tick(&mut self) {
        self.tick+=1;
        self.event=None;
        if self.player.invincible>0 { self.player.invincible-=1; }

        // ── Holes ──
        let mut i=0;
        while i<self.holes.len() {
            let remove = match self.holes[i].state.clone() {
                HoleState::Digging(t) => {
                    if t<=1 { self.holes[i].state=HoleState::Open(HOLE_LIFE); } 
                    else { self.holes[i].state=HoleState::Digging(t-1); }
                    false
                }
                HoleState::Open(t) => { if t<=1{true}else{self.holes[i].state=HoleState::Open(t-1);false} }
                HoleState::Trapped{alien_id,escape} => {
                    if escape<=1 {
                        let (hx,hy)=(self.holes[i].x,self.holes[i].y);
                        let ep=[(0i32,-1),(0,1),(-1,0),(1,0)].iter()
                            .filter_map(|&(dx,dy)|{
                                let nx=hx as i32+dx; let ny=hy as i32+dy;
                                if self.is_path(nx,ny){Some((nx as usize,ny as usize))}else{None}
                            }).next();
                        if let Some(a)=self.aliens.iter_mut().find(|a|a.id==alien_id) {
                            a.state=AlienState::Active;
                            if let Some((nx,ny))=ep { a.x=nx; a.y=ny; }
                        }
                        true
                    } else { self.holes[i].state=HoleState::Trapped{alien_id,escape:escape-1}; false }
                }
                HoleState::Filling{ticks,alien_id:aid} => {
                    if ticks<=1 {
                        if let Some(alien_id)=aid {
                            if let Some(a)=self.aliens.iter_mut().find(|a|a.id==alien_id) {
                                a.state=AlienState::Dead(RESPAWN_TICKS);
                                self.kills+=1;
                                self.player.score+=10+self.kills*5;
                                self.event=Some("kill");
                            }
                        }
                        true
                    } else { self.holes[i].state=HoleState::Filling{ticks:ticks-1,alien_id:aid}; false }
                }
            };
            if remove{self.holes.remove(i);}else{i+=1;}
        }

        // ── Aliens ──
        let count=self.aliens.len();
        let pending: Vec<Option<(usize,usize)>> = (0..count).map(|i| {
            let (ax,ay)=(self.aliens[i].x,self.aliens[i].y);
            let active=self.aliens[i].state==AlienState::Active;
            let timer=self.aliens[i].move_timer;
            if active&&timer==0 { self.bfs_next(ax,ay,self.player.x,self.player.y) } else { None }
        }).collect();

        for ai in 0..count {
            match self.aliens[ai].state.clone() {
                AlienState::Dead(t) => {
                    if t<=1 {
                        // respawn far from player
                        let rx = if self.player.x < COLS/2 { COLS-2 } else { 1 };
                        let ry = if self.player.y < ROWS/2 { ROWS-2 } else { 1 };
                        self.aliens[ai].state=AlienState::Active;
                        self.aliens[ai].x=rx; self.aliens[ai].y=ry;
                        self.aliens[ai].move_timer=BASE_SPEED;
                    } else { self.aliens[ai].state=AlienState::Dead(t-1); }
                }
                AlienState::Trapped(_) => {}
                AlienState::Active => {
                    if self.aliens[ai].move_timer>0 {
                        self.aliens[ai].move_timer-=1;
                    } else {
                        let dx=self.aliens[ai].x as i32-self.player.x as i32;
                        let dy=self.aliens[ai].y as i32-self.player.y as i32;
                        let (base_spd,angry_spd)=match self.floor {
                            1 => (10u32,7u32),
                            2 => (6u32,4u32),
                            _ => (BASE_SPEED,ANGRY_SPEED),
                        };
                        let speed=if dx.abs()+dy.abs()<=ANGRY_DIST{angry_spd}else{base_spd};
                        self.aliens[ai].move_timer=speed;
                        if let Some((nx,ny))=pending[ai] {
                            let hi=self.holes.iter().position(|h|h.x==nx&&h.y==ny&&matches!(h.state,HoleState::Open(_)));
                            if let Some(hi)=hi {
                                let aid=self.aliens[ai].id;
                                let hid=self.holes[hi].id;
                                self.aliens[ai].state=AlienState::Trapped(hid);
                                self.aliens[ai].x=nx; self.aliens[ai].y=ny;
                                self.holes[hi].state=HoleState::Trapped{alien_id:aid,escape:ESCAPE_TICKS};
                                self.event=Some("trapped");
                            } else {
                                self.aliens[ai].x=nx; self.aliens[ai].y=ny;
                            }
                        }
                    }
                    if matches!(self.aliens[ai].state,AlienState::Active)
                        &&self.aliens[ai].x==self.player.x&&self.aliens[ai].y==self.player.y
                        &&self.player.invincible==0
                    {
                        self.player.hp=self.player.hp.saturating_sub(1);
                        self.player.invincible=INVINCIBLE_TICKS;
                        self.event=Some("dmg");
                        if self.player.hp==0 { self.phase=Phase::GameOver; }
                    }
                }
            }
        }
    }

    fn grid_json(&self) -> String {
        serde_json::json!({
            "type":"grid","grid":self.grid,"floor":self.floor,
            "goal_x":self.goal.0,"goal_y":self.goal.1,"bonus":self.floor_bonus,
            "cleared_floor":self.cleared_floor
        }).to_string()
    }

    fn to_json(&self) -> String {
        let p=&self.player;
        let aliens: Vec<serde_json::Value>=self.aliens.iter()
            .filter(|a|!matches!(a.state,AlienState::Dead(_)))
            .map(|a|serde_json::json!({"x":a.x,"y":a.y,"trapped":matches!(a.state,AlienState::Trapped(_)),"id":a.id}))
            .collect();
        let holes: Vec<serde_json::Value>=self.holes.iter().map(|h|{
            let (st,t,total)=match &h.state {
                HoleState::Digging(t)=>("dig",*t,DIG_TICKS),
                HoleState::Open(t)=>("open",*t,HOLE_LIFE),
                HoleState::Trapped{escape,..}=>("trapped",*escape,ESCAPE_TICKS),
                HoleState::Filling{ticks,..}=>("fill",*ticks,FILL_TICKS),
            };
            serde_json::json!({"x":h.x,"y":h.y,"st":st,"t":t,"total":total})
        }).collect();
        serde_json::json!({
            "type":"state",
            "phase":if self.phase==Phase::Playing{"playing"}else{"gameover"},
            "player":{"x":p.x,"y":p.y,"hp":p.hp,"score":p.score,"dir":p.dir.name(),"inv":p.invincible>0},
            "aliens":aliens,"holes":holes,"event":self.event,"tick":self.tick,
            "floor":self.floor,"goal":{"x":self.goal.0,"y":self.goal.1}
        }).to_string()
    }
}

fn make_aliens(grid: &Vec<Vec<u8>>, floor: u32, start: (usize, usize)) -> Vec<Alien> {
    let extra = (floor / 3) as usize; // add 1 alien every 3 floors, up to 8
    let count = (INITIAL_ALIENS + extra).min(8);
    let mut cells: Vec<(usize, usize, usize)> = (1..ROWS-1)
        .flat_map(|y| (1..COLS-1).filter_map(move |x| {
            if grid[y][x] != 1 || (x, y) == start {
                return None;
            }
            let dist = x.abs_diff(start.0) + y.abs_diff(start.1);
            Some((x, y, dist))
        }))
        .collect();
    cells.sort_by(|a, b| b.2.cmp(&a.2));

    let mut picked: Vec<(usize, usize)> = Vec::new();
    for (x, y, _) in cells {
        if picked.iter().all(|&(px, py)| px.abs_diff(x) + py.abs_diff(y) >= 6) {
            picked.push((x, y));
            if picked.len() >= count {
                break;
            }
        }
    }
    if picked.is_empty() {
        picked.push(start);
    }
    while picked.len() < count {
        picked.push(*picked.last().unwrap());
    }

    picked.into_iter().enumerate().map(|(i, (ax, ay))| {
        Alien { x:ax, y:ay, state:AlienState::Active, move_timer:BASE_SPEED+i as u32*3, id:i }
    }).collect()
}

#[derive(Deserialize)]
#[serde(tag="type",rename_all="lowercase")]
enum ClientMsg { Start, Move{dir:String}, Act, Submit{name:String,score:u32}, Restart }

pub async fn run(mut socket: WebSocket, scores: Arc<Mutex<ScoreBoard>>) {
    let session_id = now_ns();
    eprintln!("[dungeon] session_open id={}", session_id);
    send_title(&mut socket, &scores).await;
    let mut gs: Option<Game> = None;
    let mut ticker = interval(Duration::from_millis(TICK_MS));
    let mut gameover_sent = false;

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if let Some(ref mut g) = gs {
                    if g.phase == Phase::Playing {
                        g.tick();
                        if g.phase == Phase::GameOver && !gameover_sent {
                            // Send a single combined message so the client never misses gameover
                            gameover_sent = true;
                            let msg_str = {
                                let sb=scores.lock().unwrap();
                                let score=g.player.score;
                                serde_json::json!({
                                    "type":"gameover","score":score,
                                    "event":g.event,
                                    "qualifies":sb.qualifies(score),
                                    "scores":sb.list(),"min_score":sb.min_score()
                                }).to_string()
                            };
                            let _ = socket.send(Message::Text(msg_str)).await;
                        } else {
                            if socket.send(Message::Text(g.to_json())).await.is_err() { break; }
                        }
                    }
                }
            }
            msg = socket.recv() => {
                let Some(Ok(message)) = msg else {
                    eprintln!("[dungeon] session_recv_end id={}", session_id);
                    break;
                };
                let txt = match message {
                    Message::Text(txt) => txt,
                    Message::Ping(payload) => {
                        eprintln!("[dungeon] session_ping id={} bytes={}", session_id, payload.len());
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            eprintln!("[dungeon] session_pong_send_error id={}", session_id);
                            break;
                        }
                        continue;
                    }
                    Message::Pong(_) => {
                        eprintln!("[dungeon] session_pong id={}", session_id);
                        continue;
                    }
                    Message::Binary(payload) => {
                        eprintln!("[dungeon] session_binary id={} bytes={}", session_id, payload.len());
                        continue;
                    }
                    Message::Close(frame) => {
                        eprintln!("[dungeon] session_close id={} frame={:?}", session_id, frame);
                        break;
                    }
                };
                let Ok(cm) = serde_json::from_str::<ClientMsg>(&txt) else { continue; };
                match cm {
                    ClientMsg::Start => {
                        gameover_sent = false;
                        let seed = now_ns();
                        let g = Game::new(seed);
                        if socket.send(Message::Text(g.grid_json())).await.is_err() { break; }
                        if socket.send(Message::Text(g.to_json())).await.is_err() { break; }
                        gs = Some(g);
                    }
                    ClientMsg::Move { dir } => {
                        if let Some(ref mut g)=gs {
                            if g.phase==Phase::Playing {
                                g.apply_move(&dir);
                                // send new grid if floor advanced
                                if g.new_grid_ready {
                                    g.new_grid_ready = false;
                                    if socket.send(Message::Text(g.grid_json())).await.is_err() { break; }
                                    if socket.send(Message::Text(g.to_json())).await.is_err() { break; }
                                }
                            }
                        }
                    }
                    ClientMsg::Act => {
                        if let Some(ref mut g)=gs { if g.phase==Phase::Playing { g.apply_act(); } }
                    }
                    ClientMsg::Submit { name, score } => {
                        let msg_str = {
                            let mut sb=scores.lock().unwrap();
                            let rank=sb.add(name,score);
                            serde_json::json!({"type":"submitted","rank":rank,"scores":sb.list(),"min_score":sb.min_score()}).to_string()
                        };
                        let _ = socket.send(Message::Text(msg_str)).await;
                    }
                    ClientMsg::Restart => {
                        gs=None; gameover_sent=false;
                        send_title(&mut socket, &scores).await;
                    }
                }
            }
        }
    }
}

async fn send_title(socket: &mut WebSocket, scores: &Arc<Mutex<ScoreBoard>>) {
    let msg_str = {
        let sb=scores.lock().unwrap();
        serde_json::json!({"type":"title","scores":sb.list(),"min_score":sb.min_score()}).to_string()
    };
    let _ = socket.send(Message::Text(msg_str)).await;
}
