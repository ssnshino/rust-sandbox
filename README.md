# rust-sandbox

## 再開入口

1. `AGENTS.md`
2. `snapshot/CURRENT_SNAPSHOT.md` — 現在地と確認済みcommit
3. `snapshot/LAST_RUN.md` — 直近確認と未実施
4. `snapshot/NEXT_CHAT_PROMPT.txt` — 次セッションへの指示
5. `snapshot/SNAPSHOT_MANIFEST.md` — 今読む資料
6. `docs/head/CURRENT_SPEC.md` — 現行仕様と資料差異

受領原文は `docs/history/`、旧計画は `plan/history/`、旧snapshotは `snapshot/history/` に保持する。通常再開時はhistory全体を読み直さない。

確認済み再開基準: `dev` / `4b7dcc079c311a05f2e6f6429fe00402992aa7e8`（2026-10-10 read-only確認）。開始時にlive branch/HEAD/dirty state/open PRを確認し、この記録だけでlive状態を上書きしない。

重資料・受領保管: [Google Drive](https://drive.google.com/drive/folders/12WX_XviuLWFAsVneXv5WWjoV91RffFKd)。原本保全とGitHub資料統合、実行検証、公開反映は別の完了条件。

## 既存README（本文を維持）

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
- Colony Racer URL: `https://unity-games.wos.ktsys.jp/colony-racer/`
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
- `http://localhost:18093/colony-racer/`
- `http://localhost:18093/api/airrace3d/round-index`
- `http://localhost:18093/api/colony-racer/round-index`

## AirRace3D WebGL 反映

公開物の正規配置は `web/public/airrace3d`。

Unity の Factory prefab、RoundWorld prefab、Material を変更した場合、WebGL ビルドだけでは AssetBundle に反映されない。`AirRace/WebGL/Build Rust Server Package` を実行して、AssetBundle、catalog、WebGL 本体をまとめて生成する。

詳しい手順は [docs/airrace3d_webgl_assetbundle_release.md](docs/airrace3d_webgl_assetbundle_release.md) を参照する。

## Colony Racer WebGL 反映

Colony Racer は AirRace3D から分離した Unity clone として `/Volumes/SSD250GBUSB/source/unity/colony-racer` に置く。公開物の正規配置は `web/public/colony-racer`。

Unity 側は `Colony Racer/WebGL/Build Server Package`、または batchmode の `ColonyRacerWebGLBuilder.BuildServerPackage` を使う。WebGL は `/colony-racer/`、JSON/API は `/api/colony-racer/*` を参照するため、AirRace3D の公開物とは別に更新できる。

## 方針

- `unity-rust-games` というリポジトリ名は維持する。
- WebGL ビルド、AssetBundle、JSON カタログ、軽 API は Node.js/Fastify 側へ移す。
- Rust は将来の `/ws/airrace3d`、room/match/state sync などに集中させる。
- 古い clone 元ドキュメントは整理済み。コードを正として扱う。

## dev 配布の再開手順

Node とビルド済み WebGL の検証、自動配布の設定・停止条件は [dev 配布手順](docs/head/DEV_DELIVERY.md) を参照する。配布は初期 OFF。BASE の配置・接続先・ホスト鍵と利用権限を確認し、設定の承認後に有効化する。Unity Editor のビルドは別工程。
