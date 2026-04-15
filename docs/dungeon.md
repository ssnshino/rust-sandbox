# あやかダンジョン × 平安京エイリアン 仕様書

`ssnshino/rust-sandbox` で動く Rust 製ダンジョン探索ゲームの仕様まとめ。

作成: 2026-04-15

---

## 概要

- 平安京エイリアン風の「穴を掘ってエイリアンを閉じ込める」メカニクスと
  ローグライク的なフロア進行を組み合わせたダンジョン探索ゲーム
- 41×41 グリッド迷路（再帰バックトラッカー生成）
- サーバーサイドで 150ms ティックのゲームステートを管理
- フォグオブウォー、HP 制、フロア進行、ハイスコア対応
- Web Audio API による手続き的サウンドエフェクト（音源ファイルなし）

---

## アーキテクチャ

### ファイル構成

```
app/src/
├── main.rs            # ルーティング（全ゲーム共通）
├── dungeon.rs         # ゲームロジック・WebSocket ハンドラ
├── dungeon_gen.rs     # 迷路生成（再帰バックトラッカー + ブレイド処理）
├── scores.rs          # ハイスコア管理（Breakout と共通）
└── dungeon.html       # ゲームページ（描画・入力・UI）
```

### ルーティング

```
GET  /dungeon              → dungeon.html を返す
GET  /ws/dungeon           → WebSocket: ゲームセッション（1接続=1ゲーム）
GET  /api/dungeon/scores   → ハイスコア一覧 JSON
POST /api/dungeon/scores   → ハイスコア登録
```

### WebSocket セッション

- 接続 1 本につき独立したゲームインスタンス（`Arc<Mutex<DungeonGame>>`）
- サーバーが 150ms ティックでゲームを進め、状態変化時に state を送信
- クライアントからの入力は非同期受信（`tokio::select!`）
- MutexGuard は `.await` をまたがないよう `{}` スコープで即ドロップ

---

## 迷路生成（dungeon_gen.rs）

### アルゴリズム

再帰バックトラッカー（DFS）でパーフェクト迷路を生成した後、
ブレイド処理で行き止まりを部分的に除去してループを作る。

```
pub const COLS: usize = 41;
pub const ROWS: usize = 41;

pub fn generate(seed: u64) -> Vec<Vec<u8>>
```

- グリッドは奇数サイズ（COLS=41, ROWS=41）
- セル座標は偶数インデックス（0,2,4,...,40）のみパス候補
- 壁（1）とパス（0）の二値マップ

### ブレイド処理（add_braids）

```rust
fn add_braids(grid: &mut Vec<Vec<u8>>, rng: &mut SmallRng, rate: f64)
```

- 行き止まり（隣接パスが1つのみのパスセル）を検出
- 両側がパスの壁をランダムに約 18% の確率で除去
- これによりループが生まれ、行き止まりが大幅に減少

---

## ゲームロジック（dungeon.rs）

### 定数

| 定数 | 値 | 説明 |
|------|-----|------|
| `MAX_HP` | 5 | 初期 HP |
| `DIG_TICKS` | 5 | 穴掘り完了までのティック数 |
| `FILL_TICKS` | 5 | 穴が埋まるまでのティック数 |
| `HOLE_LIFE` | 60 | Open 状態の持続ティック（約 9 秒） |
| `ESCAPE_TICKS` | 30 | 閉じ込められたエイリアンが逃げるまでのティック |
| `INVINCIBLE_TICKS` | 20 | ダメージ後の無敵時間（ティック） |
| `TICK_MS` | 150 | サーバーティック間隔（ms） |

### フロア別エイリアン速度

| フロア | 通常速度 | 怒り速度 |
|--------|---------|---------|
| FL1 | 10 tick/move | 7 tick/move |
| FL2 | 6 tick/move | 4 tick/move |
| FL3以上 | 4 tick/move | 2 tick/move |

「怒り」状態：プレイヤーが視界内（BFS 距離 ≤ 8）に入ったとき。

### 穴のステートマシン

```
なし
  →[act on path]→ Digging(ticks)
                    ↓ ticks==0
                  Open(life)
                  ├──[act on Open]→ Filling{ticks, alien_id: None}
                  ├──[エイリアン侵入]→ Trapped{alien_id, escape}
                  │                   ├──[escape==0]→ なし（エイリアン復活）
                  │                   └──[act on Trapped]→ Filling{ticks, alien_id: Some(id)}
                  └──[life==0]→ なし（自然消滅）

Filling{ticks, alien_id}
  ↓ ticks==0
  → なし（alien_id が Some ならエイリアンを kills、スコア加算）
```

| 状態 | プレイヤー通過 | エイリアン通過 |
|------|--------------|--------------|
| Digging | 不可 | 不可 |
| Open | 不可 | 可（落ちてTrapperへ） |
| Trapped | 不可 | 不可 |
| Filling | 不可 | 不可 |

### エイリアン AI（BFS パスファインド）

