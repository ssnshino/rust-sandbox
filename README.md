# rust-sandbox

`ayano-lab/docker/rust-sandbox` は、Rust の実験用コンテナ。

## 構成

- `Dockerfile`: `rust:bookworm` ベース
- `compose.yaml`: axum サーバーを `cargo run` で起動
- `app/`: Rust プロジェクト本体（`Cargo.toml` + `src/`）

## 使い方

```bash
cd /home/shino/work/projects/ayano-lab/docker/rust-sandbox
docker compose up --build
```

ブラウザで `http://localhost:18081` にアクセス。

## ポート

- ホスト: 18081 → コンテナ: 3000

## 注意

- 初回ビルドは依存クレートのダウンロード＋コンパイルで時間がかかる
- `app/` を volume mount しているので、コードを編集したら `docker compose restart` で反映
