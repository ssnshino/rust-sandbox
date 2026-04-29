# 開発ワークフロー

## 基本の流れ

```
ローカルで修正
  → git push origin dev
  → GitHub Actions が base (rust-sandbox.wos.ktsys.jp) に自動デプロイ
  → base で動作確認
  → GitHub で PR（dev → main）
  → マージ → GitHub Actions が games.lab.ktsys.jp に自動デプロイ
```

## 環境

| 環境 | サーバー | URL | compose ファイル | ブランチ |
|------|---------|-----|----------------|---------|
| 開発 | base | `rust-sandbox.wos.ktsys.jp` | `compose.dev.yaml` | `dev` |
| 本番 | ktsys-pubserver | `games.lab.ktsys.jp` | `compose.yaml` | `main` |

## 手順詳細

### 1. ローカルで修正する

```bash
# ローカル
cd ~/source/repos/upstream/rust-sandbox
git checkout dev
```

### 2. dev ブランチに push

```bash
git add <変更ファイル>
git commit -m "fix: ..."
git push origin dev
```

push 後、`deploy-dev.yml` が動いて base 側に `compose.dev.yaml` で自動デプロイされる。

事前に GitHub Secrets を設定しておく:

- `DEPLOY_DEV_HOST`
- `DEPLOY_DEV_PORT`
- `DEPLOY_SSH_KEY`

### 3. base で動作確認

ブラウザで `http://rust-sandbox.wos.ktsys.jp` にアクセスして確認。

問題があれば 1. に戻って修正し、再度 push。

### 4. PR を作成 → main にマージ

```bash
# GitHub CLI を使う場合
gh pr create --base main --title "..." --body "..."
```

または GitHub の Web UI で PR を作成。

レビュー・確認後にマージすると GitHub Actions (`deploy.yml`) が起動し、
ktsys-pubserver 上で `git pull && docker compose up --build -d` が自動実行される。

### 5. 本番確認

`https://games.lab.ktsys.jp` で動作確認。

---

## NG パターン

- `dev` を通さず `main` に直接 push → **本番へ即反映される**
- force-push で履歴を壊す → **ロールバックが難しくなる**

## ktsys-pubserver で手動操作が必要な場合

通常は GitHub Actions で自動デプロイされるが、緊急時や初期セットアップ時：

```bash
ssh ktsys-pubserver
cd ~/source/repos/upstream/rust-sandbox
git pull origin main
docker compose up --build -d
```