- `HoleState::Trapped{..}` のセルのみ移動不可としてBFSを実行
- `HoleState::Open(_)` は移動可能扱い → エイリアンは開いた穴に落ちる
- `HoleState::Digging` / `Filling` のセルは移動不可
- BFS の距離が最短になる方向に 1 歩ずつ移動（速度はティックカウンターで制御）

### フロア進行

```rust
fn bfs_farthest(grid, sx, sy) -> (usize, usize)
fn pick_start(grid, seed) -> (usize, usize)
fn advance_floor(&mut self)
```

- `pick_start`: シードからランダムなパスセルを選択してスタート位置とする
- `bfs_farthest`: スタートから BFS で最遠のパスセルをゴールとする
- `advance_floor`: 新しい迷路生成・ゴール再配置・プレイヤー/穴/エイリアンをリセット
- ゴールに到達するとフロアボーナス加算（`floor_bonus = floor × 100`）

### スコア計算

| 行動 | 点数 |
|------|------|
| エイリアン1体を穴で倒す | `floor × 100` 点 |
| フロアクリア | `floor × 100` 点 |

### ゲームオーバー条件

- HP が 0 になる（エイリアンに触れる、または閉じ込め穴に落とされる）

---

## プロトコル

### サーバー → クライアント

#### `title` メッセージ（接続直後）

```json
{ "type": "title", "high": 2500 }
```

#### `grid` メッセージ（フロア開始時）

```json
{
  "type":    "grid",
  "cells":   [[0,1,0,...], ...],
  "floor":   1,
  "goal_x":  38,
  "goal_y":  38,
  "bonus":   100
}
```

| フィールド | 説明 |
|-----------|------|
| `cells` | 41×41 の二値マップ（0=パス, 1=壁） |
| `floor` | 現在フロア番号（1始まり） |
| `goal_x/y` | ゴールセル座標 |
| `bonus` | フロアクリアボーナス点数（表示用） |

#### `state` メッセージ（毎ティック）

```json
{
  "type":    "state",
  "px":      5,
  "py":      3,
  "hp":      5,
  "score":   300,
  "aliens":  [{"x":10,"y":7},{"x":20,"y":15}],
  "holes":   [{"x":6,"y":3,"state":"open","life":45}],
  "event":   "kill",
  "floor":   1,
  "goal_x":  38,
  "goal_y":  38
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `px/py` | u32 | プレイヤー座標 |
| `hp` | u32 | 現在 HP |
| `score` | u32 | 現在スコア |
| `aliens` | `[{x,y}]` | 生存エイリアンの座標リスト |
| `holes` | `[{x,y,state,life}]` | 穴の座標・状態・残り寿命 |
| `event` | str? | そのティックのイベント（音出し用） |
| `floor` | u32 | 現在フロア番号 |
| `goal_x/y` | u32 | ゴール座標 |

`holes[].state` の値:

| 値 | 説明 |
|----|------|
| `"digging"` | 掘り中 |
| `"open"` | 落ちられる穴（プレイヤー通過不可） |
| `"trapped"` | エイリアンが閉じ込められている |
| `"filling"` | 埋まり中 |

`event` の値:

| 値 | タイミング |
|----|-----------|
| `"dig"` | 穴掘り開始 |
| `"trapped"` | エイリアンを穴に閉じ込めた |
| `"kill"` | 穴を埋めてエイリアンを倒した |
| `"dmg"` | プレイヤーがダメージを受けた |
| `"floor_complete"` | フロアクリア |
| `"act"` | 穴を埋めた（エイリアンなし） |
| `"step"` | 移動（通常歩行音） |

#### `gameover` メッセージ

```json
{
  "type":       "gameover",
  "score":      1500,
  "high":       2500,
  "is_hiscore": false
}
```

#### `submitted` メッセージ（ハイスコア登録後）

```json
{
  "type":   "submitted",
  "rank":   3,
  "scores": [{"name":"あやか","score":3000}, ...]
}
```

### クライアント → サーバー

```json
{ "type": "start" }
{ "type": "move", "dir": "up" }
{ "type": "act" }
{ "type": "submit", "name": "あやか" }
{ "type": "restart" }
```

| メッセージ | タイミング |
|-----------|-----------|
| `start` | タイトル画面でゲーム開始 |
| `move` | プレイヤー移動（`dir`: `"up"/"down"/"left"/"right"`） |
| `act` | 穴を掘る / 穴を埋める（プレイヤー足元） |
| `submit` | ゲームオーバー後のハイスコア名前登録 |
| `restart` | ゲームオーバー後にタイトルへ戻る |

---

## フロントエンド（dungeon.html）

### 描画システム

- Canvas 540×540px（9×9 タイルビューポート、TILE=60px）
- `const COLS=41, ROWS=41, VIEW=9, HALF=4, FOG_R=4`
- `camPos()`: プレイヤー中心で `{cx: player.x - HALF, cy: player.y - HALF}`
- `gridToCanvas(gx, gy, cam)`: グリッド座標 → Canvas ピクセル座標

### フォグオブウォー

- クライアント側の `visited: Set<string>` にマンハッタン距離 4 以内のセルを蓄積
- 未訪問 = 非表示、訪問済み = 薄暗く描画、現在視界内 = 通常描画

### 描画要素

| 要素 | 見た目 |
|------|--------|
| 壁 | 濃いグレー（#4a4a6a） |
| パス（床） | 暗いパープル系（#1a1a2e） |
| プレイヤー | 白い円 + ハイライト |
| エイリアン | 緑の円（通常）/ 赤の円（怒り状態） |
| 穴 Digging | 薄茶色 |
| 穴 Open | 黒い穴（影付き） |
| 穴 Trapped | 赤みがかった穴（エイリアン捕獲中） |
| 穴 Filling | 砂色（埋まり中） |
| ゴール | 金色の脈打つタイル + ↓ シンボル（rAF アニメーション） |

### HUD（3カラムグリッド）

```
SCORE [スコア]  FL.[フロア番号]  HIGH [ハイスコア]
♥♥♥♡♡  （右寄せ、最大5個）
```

### エフェクト

| エフェクト | 実装 |
|-----------|------|
| ダメージフラッシュ | `#dmg-flash` div の CSS animation |
| フロアクリアフラッシュ | `#floor-flash` div（金色オーバーレイ） |
| ゴール点滅 | rAF ループ内の `Math.sin(Date.now())` で透明度変化 |

