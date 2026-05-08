# unity-rust-games

Unity WebGL ゲーム配信と、将来の Rust realtime/game-engine 実験をまとめるリポジトリ。

現状は Rust/axum の既存コンテナが Web 配信、軽量 API、旧ゲーム実験をまとめて担当している。今後は AirRace3D の WebGL/AssetBundle/JSON API 配信を Node.js Web コンテナへ分離し、Rust はマルチユーザー同期やゲームエンジン用途へ寄せていく。

## 現在の主要対象

- Unity クライアント: `/Volumes/SSD250GBUSB/source/unity/airraceUnity`
- サーバリポジトリ: このリポジトリ
- 現行 Rust app: `app/`
- Node Web コンテナ計画: [docs/airrace3d_node_web_container_plan.md](docs/airrace3d_node_web_container_plan.md)
- WebGL / AssetBundle 反映手順: [docs/airrace3d_webgl_assetbundle_release.md](docs/airrace3d_webgl_assetbundle_release.md)
- 正規ポータルURL: `https://unity-games.wos.ktsys.jp/`
- AirRace3D URL: `https://unity-games.wos.ktsys.jp/airrace3d/`
- 旧確認用alias: `https://airrace-web.wos.ktsys.jp/airrace3d/`

## 開発環境

```bash
docker compose -f compose.dev.yaml up --build -d
```

現行の compose は Rust コンテナ中心。Node Web コンテナは段階的に追加する。

Node Web コンテナだけ起動する場合:

```bash
docker compose -f compose.dev.yaml up --build airrace-web
```

ローカル Node で直接確認する場合:

```bash
cd web
npm install
npm run dev
```

確認 URL:

- `http://localhost:18093/healthz`
- `http://localhost:18093/`
- `http://localhost:18093/airrace3d/`
- `http://localhost:18093/api/airrace3d/round-index`

## AirRace3D WebGL 反映

公開物の正規配置は `web/public/airrace3d`。

Unity の Factory prefab、RoundWorld prefab、Material を変更した場合、WebGL ビルドだけでは AssetBundle に反映されない。`AirRace/WebGL/Build Rust Server Package` を実行して、AssetBundle、catalog、WebGL 本体をまとめて生成する。

詳しい手順は [docs/airrace3d_webgl_assetbundle_release.md](docs/airrace3d_webgl_assetbundle_release.md) を参照する。

## 方針

- `unity-rust-games` というリポジトリ名は維持する。
- WebGL ビルド、AssetBundle、JSON カタログ、軽 API は Node.js/Fastify 側へ移す。
- Rust は将来の `/ws/airrace3d`、room/match/state sync などに集中させる。
- 古い clone 元ドキュメントは整理済み。コードを正として扱う。
