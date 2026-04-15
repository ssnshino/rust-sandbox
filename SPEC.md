# rust-sandbox 仕様書 インデックス

`ayano-lab/docker/rust-sandbox` で動く Rust 製 Web ゲーム実験の仕様まとめ。

## ゲーム一覧

| ゲーム | 仕様書 | エンドポイント |
|-------|--------|--------------|
| Pong（vs CPU / 2P） | [SPEC_PONG.md](./SPEC_PONG.md) | `/pong`, `/ws`, `/ws2p` |
| Breakout | [SPEC_BREAKOUT.md](./SPEC_BREAKOUT.md) | `/breakout`, `/ws/breakout`, `/api/breakout/scores` |

## 共通アーキテクチャ

### プロキシチェーン

```
ブラウザ (HTTPS)
  → apache2 (wos.ktsys.jp, :443)
  → wos-proxy (nginx-proxy コンテナ, :9080)  ※ WebSocket: mod_proxy_wstunnel 使用
  → rust-sandbox コンテナ (axum, :3000)
```

### 技術スタック

```
Cargo.toml 依存: axum 0.7 (ws feature), tokio (full), serde, serde_json
```

### ファイル構成

```
rust-sandbox/
├── Dockerfile          # rust:bookworm ベース、WORKDIR のみ設定
├── compose.yaml        # wos-proxy-network 参加、port 18081:3000、app/ を volume mount
├── SPEC.md             # この文書（インデックス）
├── SPEC_PONG.md        # Pong 仕様書
├── SPEC_BREAKOUT.md    # Breakout 仕様書
└── app/
    ├── Cargo.toml
    ├── breakout_scores.json  # Breakout ハイスコア永続ファイル
    └── src/
        ├── main.rs       # axum ルーティング（全ゲーム共通）
        ├── game.rs       # Pong ゲームロジック・WebSocket ハンドラ
        ├── breakout.rs   # Breakout ゲームロジック・WebSocket ハンドラ
        ├── scores.rs     # Breakout ハイスコア管理
        ├── index.html    # トップメニュー
        ├── pong.html     # Pong ゲームページ
        └── breakout.html # Breakout ゲームページ
```

---

## 開発予定: ayaka dungeon × 平安京エイリアン 融合版

| 項目 | 内容 |
|------|------|
| 企画書 | `projects/ayaka-dungeon/plan/03_平安京融合版_企画書_20260415.md` |
| 実装予定ファイル | `app/src/dungeon.rs`, `dungeon_gen.rs`, `dungeon.html` |
| 予定エンドポイント | `/dungeon`, `/ws/dungeon` |

主な新要素: グリッド迷路生成（再帰バックトラッカー）、エイリアン BFS AI、穴メカニクス、フォグオブウォー、HP 制、あやか GM プリセットメッセージ