### 音（Web Audio API）

AudioContext はゲーム開始時（START ボタン押下）に初期化。

| イベント | 音の特徴 |
|---------|---------|
| `"dig"` | 短いノイズバースト（ザッ） |
| `"trapped"` | 下降トーン（エイリアン捕獲） |
| `"kill"` | 上昇 + ノイズ（撃破音） |
| `"dmg"` | 不快な低音（ダメージ） |
| `"floor_complete"` | 明るいファンファーレ |
| `"gameover"` | 長い下降音 |
| `"act"` | 短いクリック音（穴埋め） |
| `"step"` | 微細な足音 |

`tone(freq, type, duration)` / `noise(duration)` のヘルパー関数で生成。

### スマホ対応

- 十字キーパッド（▲▼◀▶）+ ACT ボタンを画面下部に固定表示
- `touchstart` / `touchend` で `preventDefault()` を呼び、スクロール防止
- ただし `e.target.closest('button,a,input,textarea')` で対話要素はスキップ
  （START ボタン・名前入力フォームが反応しなくなるバグ対策）

### タイトル画面 / ゲームオーバー画面

- タイトル: ハイスコアランキング + 「ゲームを始める」ボタン
- ゲームオーバー: スコア表示 + ハイスコア時は名前入力フォーム
- 登録後: 「X位 で登録！」 + ランキング更新

---

## ハイスコア（scores.rs 共通）

Breakout と同じ `ScoreBoard` 構造体を流用、保存先ファイルのみ別。

- 保存先: `/workspace/app/dungeon_scores.json`
- `ScoreBoard::load("dungeon_scores.json")` で初期化
- 上位 10 件を保持、同スコアは先着優先

### API

#### `GET /api/dungeon/scores`

```json
{ "scores": [{"name":"あやか","score":3000}], "min_score": 800 }
```

#### `POST /api/dungeon/scores`

リクエスト: `{ "name": "あやか", "score": 1500 }`

レスポンス: `{ "rank": 3, "scores": [...], "min_score": 800 }`

---

## 既知の挙動・設計メモ

### 穴とプレイヤーの衝突

`apply_move()` はすべての穴状態（Digging/Open/Trapped/Filling）に対して通過を禁止する。
Open の穴だけを禁止すると Digging 中を通り抜けられるバグがあったため全状態対象に修正済み。

### エイリアンが Open 穴に落ちる仕組み

BFS の blocked 判定は `HoleState::Trapped{..}` のみ不可とし、
`Open` は通行可能のままにすることで、エイリアンが Open 穴のマスに踏み込んで
`Trapped` 状態に遷移する動作を実現している。

### floor_complete イベントの重複防止

`tick()` の先頭で毎ティック `self.event = None` を実行する。
「floor_complete を保持し続けて上書きしない」実装にすると
クリア音が鳴り続けるバグが発生するため、必ず毎ティックリセットする。

### MutexGuard と async の境界

`Arc<Mutex<DungeonGame>>` のガードは `.await` をまたがないよう `{}` で即ドロップ。
これを怠ると `MutexGuard` が `Send` を実装していないためコンパイルエラーになる。

### BFS のエイリアン脱出計算

`Trapped` → `Filling` 遷移時に「エイリアンが逃げた後の位置」を計算する際、
`bfs_escape_pos()` の呼び出し（不変借用）を `aliens.iter_mut()`（可変借用）より
前に行う必要がある。同時に不変・可変借用を持つとコンパイルエラーになるため、
`escape_pos` を事前に計算して `let` で束縛してから可変ループに入る。

---

## 今後の拡張メモ

- [ ] エイリアン種別（速度・行動パターン違い）
- [ ] アイテム（スピードアップ、穴の寿命延長など）
- [ ] ボス敵（フロア 5 の倍数で出現）
- [ ] フロア数無制限のエンドレスモード
- [ ] リプレイ / シェア機能
