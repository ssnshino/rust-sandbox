# Pong 仕様書

`ayano-lab/docker/rust-sandbox` で動く Rust 製 Pong ゲームの仕様まとめ。

作成: 2026-04-15（元 SPEC.md より分離）

---

## 概要

- Rust + axum による WebSocket ゲームサーバー
- サーバーサイドで 60fps のゲームステートを管理し、クライアントは描画・入力・音のみ担当
- 1人用（vs CPU）と 2人用（同一サーバーで対戦）を実装
- テニス方式のポイント計算

---

## アーキテクチャ

### プロキシチェーン

```
ブラウザ (HTTPS)
  → apache2 (wos.ktsys.jp, :443)
  → wos-proxy (nginx-proxy コンテナ, :9080)  ※ WebSocket: mod_proxy_wstunnel 使用
  → rust-sandbox コンテナ (axum, :3000)
```

### ファイル構成

```
rust-sandbox/
├── compose.yaml
├── SPEC_PONG.md         # この文書
├── SPEC_BREAKOUT.md     # Breakout 仕様書
└── app/
    └── src/
        ├── main.rs      # ルーティング（全ゲーム共通）
        ├── game.rs      # ゲームロジック・WebSocket ハンドラ（1P / 2P）
        └── pong.html    # ゲームページ（描画・入力・UI）
```

### ルーティング

```
GET /pong    → pong.html を返す
GET /ws      → WebSocket: 1人用ゲームセッション
GET /ws2p    → WebSocket: 2人用ゲームセッション（AppState で共有）
```

---

## プロトコル

### サーバー → クライアント（毎フレーム）

```json
{
  "ball_x":       400.0,
  "ball_y":       300.0,
  "player_y":     300.0,
  "cpu_y":        300.0,
  "point_player": "30",
  "point_cpu":    "Love",
  "games_player": 1,
  "games_cpu":    0,
  "phase":        "playing",
  "event":        "hit"
}
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `ball_x/y` | f64 | ボール中心座標 |
| `player_y` | f64 | プレイヤーパドル中心 Y |
| `cpu_y` | f64 | CPU パドル中心 Y |
| `point_player/cpu` | str | テニスポイント表示（"Love"/"15"/"30"/"40"/"Adv"） |
| `games_player/cpu` | u32 | 取得ゲーム数 |
| `phase` | str | `"playing"` / `"paused"` / `"game_over"` |
| `event` | str? | `"hit"` / `"score"` / `null` |

### クライアント → サーバー

```json
{ "dir": -1, "pause": false, "restart": false }
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `dir` | i32 | `-1` 上 / `0` 停止 / `1` 下 |
| `pause` | bool | ポーズ切り替え |
| `restart` | bool | ゲームリスタート |

---

## ゲームロジック（game.rs）

### フィールド定数

| 定数 | 値 |
|------|----|
| W × H | 800 × 600 |
| BALL_R | 8 px |
| PADDLE_W × PADDLE_H | 12 × 80 px |
| PLAYER_X | 20 px |
| CPU_X | W − 20 − PADDLE_W = 768 px |

### 速度定数

| 種別 | 値 |
|------|----|
| BALL_SPEED（基準） | 5.0 px/tick |
| BALL_SPEED_FAST（端ゾーン打返し） | × 1.6 |
| SERVE_SLOW | × 0.6 |
| SERVE_NORMAL | × 1.0 |
| SERVE_FAST | × 1.4 |
| PADDLE_SPEED | 6.0 px/tick |
| CPU_SPEED | 3.8 px/tick |

### パドルヒットゾーン

```
┌─────────────┐  ← 端
│  速い (1/4) │  |rel| > 0.5 → BALL_SPEED_FAST
├─────────────┤
│ 通常 (2/4)  │  |rel| ≤ 0.5 → BALL_SPEED
├─────────────┤
│  速い (1/4) │  |rel| > 0.5 → BALL_SPEED_FAST
└─────────────┘  ← 端
```

`rel = (ball_y - paddle_y) / (PADDLE_H / 2)`。打点で `ball_vy` の角度も変化。

### サーブ（毎ラリー後ランダム）

- 縦方向：上下ランダム、角度比率 0.3〜0.9 でランダム
- 速度：SLOW / NORMAL / FAST を等確率でランダム選択
- 乱数：`tick_count` をシードにした疑似乱数（外部クレート不使用）

### CPU AI

- ボールの y 座標 + ランダムオフセットを目標に追尾
- オフセットはプレイヤーが打ち返すたびに更新（−PADDLE_H/2 〜 +PADDLE_H/2）
- これにより CPU が常に中心で返さず、端ゾーンで返すこともある

### テニスポイント方式

```
Love → 15 → 30 → 40 → Deuce → Adv → Game
```

- 40-40 でデュース、その後アドバンテージを経て 2 点差でゲーム取得
- ゲーム取得数は累積記録（セット・マッチは未実装）

---

## フロントエンド（pong.html）

### 描画

- Canvas 800×600（CSS でスケーリング: `width: 100%; height: auto`）
- `requestAnimationFrame` でサーバーの最新 state を毎フレーム描画
- ボールに放射グラデーションのグロー効果
- スコアはフィールド中央に大きく表示

### 音（Web Audio API）

| イベント | 波形 | 周波数 |
|---------|-----|--------|
| ヒット | square | 480→320 Hz / 80ms |
| スコア | sawtooth | 180→60 Hz / 450ms |

サーバーの `event` フィールドで発火。

### スマホ対応

- タッチデバイス（`pointer: coarse`）のみコントローラーを表示
- スライド操作：ドキュメント全体に `pointerdown/move/up` を貼り、アンカー-デルタ方式で Y 移動
  - `pointerdown`: 指の Y 座標とその時点のパドル Y を記憶
  - `pointermove`: 指の移動量（スクリーン px）→ ゲーム座標に変換してパドルに加算
  - `pointerup/cancel`: アンカーリセット

### スマホ対策

```css
/* ダブルタップズーム防止 */
viewport: maximum-scale=1, user-scalable=no

/* テキスト選択・コピペマーカー無効化 */
body { user-select: none; -webkit-user-select: none; -webkit-touch-callout: none; }
canvas { touch-action: none; }

/* タップハイライト無効化 */
body { -webkit-tap-highlight-color: transparent; }

/* ボタン: タップ応答を残しながらズーム防止 */
.btn { touch-action: manipulation; -webkit-tap-highlight-color: transparent; }
```

### コントローラーレイアウト

```
[  ▲  ]   [SELECT]   [  X  ]
[◀][  ][▶]  [ ⏸ ]  [Y][  ][A]
[  ▼  ]   [START ]   [  B  ]
```

- **▲▼**：パドル操作
- **⏸ PAUSE**：ポーズ／再開
- **START**：New Game
- **SELECT / XYAB**：将来拡張用（押下で光る）

---

## 今後の拡張メモ

- [ ] 2人対戦（ルーム・マッチング機構）
- [ ] セット・マッチ方式
- [ ] ハイスコア（ラリー数・最長戦績など）
- [ ] XYAB ボタンへの機能割り当て（パワーアップ等）
- [ ] `cargo run` 起動時のクレートキャッシュ永続化（volume 追加）
- [ ] リリースビルド対応（`--release` フラグ）
