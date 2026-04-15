# Breakout 仕様書

`ayano-lab/docker/rust-sandbox` で動く Rust 製 Breakout ゲームの仕様まとめ。

作成: 2026-04-15

---

## 概要

- Rust + axum による WebSocket ゲームサーバー
- サーバーサイドで 60fps のゲームステートを管理し、クライアントは描画・入力・音のみ担当
- ハイスコアをサーバー側の JSON ファイルに永続保存
- スマホ・PC 両対応（スライド操作 / マウス / キーボード）

---

## アーキテクチャ

### ファイル構成

```
rust-sandbox/
├── compose.yaml
├── SPEC_BREAKOUT.md        # この文書
├── SPEC_PONG.md            # Pong 仕様書
└── app/
    ├── Cargo.toml
    ├── breakout_scores.json  # ハイスコア永続ファイル（起動時に読み込み）
    └── src/
        ├── main.rs           # ルーティング（全ゲーム共通）
        ├── breakout.rs       # ゲームロジック・WebSocket ハンドラ
        ├── scores.rs         # ハイスコア管理
        └── breakout.html     # ゲームページ（描画・入力・UI）
```

### ルーティング

```
GET  /breakout              → breakout.html を返す
GET  /ws/breakout           → WebSocket: ゲームセッション（1接続=1ゲーム）
GET  /api/breakout/scores   → ハイスコア一覧 JSON
POST /api/breakout/scores   → ハイスコア登録
```

### WebSocket セッション

- 接続 1 本につき独立したゲームインスタンス
- サーバーが 16ms ティック (60fps) でゲームを進め、毎フレーム state を送信
- クライアントからの入力は非同期受信（`tokio::select!`）

---

## プロトコル

### サーバー → クライアント（毎フレーム）

```json
{
  "ball_x":   400.0,
  "ball_y":   300.0,
  "paddle_x": 400.0,
  "bricks":   [1,1,1,...],
  "score":    0,
  "lives":    3,
  "phase":    "ready",
  "event":    null
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `ball_x/y` | f64 | ボール中心座標（ゲーム座標系） |
| `paddle_x` | f64 | パドル中心 X |
| `bricks` | `u8[]` | ブロック存在フラグ（0/1）、COLS×ROWS 順 |
| `score` | u32 | 現在スコア |
| `lives` | u32 | 残機（初期値 3） |
| `phase` | str | `"ready"` / `"playing"` / `"game_over"` / `"clear"` |
| `event` | str? | そのフレームのイベント（音出し用）、なければ `null` |

`event` の値:

| 値 | タイミング |
|----|-----------|
| `"hit_brick"` | ブロック破壊 |
| `"hit_wall"` | 左右壁・天井反射 |
| `"hit_paddle"` | パドル反射 |
| `"miss"` | ボールが画面下に落下 |

### クライアント → サーバー

```json
{ "dir": -1, "launch": false, "paddle_x": 320.0 }
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `dir` | i32 | `-1` 左 / `0` 停止 / `1` 右（キーボード用） |
| `launch` | bool | `true` でボール発射 / リスタート |
| `paddle_x` | f64? | パドル中心 X を直接指定（マウス・タッチ用）。省略時は `dir` で移動 |

`paddle_x` と `dir` は排他的に使い分ける。`paddle_x` が来たフレームは即座に反映、来なかったフレームは `dir` による速度移動にフォールバック。

---

## ゲームロジック（breakout.rs）

### フィールド定数

| 定数 | 値 |
|------|----|
| W × H | 800 × 600 |
| BALL_R | 8 px |
| PADDLE_W × PADDLE_H | 100 × 12 px |
| PADDLE_Y | H − 40 = 560 px |
| PADDLE_SPEED | 8 px/tick（キーボード時） |
| BALL_SPEED | 5.5 px/tick |
| LIVES | 3 |

### ブロック配置

