# rust-sandbox 仕様書 インデックス

`ssnshino/rust-sandbox` で動く Rust 製 Web ゲーム実験の仕様まとめ。

## ドキュメント

- [dev-workflow.md](./dev-workflow.md) — 開発・デプロイ手順

## ゲーム一覧

| ゲーム | 仕様書 | エンドポイント |
|-------|--------|--------------|
| Pong（vs CPU / 2P） | [pong.md](./pong.md) | `/pong`, `/ws`, `/ws2p` |
| Breakout | [breakout.md](./breakout.md) | `/breakout`, `/ws/breakout`, `/api/breakout/scores` |
| あやかダンジョン × 平安京エイリアン | [dungeon.md](./dungeon.md) | `/dungeon`, `/ws/dungeon`, `/api/dungeon/scores` |

## 共通アーキテクチャ

### デプロイ構成

```
ブラウザ (HTTPS)
  → nginx-proxy + acme-companion (Let's Encrypt 自動取得)
  → games-rust-sandbox コンテナ (axum, :3000)
  ※ ktsys-pubserver: games.lab.ktsys.jp
  ※ base 開発機: rust-sandbox.wos.ktsys.jp (compose.dev.yaml)
```

### 開発フロー

```
dev ブランチで開発
  → compose.dev.yaml で base 機動作確認
  → PR → main マージ
  → GitHub Actions (deploy.yml) が ktsys-pubserver に SSH デプロイ
```

### 技術スタック

```
Cargo.toml 依存: axum 0.7 (ws feature), tokio (full), serde, serde_json, rand 0.8
```

### ファイル構成

```
rust-sandbox/
├── Dockerfile              # rust:bookworm ベース
├── compose.yaml            # 本番用（container_network, VIRTUAL_HOST=games.lab.ktsys.jp）
├── compose.dev.yaml        # 開発用（wos-proxy-network, port 18081:3000）
├── .github/workflows/
│   └── deploy.yml          # main push → ktsys-pubserver 自動デプロイ
├── docs/
│   ├── index.md            # この文書（インデックス）
│   ├── pong.md             # Pong 仕様書
│   ├── breakout.md         # Breakout 仕様書
│   └── dungeon.md          # あやかダンジョン仕様書
└── app/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── game.rs           # Pong
        ├── breakout.rs       # Breakout
        ├── dungeon.rs        # あやかダンジョン
        ├── dungeon_gen.rs    # 迷路生成
        ├── scores.rs         # ハイスコア共通
        ├── index.html
        ├── pong.html
        ├── breakout.html
        └── dungeon.html
```
