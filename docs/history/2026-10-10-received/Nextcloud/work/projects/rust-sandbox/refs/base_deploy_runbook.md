# rust-sandbox: base手動デプロイ手順

最終更新: 2026-04-29

## 0. 前提
- `base` 側は直接ファイル編集しない（`git fetch/pull` のみ）。
- 対象リポジトリ:
  - `/home/shino/source/repos/upstream/rust-sandbox`
- 通常反映先:
  - `dev` ブランチ（staging想定）

## 1. baseへログイン
```bash
ssh base
```

## 2. リポジトリへ移動
```bash
cd /home/shino/source/repos/upstream/rust-sandbox
```

## 3. ブランチ更新

### devを反映（通常）
```bash
git switch dev
git fetch origin dev
git pull --ff-only origin dev
```

### mainを反映（必要時のみ）
```bash
git switch main
git fetch origin main
git pull --ff-only origin main
```

## 4. コンテナ再起動
```bash
docker compose -f compose.dev.yaml up -d --build --force-recreate rust-sandbox
```

## 5. 動作確認
```bash
docker compose -f compose.dev.yaml ps
curl -I https://rust-sandbox.wos.ktsys.jp/
```

## 6. 追加確認（任意）
```bash
git rev-parse --abbrev-ref HEAD
git rev-parse HEAD
git status -sb
```

## 7. トラブル時
- `git pull` で問題が出たら、まずローカル変更有無を確認:
```bash
git status -sb
```
- 原則、`base` では手編集しない。必要な修正はローカルで実施してpushし、`base`でpullする。

