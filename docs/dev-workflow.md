# 開発ワークフロー

## 基本の流れ

```
base で修正
  → rust-sandbox.wos.ktsys.jp で動作確認
  → git push origin dev
  → GitHub で PR（dev → main）
  → マージ → GitHub Actions が games.lab.ktsys.jp に自動デプロイ
```

## 環境

| 環境 | サーバー | URL | compose ファイル | ブランチ |
|------|---------|-----|----------------|---------|
| 開発 | base | `rust-sandbox.wos.ktsys.jp` | `compose.dev.yaml` | `dev` |
| 本番 | ktsys-pubserver | `games.lab.ktsys.jp` | `compose.yaml` | `main` |

## 手順詳細

### 1. base で修正する

```bash
# base サーバーにログイン
ssh base

# リポジトリに移動（dev ブランチ）
cd ~/work/upstream/rust-sandbox
git checkout dev
```

コードを編集したら、コンテナをリビルドして反映：

```bash
docker compose -f compose.dev.yaml up --build -d
```

### 2. 動作確認

ブラウザで `http://rust-sandbox.wos.ktsys.jp` にアクセスして確認。

問題があれば 1. に戻って修正。

### 3. dev ブランチに push

```bash
git add <変更ファイル>
git commit -m "fix: ..."
git push origin dev
```

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

- Mac のローカルクローンで直接編集して `dev:main` に force-push → **本番に未検証コードが飛ぶ**
- base で確認せずに push → **同上**

## ktsys-pubserver で手動操作が必要な場合

通常は GitHub Actions で自動デプロイされるが、緊急時や初期セットアップ時：

```bash
ssh ktsys-pubserver
cd ~/work/upstream/rust-sandbox
git pull origin main
docker compose up --build -d
```
