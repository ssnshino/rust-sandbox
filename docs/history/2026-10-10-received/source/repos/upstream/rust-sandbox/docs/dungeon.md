# あやかダンジョン × 平安京エイリアン 仕様書

`ssnshino/rust-sandbox` で動く Rust 製ダンジョン探索ゲームの仕様まとめ。

作成: 2026-04-15  
最終更新: 2026-04-15

---

## 目次

1. [概要](#概要)
2. [システム構成図](#システム構成図)
3. [ゲーム全体フロー](#ゲーム全体フロー)
4. [フロア遷移フロー](#フロア遷移フロー)
5. [WebSocket メッセージシーケンス](#websocket-メッセージシーケンス)
6. [迷路生成](#迷路生成)
7. [ゲームロジック](#ゲームロジック)
8. [プロトコル（正確な JSON）](#プロトコル正確な-json)
9. [フロントエンド](#フロントエンド)
10. [ハイスコア](#ハイスコア)
11. [設計メモ・既知の挙動](#設計メモ既知の挙動)
12. [今後の拡張メモ](#今後の拡張メモ)

---

## 概要

- 平安京エイリアン風の「穴を掘ってエイリアンを閉じ込める」メカニクスと
  ローグライク的なフロア進行を組み合わせたダンジョン探索ゲーム
- 基本は 41×41 グリッド迷路（再帰バックトラッカー生成）。序盤 1〜3F は縮小サイズ
- サーバーサイドで 150ms ティックのゲームステートを管理
- フォグオブウォー、HP 制、フロア進行、ハイスコア対応
- Web Audio API による手続き的サウンドエフェクト（音源ファイルなし）

---

## システム構成図

```
┌─────────────────────────────────────────────────────────────────┐
│ Browser                                                         │
│                                                                 │
│  dungeon.html                                                   │
│  ┌─────────────┐   WS メッセージ    ┌──────────────────────┐   │
│  │ JS Game Loop│ ←─────────────── │  axum WebSocket      │   │
│  │ (rAF 60fps) │                   │  run() / tokio::select│  │
│  │             │ ──────────────── →│                      │   │
│  │  Canvas     │   入力イベント     │  Game struct          │   │
│  │  540×540    │                   │  (150ms tick loop)   │   │
│  └─────────────┘                   └──────────┬───────────┘   │
│                                               │                │
└───────────────────────────────────────────────│────────────────┘
                                                │
                          ┌─────────────────────┼──────────────────┐
                          │  axum Server        │                  │
                          │                     ▼                  │
                          │  GET  /dungeon  ─→ dungeon.html        │
                          │  GET  /ws/dungeon ─→ run(ws, scores)   │
                          │  GET  /api/dungeon/scores ─→ ScoreBoard│
                          │  POST /api/dungeon/scores ─→ ScoreBoard│
                          │                                        │
                          │  dungeon.rs   ← dungeon_gen.rs         │
                          │  scores.rs    (Breakout と共通)        │
                          │                                        │
                          │  dungeon_scores.json  (永続)           │
                          └────────────────────────────────────────┘
```

### ファイル構成

```
app/src/
├── main.rs            # ルーティング（全ゲーム共通）。GIT_HASH 注入
├── dungeon.rs         # ゲームロジック・WebSocket ハンドラ
├── dungeon_gen.rs     # 迷路生成（再帰バックトラッカー + ブレイド処理）
├── scores.rs          # ハイスコア管理（Breakout と共通）
└── dungeon.html       # ゲームページ（描画・入力・UI）
```

### WebSocket セッション

- 接続 1 本につき独立したゲームインスタンス（`Arc<Mutex<ScoreBoard>>` は共有）
- サーバーが 150ms ティックでゲームを進め、毎ティック `state` を送信
- クライアントからの入力は `tokio::select!` で非同期受信
- MutexGuard は `.await` をまたがないよう `{}` スコープで即ドロップ

---

## ゲーム全体フロー

### サーバー側ゲーム状態機

```
[接続]
  │
  ▼
[Title 待機]
  │ ClientMsg::Start
  ▼
  ─ Game::new(seed) ─→ grid_json 送信 ─→ state 送信
  │
  ▼
[Playing]  ←──────────────────────────────────────────┐
  │  150ms ごとに tick()                               │
  │  ClientMsg::Move / Act を受けたら即時処理 + 送信   │
  │                                                   │
  ├──[ゴール到達]──→ advance_floor() ─→ grid_json 送信 ─┘
  │                                   （floor++、新マップ）
  │
  └──[HP = 0]──→ Phase::GameOver
                  │
                  ▼
               gameover 送信（score, event, qualifies, scores, min_score）
                  │
                  ├──[ClientMsg::Submit]──→ ScoreBoard に登録 ──→ submitted 送信
                  │
                  └──[ClientMsg::Restart]──→ Title 待機へ戻る
                                             send_title() 送信
```

### クライアント側画面遷移

```
[title div 表示]
  ├── ランキング表示
  └── START ボタン
        │ onclick: startGame() → WS send {type:"start"}
        ▼
[game div 表示]  ←─────────────────────────────────────────┐
  │                                                        │
  │  rAF ループ: gameActive=true の間 render() を呼ぶ      │
  │                                                        │
  ├──[grid 受信, cleared_floor=0]                          │
  │     → grid/goalPos/floor を更新、描画開始              │
  │                                                        │
  ├──[grid 受信, cleared_floor>0]  ← フロアクリア          │
  │     → gameActive=false（描画停止・黒塗り）             │
  │     → FLOOR N CLEAR! 表示 (1400ms)                    │
  │     → grid/goalPos/floor を更新                        │
  │     → FLOOR N+1 START! 表示 (1200ms)                  │
  │     → gameActive=true（描画再開）──────────────────────┘
  │
  └──[gameover 受信]
        → gameActive=false
        → #gameover div 表示
              │
              ├──[ハイスコア] 名前入力フォーム表示
              │   └── submit → WS send {type:"submit", ...}
              │       └── submitted 受信 → ランキング更新
              │
              └──[restart ボタン / Space] → WS send {type:"restart"}
                    → title div へ
```

---

## フロア遷移フロー

フロアクリアから次フロア開始までのタイムライン：

```
プレイヤーがゴールに乗る
       │
       ▼ [サーバー]
  advance_floor()
  ├── cleared_floor = 現在フロア番号を保存
  ├── floor_bonus 計算・スコア加算
  ├── floor++
  ├── 新迷路生成（generate_for_floor）
  ├── start/goal を再配置
  ├── player 位置リセット
  ├── holes クリア
  └── aliens リセット（フロアに応じた数・速度）

  grid_json 送信（cleared_floor > 0 を含む）
  state 送信
       │
       ▼ [クライアント]

  t=0ms
  ├── gameActive = false（描画ループ停止）
  ├── ctx.fillRect → キャンバス黒塗り
  ├── 新 grid / goalPos / floor を変数に反映（まだ描画しない）
  └── FLOOR N CLEAR! + ボーナス表示開始

  t=1400ms
  ├── FLOOR N+1 START! 表示開始
  └── setTimeout(1200ms) を追加セット

  t=2600ms
  ├── FLOOR N+1 START! 表示終了
  └── gameActive = true（描画再開 → 新マップが初めて描画される）
```

---

## WebSocket メッセージシーケンス

### 通常ゲーム開始

```
Client                          Server
  │                               │
  │─── (WS 接続) ────────────────→│
  │                               │
  │←── title (scores, min_score) ─│  接続直後に送信
  │                               │
  │─── {type:"start"} ───────────→│
  │                               │
  │←── grid (floor=1, ...) ───────│  Game::new() + grid_json
  │←── state ─────────────────────│  初期 state
  │                               │
  │ ............150ms tick........ │
  │←── state ─────────────────────│
  │←── state ─────────────────────│
  │                               │
  │─── {type:"move", dir:"up"} ──→│
  │←── state ─────────────────────│  即時返信（move 処理後）
  │                               │
```

### フロアクリア

```
Client                          Server
  │                               │
  │─── {type:"move", dir:"right"} →│  ゴールマスに乗る
  │←── grid (cleared_floor=N) ────│  advance_floor() 後の新グリッド
  │←── state ─────────────────────│
  │                               │
  │ [クライアントは黒塗り + CLEAR! 表示中、state 受信は続く]
  │←── state ─────────────────────│
  │←── state ─────────────────────│
  │                               │
  │ [t=2600ms で gameActive=true → 新マップ描画開始]
```

### ゲームオーバー

```
Client                          Server
  │                               │
  │←── gameover (score, qualifies, scores) ─│
  │                               │
  │─── {type:"submit", name, score} ──→│  ※ハイスコア時のみ
  │←── submitted (rank, scores) ──│
  │                               │
  │─── {type:"restart"} ─────────→│
  │←── title ─────────────────────│
```

---

## 迷路生成

### フロアサイズ

| フロア | グリッドサイズ | 有効パスセル概算 |
|--------|-------------|--------------|
| 1F     | 17 × 17     | 小さい迷路    |
| 2F     | 23 × 23     | 中程度        |
| 3F     | 29 × 29     | やや広め      |
| 4F以降 | 41 × 41     | フルサイズ    |

グリッドは常に `COLS=41, ROWS=41` の配列として確保し、有効範囲のみに迷路を生成する。

### アルゴリズム（dungeon_gen.rs）

```
generate_for_floor(seed, floor)
  │
  ├── floor_size(floor) でアクティブ範囲を決定
  ├── SmallRng::seed_from_u64(seed) で乱数初期化
  ├── carve(grid, 1, 1, ...) ─── 再帰バックトラッカー
  │     ├── (1,1) からスタート
  │     ├── 4 方向をランダムにシャッフル
  │     ├── 2 歩先が未訪問かつ範囲内 → 壁を開通して再帰
  │     └── パーフェクト迷路（ループなし）が完成
  │
  └── add_braids(grid, ..., rate=0.18)
        ├── 壁セルのうち「両隣がパス」なものを検出
        └── 約 18% の確率で開通 → ループを追加（行き止まり削減）
```

### start / goal の決め方

```
pick_start(grid, seed)
  → 有効パスセル一覧から seed を使った擬似乱数で1点選択

bfs_farthest(grid, sx, sy)
  → start から BFS で最遠のパスセルを goal に
  （必然的に迷路の「端」になる）
```

---

## ゲームロジック

### 定数一覧（dungeon.rs）

| 定数 | 値 | 説明 |
|------|-----|------|
| `MAX_HP` | 5 | 初期 HP |
| `DIG_TICKS` | 5 | 穴掘り完了までのティック数（750ms） |
| `FILL_TICKS` | 5 | 穴が埋まるまでのティック数（750ms） |
| `HOLE_LIFE` | 60 | Open 状態の持続ティック（9秒） |
| `ESCAPE_TICKS` | 30 | 閉じ込めエイリアンが逃げるまでのティック（4.5秒） |
| `INVINCIBLE_TICKS` | 20 | ダメージ後の無敵時間（3秒） |
| `BASE_SPEED` | 4 | エイリアン基本速度（tick/move、3F以降通常時） |
| `ANGRY_SPEED` | 2 | エイリアン怒り速度（tick/move、3F以降） |
| `ANGRY_DIST` | 6 | 怒り判定のマンハッタン距離しきい値 |
| `TICK_MS` | 150 | サーバーティック間隔（ms） |
| `INITIAL_ALIENS` | 4 | 開始時エイリアン数（基準） |
| `RESPAWN_TICKS` | 40 | 倒されたエイリアンが復活するまでのティック（6秒） |

### フロア別エイリアン速度

| フロア | 通常速度 | 怒り速度 | 説明 |
|--------|---------|---------|------|
| 1F | 10 tick/move | 7 tick/move | 遅い |
| 2F | 6 tick/move  | 4 tick/move | 中程度 |
| 3F以降 | 4 tick/move | 2 tick/move | `BASE_SPEED` / `ANGRY_SPEED` |

「怒り」状態：エイリアンとプレイヤーのマンハッタン距離が `ANGRY_DIST`（6）以下のとき。

### エイリアン数のフロア進行

```
count = min(INITIAL_ALIENS + floor/3, 8)

1F:  4体
3F:  5体
6F:  6体
9F:  7体
12F: 8体（上限）
```

配置は start から最も遠い座標順に、互いの距離 6 以上を確保して分散配置。

### エイリアン状態

```
AlienState:
  Active          ─ 通常追跡中
  Trapped(hole_id)─ 穴に捕獲された（hole と連動）
  Dead(ticks)     ─ 倒された。RESPAWN_TICKS 後に画面端に復活
```

復活位置はプレイヤーが左半分にいれば右端、右半分にいれば左端（y も同様）。

### 穴のステートマシン

```
         [act on path]
  なし ──────────────────→ Digging(ticks)
                                │ ticks が 0 になる
                                ▼
                           Open(life) ─────────────────→ なし（life=0 で自然消滅）
                           │         │
              [エイリアン侵入]   [act on Open]
                           │         │
                           ▼         ▼
              Trapped{alien_id,   Filling{ticks,
               escape}             alien_id:None}
                  │  │                  │
    [escape=0]    │  │ [act on Trapped]  │ [ticks=0]
   エイリアン復活 │  └──────────────────→ なし（alien なし）
                  │         ↓
                  │   Filling{ticks,
                  │    alien_id:Some(id)}
                  │         │ [ticks=0]
                  └─────────→ エイリアンを Dead(RESPAWN_TICKS) に
                              スコア加算: 10 + kills*5 点
```

| 状態 | プレイヤー通過 | エイリアン通過 |
|------|:------------:|:------------:|
| Digging | ✗ | ✗ |
| Open    | ✗ | ✓（落ちて Trapped へ） |
| Trapped | ✗ | ✗ |
| Filling | ✗ | ✗ |

> **注意**: Open のみ禁止では Digging 中をすり抜けるバグが発生する。
> 全状態で通過禁止にする必要がある。

### エイリアン AI（BFS パスファインド）

```
bfs_next(from, to) の blocked 判定:
  - HoleState::Trapped{..}  → 移動不可
  - HoleState::Open         → 移動可能（落ちる）
  - HoleState::Digging      → 移動不可
  - HoleState::Filling      → 移動不可
```

Open 穴を通行可能にすることで、エイリアンが Open マスに踏み込んで Trapped 遷移が発生する。

### スコア計算

| 行動 | 点数 |
|------|------|
| エイリアンを穴で倒す | `10 + kills × 5` 点（累計撃破数 kills に応じて増加） |
| フロアクリア | `floor × 100` 点（cleared_floor 番号 × 100） |

> `kills` はゲーム全体の累計。倒すほど 1 体あたりの点数が上がる。

### ゲームオーバー条件

- HP が 0 になる
  - エイリアンに接触（無敵時間外）
  - 閉じ込め穴に落とされる（Filling → HP減少は未実装、現状はエイリアン接触のみ）

---

## プロトコル（正確な JSON）

### サーバー → クライアント

#### `title`（接続直後 / restart 後）

```json
{
  "type": "title",
  "scores": [{"name": "あやか", "score": 3000}],
  "min_score": 800
}
```

#### `grid`（ゲーム開始時・フロアクリア時）

```json
{
  "type":          "grid",
  "grid":          [[0,1,0,...], ...],
  "floor":         2,
  "goal_x":        38,
  "goal_y":        38,
  "bonus":         100,
  "cleared_floor": 1
}
```

| フィールド | 説明 |
|-----------|------|
| `grid` | 41×41 の二値マップ（`1`=パス、`0`=壁） |
| `floor` | 新しいフロア番号（1始まり） |
| `goal_x/y` | ゴールセル座標 |
| `bonus` | フロアクリアボーナス点（表示用） |
| `cleared_floor` | クリアしたフロア番号。初回は `0` |

#### `state`（毎ティック）

```json
{
  "type": "state",
  "phase": "playing",
  "player": {"x": 5, "y": 3, "hp": 5, "score": 300, "dir": "down", "inv": false},
  "aliens": [{"x": 10, "y": 7, "trapped": false, "id": 0}],
  "holes":  [{"x": 6, "y": 3, "st": "open", "t": 45, "total": 60}],
  "event":  "kill",
  "tick":   42,
  "floor":  1,
  "goal":   {"x": 38, "y": 38}
}
```

| フィールド | 説明 |
|-----------|------|
| `player.inv` | 無敵中なら `true`（点滅描画用） |
| `player.dir` | 向き（`"up"/"down"/"left"/"right"`）|
| `aliens[].trapped` | 穴に捕獲されているなら `true` |
| `holes[].st` | `"dig"` / `"open"` / `"trapped"` / `"fill"` |
| `holes[].t` | 残りティック数 |
| `holes[].total` | その状態の最大ティック数（進行バー描画用） |
| `event` | そのティックのイベント（後述）。なければ `null` |

`event` の値:

| 値 | タイミング |
|----|-----------|
| `"dig"` | 穴掘り開始 |
| `"trapped"` | エイリアンを穴に閉じ込めた |
| `"kill"` | 穴を埋めてエイリアンを倒した |
| `"dmg"` | プレイヤーがダメージを受けた |
| `"floor_complete"` | フロアクリア（ゴール到達） |
| `"act"` | 穴を埋めた（エイリアンなし） |
| `"step"` | 移動（通常歩行音） |

#### `gameover`

```json
{
  "type":       "gameover",
  "score":      1500,
  "event":      "dmg",
  "qualifies":  false,
  "scores":     [{"name": "あやか", "score": 3000}],
  "min_score":  800
}
```

`qualifies`: 現在のハイスコアランキングに入るなら `true`。

#### `submitted`（ハイスコア登録後）

```json
{
  "type":      "submitted",
  "rank":      3,
  "scores":    [{"name": "あやか", "score": 3000}],
  "min_score": 800
}
```

`rank` は 1-based。

### クライアント → サーバー

| メッセージ | 送信タイミング | JSON |
|-----------|--------------|------|
| `start` | タイトル画面で START ボタン押下 | `{"type":"start"}` |
| `move` | 移動入力（キーボード / ぷにこん） | `{"type":"move","dir":"up"}` |
| `act` | ACT ボタン / X キー | `{"type":"act"}` |
| `submit` | ハイスコア名前登録 | `{"type":"submit","name":"あやか","score":1500}` |
| `restart` | ゲームオーバー画面で restart | `{"type":"restart"}` |

`move.dir`: `"up"` / `"down"` / `"left"` / `"right"`

---

## フロントエンド

### 描画システム

```
Canvas: 540×540px（レスポンシブにスケーリング）

定数:
  TILE = 60px    ← 1 マスのピクセルサイズ
  VIEW = 9       ← 表示マス数（9×9）
  HALF = 4       ← 視野の半径（プレイヤー中心）
  FOG_R = 4      ← フォグオブウォーの視野半径（マンハッタン距離）

カメラ:
  cam = { cx: player.x - HALF, cy: player.y - HALF }
  gridToCanvas(gx, gy, cam) = { sx: (gx - cam.cx) * TILE, sy: (gy - cam.cy) * TILE }
```

### フォグオブウォー

```
visited: Set<"x,y">  ← クライアント側で蓄積

毎フレーム:
  プレイヤー周囲 FOG_R=4 以内（マンハッタン距離）を visited に追加

描画時:
  未訪問         → 描画しない（黒のまま）
  訪問済み       → 薄暗く描画（#0a0a12）
  現在視界内     → 通常描画（#1c1c2e）
```

### 描画要素

| 要素 | 見た目 |
|------|--------|
| 壁 | 濃いグレー（`#3d3020` 訪問済み / 非表示） |
| 床（パス） | 暗いパープル系（`#1c1c2e` 通常 / `#0a0a12` 訪問済み薄暗） |
| プレイヤー | 白い円 + ハイライト。無敵中は 80ms 周期で点滅 |
| エイリアン（通常） | 緑の円 |
| エイリアン（怒り） | 赤の円（距離 ≤ ANGRY_DIST） |
| 穴 Digging | 薄茶色 |
| 穴 Open | 黒い穴（影付き） |
| 穴 Trapped | 赤みがかった穴 |
| 穴 Filling | 砂色 |
| ゴール | 金色タイル + ↓ シンボル（`Math.sin(Date.now())` で点滅） |

### HUD

```
┌─────────────────────────────────────┐
│ SCORE 300    FL.2    HIGH 3000       │
│                            ♥♥♥♡♡   │
└─────────────────────────────────────┘
```

### エフェクト

| エフェクト | トリガー | 実装 |
|-----------|---------|------|
| ダメージフラッシュ | `event="dmg"` | `#dmg-flash` div の CSS keyframe animation |
| フロアクリアフラッシュ | `cleared_floor > 0` の grid 受信 | `#floor-flash` div（金色オーバーレイ） |
| ゴール点滅 | 毎フレーム | rAF ループ内 `Math.sin(Date.now())` で透明度変化 |

### 音（Web Audio API）

AudioContext はゲーム開始時（START ボタン押下）に初期化。

| イベント | 音の特徴 |
|---------|---------|
| `"dig"` | 短いノイズバースト（ザッ） |
| `"trapped"` | 下降トーン（エイリアン捕獲） |
| `"kill"` | 上昇トーン + ノイズ（撃破音） |
| `"dmg"` | 不快な低音（ダメージ） |
| `"floor_complete"` | 明るいファンファーレ（4 音和音） |
| `"gameover"` | 長い下降音 |
| `"act"` | 短いクリック音（穴埋め） |
| `"step"` | 微細な足音 |

`tone(freq, type, duration, gain, delay)` / `noise(duration)` のヘルパー関数で生成。

### スマホ対応

- タッチ端末では左下に仮想スティック（ぷにこん）、右下に ACT ボタンを表示
- ぷにこんは倒したベクトルを 4 方向へ量子化し、連続移動入力を `startRepeat` で送る
- PC / キーボード環境では従来どおり十字キーパッド（▲▼◀▶）も使える
- `touchstart` / `touchend` / `touchmove` で `preventDefault()` を呼び、スクロール防止
- ただし `e.target.closest('button,a,input,textarea')` で対話要素はスキップ
  （START ボタン・名前入力フォームが反応しなくなるバグ対策）

---

## ハイスコア

`scores.rs` の `ScoreBoard` 構造体を Breakout と共用、保存先ファイルのみ別。

- 保存先: `/workspace/app/dungeon_scores.json`
- 上位 10 件を保持、同スコアは先着優先（新しいほうが下位に挿入）
- 名前は最大 20 文字、空白トリム、空名は拒否

### API

#### `GET /api/dungeon/scores`

```json
{ "scores": [{"name": "あやか", "score": 3000}], "min_score": 800 }
```

`min_score`: 10 位のスコア。これより大きければランクイン可能。10 件未満は `0`。

#### `POST /api/dungeon/scores`

リクエスト: `{ "name": "あやか", "score": 1500 }`

レスポンス: `{ "rank": 3, "scores": [...], "min_score": 800 }`

`rank` は 1-based。ランクインしなかった場合は `null`。

---

## 設計メモ・既知の挙動

### 穴の全状態でプレイヤー通過を禁止する

`apply_move()` はすべての穴状態（Digging/Open/Trapped/Filling）で通過禁止。
Open のみ禁止にすると Digging 中をすり抜けられるバグが発生するため修正済み。

### エイリアンが Open 穴に落ちる仕組み

BFS の blocked 判定は `HoleState::Trapped{..}` のみ不可とし、
`Open` は通行可能のままにすることで、エイリアンが Open マスに踏み込んで
自動的に Trapped 状態に遷移する動作を実現している。

### floor_complete イベントの重複防止

`tick()` の先頭で毎ティック `self.event = None` を実行する。
「floor_complete を保持し続けて上書きしない」実装にすると
クリア音が鳴り続けるバグが発生するため、必ず毎ティックリセットする。

### MutexGuard と async の境界

`Arc<Mutex<ScoreBoard>>` のガードは `.await` をまたがないよう `{}` で即ドロップ。
これを怠ると `MutexGuard` が `Send` を実装していないためコンパイルエラーになる。

### BFS のエイリアン脱出計算

`Trapped` → Timeout 時に「エイリアンが逃げた後の位置」を計算する際、
不変借用（`bfs_escape_pos` 相当）を `aliens.iter_mut()`（可変借用）より
前に行う必要がある。同時に不変・可変借用を持つとコンパイルエラーになるため、
脱出位置を事前に `let` で束縛してから可変ループに入る。

### フロア遷移タイミングとクライアント描画

フロアクリア時、サーバーは即座に新グリッドを送信するが、
クライアントはそのグリッドを受け取っても `gameActive=false` にして描画を止め、
`CLEAR!`（1400ms）→ `START!`（1200ms）のメッセージが終わってから描画を再開する。
これにより旧マップと新マップが混在して見える問題を防ぐ。

### フロア遷移シードのオーバーフロー対策

`advance_floor()` でフロアシードを生成する際に `wrapping_mul` を使う。
debug build では整数オーバーフローが panic になるため、
`wrapping_mul(6364136223846793005)` のように明示的にラップアラウンドさせる。

### gameover を単一メッセージにまとめる理由

HP 0 → `phase=GameOver` が `tick()` 内で発生した場合、
`state` と `gameover` を別々に送ると `state` がクライアントに届く前に
`gameover` が届いてしまい、画面フリーズが起きることがあった。
`gameover` に `event` フィールドを含めることで単一メッセージで完結させている。

---

## 今後の拡張メモ

- [ ] エイリアン種別（速度・行動パターン違い）
- [ ] アイテム（スピードアップ、穴の寿命延長など）
- [ ] ボス敵（フロア 5 の倍数で出現）
- [ ] フロア数無制限のエンドレスモード
- [ ] 穴に落ちたときの HP 減少（現状はエイリアン接触のみ）
- [ ] リプレイ / シェア機能
- [ ] スコアに日時を記録

---

## 更新履歴

- 作成日: 2026-04-15 JST
- 最終更新日: 2026-04-15 JST
- 更新メモ:
  2026-04-15: 初版作成（仕様書として dungeon.md を新規作成）
  2026-04-15: 全面改訂。コードと照合して JSON 構造の不一致を修正、遷移図・フロー図・シーケンス図を追加
    - 修正: title / state / gameover / grid の JSON フィールド名をコード実装に合わせた
    - 修正: スコア計算式（floor×100 → 10+kills×5）、ANGRY_DIST（8→6）
    - 追加: システム構成図、ゲーム全体フロー図、フロア遷移タイムライン、WebSocket シーケンス図
    - 追加: エイリアン数フロア進行（4〜8体）、AlienState::Dead リスポーン仕組み
    - 追加: make_aliens の配置ロジック説明
    - 追加: フロア遷移の黒塗り→CLEAR→START→描画再開（2600ms）の設計メモ
