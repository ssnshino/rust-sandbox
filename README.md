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
# ローカルで dev ブランチを開発
git checkout dev
git add <変更ファイル>
git commit -m "..."
git push origin dev

# dev -> main で PR 作成・マージ
gh pr create --base main
```

`dev` への push で GitHub Actions が base 環境 (`rust-sandbox.wos.ktsys.jp`) を自動更新し、`main` への merge で本番 (`games.lab.ktsys.jp`) を自動更新する。

必要な GitHub Secrets:

- `DEPLOY_DEV_HOST` (base のホスト)
- `DEPLOY_HOST` (ktsys-pubserver のホスト)
- `DEPLOY_SSH_KEY` (shino ユーザーで接続可能な秘密鍵)


## 構成

```
rust-sandbox/
├── Dockerfile
├── compose.yaml          # 本番用
├── compose.dev.yaml      # 開発用
├── .github/workflows/
│   ├── deploy-dev.yml    # dev push → base 自動デプロイ
│   └── deploy.yml        # main push → 本番自動デプロイ
├── docs/                 # 仕様書
└── app/                  # Rust プロジェクト
```