| 定数 | 値 |
|------|----|
| BRICK_COLS × BRICK_ROWS | 10 × 6 = 60 ブロック |
| BRICK_W × BRICK_H | 72 × 22 px |
| BRICK_GAP_X / Y | 8 px |
| BRICK_START_X / Y | 4 / 50 px |

### スコア

- ブロック破壊で `(BRICK_ROWS - row) × 10` 点
- 行が上（row=0）ほど高得点：row 0 → 60pt、row 5 → 10pt

```
row 0 (赤)    60点
row 1 (橙)    50点
row 2 (黄)    40点
row 3 (緑)    30点
row 4 (青)    20点
row 5 (紫)    10点
```

### 発射ロジック

- `tick_count` をシードにした疑似乱数で水平角をランダム決定
- vx を `[-BALL_SPEED×0.8, +BALL_SPEED×0.8]` にクランプし、速度ベクトルの長さを BALL_SPEED に正規化
- 方向は常に上向き（vy < 0）

### パドル反射

- ボールがパドル上面に当たった場合、打点の相対位置 `rel = (ball_x - paddle_x) / (PADDLE_W/2)` で反射角を変更
- 端ほど水平寄り、中央ほど真上寄りに跳ねる
- 端ヒット時は速度を微増（`speed = BALL_SPEED + |rel|`）

### ブロック衝突判定

- 円（ボール）と矩形（ブロック）の最近傍点距離で判定
- 1フレーム最大 1 ブロックまで（貫通防止）
- overlap の X/Y 比較で反射方向（水平／垂直）を決定

### フェーズ遷移

```
ready ──[launch]──→ playing ──[全ブロック破壊]──→ clear
                  └──[lives=0]──→ game_over
                  └──[miss, lives>0]──→ ready（ボールリセット）

game_over / clear ──[launch]──→ ready（Game.new()）
```

---

## ハイスコア（scores.rs）

- 上位 10 件を `/workspace/app/breakout_scores.json` に永続保存
- 同スコアは先着優先（新しいほうが下位に挿入）
- 名前は最大 20 文字、空白トリム、空名は拒否

### API

#### `GET /api/breakout/scores`

```json
{
  "scores": [
    { "name": "プレイヤー名", "score": 1500 }
  ],
  "min_score": 980
}
```

`min_score`: 現在の 10 位スコア。これより大きければランクイン可能。10 件未満なら 0。

#### `POST /api/breakout/scores`

リクエスト:
```json
{ "name": "プレイヤー名", "score": 1500 }
```

レスポンス:
```json
{
  "rank":      1,
  "scores":    [...],
  "min_score": 980
}
```

`rank` は 1-based。ランクインしなかった場合は `null`。

---

## フロントエンド（breakout.html）

### 描画

- Canvas 800×600（CSS でスケーリング: `width: 100%; height: auto`）
- `requestAnimationFrame` でサーバーの最新 state を毎フレーム描画
- ブロックは行ごとに色分け（赤→橙→黄→緑→青→紫）、上部ハイライトで立体感
- ボールに放射グラデーション + グローエフェクト
- パドルに上下グラデーション（indigo 系）

### HUD

```
SCORE [現在スコア]   HIGH [1位スコア]   LIVES [❤️❤️❤️]
```

- HIGH は起動時・ゲーム終了時・登録直後に更新

### 音（Web Audio API）

外部クレート不使用、ブラウザの AudioContext のみ。

| イベント | 波形 | 周波数 |
|---------|-----|--------|
| ブロック破壊 | square | 800→400 Hz / 60ms |
| 壁反射 | square | 440→300 Hz / 50ms |
| パドル反射 | square | 520→380 Hz / 70ms |
| ミス | sawtooth | 200→60 Hz / 400ms |

### スタート画面

- ページを開くとランキング + 「ゲームを始める」ボタンを表示
- スコアを `GET /api/breakout/scores` で非同期取得してテーブルを更新

### ゲーム終了オーバーレイ

