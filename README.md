# rust-sandbox

Rust + axum で動く WebSocket ゲーム実験コンテナ。

## ゲーム

| ゲーム | URL |
|-------|-----|
| Pong（vs CPU / 2P） | `/pong` |
| Breakout | `/breakout` |
| あやかダンジョン | `/dungeon` |

## 仕様書

`docs/` フォルダを参照。

- [docs/index.md](docs/index.md) — 全体インデックス・共通アーキテクチャ
- [docs/pong.md](docs/pong.md) — Pong 仕様書
- [docs/breakout.md](docs/breakout.md) — Breakout 仕様書
- [docs/dungeon.md](docs/dungeon.md) — あやかダンジョン仕様書

## 環境

| 環境 | compose ファイル | URL |
|------|----------------|-----|
| 本番 (ktsys-pubserver) | `compose.yaml` | https://games.lab.ktsys.jp |
| 開発 (base) | `compose.dev.yaml` | http://rust-sandbox.wos.ktsys.jp |

## 開発フロー

```bash
# dev ブランチで開発・動作確認
git checkout dev
docker compose -f compose.dev.yaml up --build -d

# main にマージ → GitHub Actions が自動デプロイ
gh pr create --base main
```

## 構成

```
rust-sandbox/
├── Dockerfile
├── compose.yaml          # 本番用
├── compose.dev.yaml      # 開発用
├── .github/workflows/
│   └── deploy.yml        # main push → 自動デプロイ
├── docs/                 # 仕様書
└── app/                  # Rust プロジェクト
```