- GAME OVER / CLEAR 時に HTML オーバーレイを前面表示
- ハイスコアのとき: 「🏆 ハイスコア！」バッジ + 名前入力フォームを表示
- 登録後: 「X位 で登録したよ！」 + ランキング更新（ハイライト付き）
- 「Space / タップでもう一度」でリスタート

### 入力制御

| 手段 | 動作 |
|-----|------|
| キーボード ←→ | `dir` で速度移動（8 px/tick） |
| マウス移動 | `paddle_x` をマウス X に追従 |
| マウスクリック | 発射 / リスタート |
| タッチ スライド | pong 方式のアンカー-デルタ。指を置いた位置を基点に相対移動 |
| タップ（移動 < 10px） | 発射 / リスタート |
| Space | 発射 / リスタート（オーバーレイ表示中ならリスタート兼クローズ） |

### スマホ対策

```css
/* ダブルタップズーム防止 */
viewport: maximum-scale=1, user-scalable=no

/* テキスト選択・コピペマーカー無効化 */
* { user-select: none; -webkit-user-select: none; -webkit-touch-callout: none; }

/* タップハイライト無効化 */
body, canvas { -webkit-tap-highlight-color: transparent; }

/* ページスクロール防止 */
html, body { overflow: hidden; touch-action: none; overscroll-behavior: none; }

/* キャンバスのデフォルトタッチ無効化 */
canvas { touch-action: none; }
```

名前入力 `<input>` のみ `user-select: text` で上書きして文字入力を維持。

---

## モバイル操作の遅延対策（ノウハウ）

WebSocket 経由でサーバーサイドがゲームステートを管理する構成では、
タッチ入力からパドル描画までに **ネットワーク RTT 分の遅延** が乗る。
iPhone 等の実機では数十 ms になり、操作感が著しく悪化する。

### 問題の構造

```
指が動く
  → fingerGameX 更新
  → 次の rAF (最大 16ms 待ち)
  → WS 送信
  → サーバー処理 (最大 16ms 待ち)
  → state 返信
  → 描画                ← 指の動きから 数十 ms〜 のズレ
```

### 対策: 即送信 ＋ クライアント予測描画

**① pointermove で即 WS 送信**（rAF を待たない）

```js
document.addEventListener('pointermove', e => {
  // ...
  fingerGameX = /* 計算 */;
  if (ws.readyState === 1) {
    ws.send(JSON.stringify({ dir: 0, launch: false, paddle_x: fingerGameX }));
  }
}, { passive: false });
```

**② パドルをローカル値で描画**（サーバー返信を待たない）

```js
// state.paddle_x ではなく fingerGameX を優先して描く
drawPaddle(fingerGameX !== null ? fingerGameX : state.paddle_x);
```

### 改善後の流れ

```
指が動く
  → fingerGameX 更新
  → 即 WS 送信           ← rAF 待ちゼロ
  → ローカルで即描画      ← RTT 待ちゼロ
```

パドルの描画は完全にクライアント完結。RTT がいくら高くてもヌルヌル動く。
ボールの当たり判定はサーバーサイドのままで、物理的な整合性は保たれる。

### 適用判断

| 要素 | 予測描画すべきか |
|------|----------------|
| パドル（プレイヤー操作） | **Yes** — 入力と描画を直結する。ズレはほぼ知覚されない |
| ボール | No — サーバー物理に従う。予測するとブロック衝突がズレる |
| スコア・ライフ | No — サーバー確定値を使う |

### 将来ゲームへの転用

- 軸が違っても同じ。Pong なら `fingerGameY → player_y` を予測描画
- 移動系の入力はすべて「即送信 ＋ ローカル描画」が基本方針
- 送信頻度が心配なら `pointermove` に throttle（16ms 間隔）を入れてもよいが、
  実測では WebSocket は十分さばける

---

## 今後の拡張メモ

- [ ] ボールスピードを進行とともに加速
- [ ] 複数ボール・ボールパワーアップ
- [ ] ボールの軌跡エフェクト
- [ ] スコアに日時を記録
- [ ] 難易度選択（ブロック数・速度）
