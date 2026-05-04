# AirRace3D Node Web Container Plan

作成日: 2026-05-04 JST

## 目的

`unity-rust-games` リポジトリを維持したまま、AirRace3D の WebGL 配信と軽量 API を Rust から分離する。

現状の Rust/axum サーバは、将来のマルチユーザー同期やゲームエンジン用途には向いている。一方で、WebGL ビルド、AssetBundle、コース JSON、カタログ JSON の配信まで Rust に含めると、軽微な変更でも Rust の再ビルドやコンテナ再起動が必要になり、開発テンポが落ちる。

そこで当面は Node.js Web コンテナを追加し、頻繁に変わる配信物と軽 API を担当させる。Rust は段階的に realtime/game-engine 専用へ寄せる。

## 方針

- リポジトリ名 `unity-rust-games` は維持する。
- 新規に `web/` ディレクトリを追加し、Node.js/Fastify ベースの Web コンテナを置く。
- WebGL と AssetBundle は `web/public/` 以下から静的配信する。
- API は Rust 互換の URL とレスポンスを先に再現する。
- Unity クライアント側は、まず URL 切り替えだけで動く状態を目指す。
- Rust 側はすぐ消さず、既存ゲームと将来の WebSocket/room/match/state sync 用に残す。

## 目標構成

```text
unity-rust-games/
  compose.yaml
  compose.dev.yaml
  Dockerfile                 # 既存 Rust 用
  app/                       # 既存 Rust app
  web/
    Dockerfile
    package.json
    tsconfig.json
    src/
      server.ts
      routes/
        airrace3d.ts
      lib/
        jsonFile.ts
        safePath.ts
    public/
      airrace3d/
        index.html
        Build/
        TemplateData/
        StreamingAssets/
          AirRace/
            Bundles/
    data/
      airrace3d/
        airrace_rounds.json
        aircraft_catalog.json
        field_catalog.json
        world_object_catalog.json
        round_world_catalog.json
        aircraft_prefab_catalog.json
```

## 役割分担

### Node Web コンテナ

- `/airrace3d/` の WebGL 起動 HTML 配信
- `/airrace3d/Build/*` の WebGL ビルド配信
- `/airrace3d/TemplateData/*` の配信
- `/airrace3d/StreamingAssets/*` の AssetBundle/JSON 配信
- `/api/airrace3d/*` の軽量 JSON API
- 将来の管理 UI、コース編集 UI、カタログ検証 API

### Rust コンテナ

- 既存の Pong/Breakout/Dungeon/Truckers/AirRace 旧 Web 実験
- 将来の `/ws/airrace3d` realtime server
- ルーム管理、マッチング、状態同期
- ゴースト/リプレイ/ランキングなど、状態管理や並行処理が重い処理

## Node API 互換対象

最初の実装では、Unity が現在使っている API をそのまま再現する。

```text
GET /api/airrace3d/round-index
GET /api/airrace3d/course/:round
GET /api/airrace3d/field-catalog
GET /api/airrace3d/world-object-catalog
GET /api/airrace3d/round-world-catalog
GET /api/airrace3d/aircraft-catalog
GET /api/airrace3d/aircraft-prefab-catalog
GET /airrace3d/
GET /airrace3d/*path
GET /airrace3d/StreamingAssets/*path
```

レスポンス方針:

- API は `Cache-Control: no-store`
- 静的配信は当面 `Cache-Control: public, max-age=300`
- gzip 済み Unity build ファイルは `Content-Encoding: gzip` を付ける
- AssetBundle は `application/octet-stream`
- JSON は `application/json; charset=utf-8`
- パストラバーサル対策として、静的配信の `*path` は normal component のみ許可する

## 段階計画

### Phase 0: 現状固定と配信物の棚卸し

目的:

- Rust に埋め込まれている AirRace3D 配信責務を洗い出す。
- Node 側へ移すファイルと API を確定する。

作業:

- `app/src/main.rs` の `/airrace3d` と `/api/airrace3d` ルートを一覧化する。
- `app/src/airrace/airrace_rounds.json` を Node 側 `web/data/airrace3d/airrace_rounds.json` へ移す候補にする。
- `app/src/airrace3d/` 配下の WebGL build と StreamingAssets を Node 側 `web/public/airrace3d/` へ移す候補にする。
- 現在欠けているカタログ JSON があれば Unity build pipeline 側から再生成する。

完了条件:

- Node 側で再現すべき URL とファイル配置が確定している。
- Unity 側の接続先変更方針が決まっている。

### Phase 1: Node/Fastify 最小 Web コンテナ追加

目的:

- Node Web コンテナを compose に追加し、単独で起動できるようにする。

作業:

- `web/package.json` を追加する。
- Fastify + TypeScript を導入する。
- `web/src/server.ts` で health check と静的配信を実装する。
- `web/Dockerfile` を追加する。
- `compose.dev.yaml` に `airrace-web` サービスを追加する。
- 開発環境では Rust と別ポートで起動する。

想定 URL:

```text
http://localhost:18092/airrace3d/
http://localhost:18092/api/airrace3d/round-index
```

完了条件:

- `docker compose -f compose.dev.yaml up airrace-web` で Node コンテナが起動する。
- `/healthz` が 200 を返す。
- `/airrace3d/` が配信される。

### Phase 2: Rust 互換 API の再現

目的:

- Unity クライアントが Node API だけで起動準備できる状態にする。

作業:

- `round-index` を `airrace_rounds.json` から生成する。
- `course/:round` を `airrace_rounds.json` から返す。
- `field-catalog` を JSON ファイルまたは既定値から返す。
- `world-object-catalog` を JSON ファイルから返す。
- `round-world-catalog` を JSON ファイルから返す。
- `aircraft-catalog` を JSON ファイルから返す。
- `aircraft-prefab-catalog` を JSON ファイルから返す。
- JSON 読み込み時に最低限のエラー応答を返す。

完了条件:

- Rust 側の `/api/airrace3d/*` と Node 側の `/api/airrace3d/*` が同じ形で返る。
- Unity Editor から Node 側 URL を向けてもタイトルのカタログ取得が通る。

### Phase 3: WebGL/AssetBundle 配信移管

目的:

- WebGL build と StreamingAssets を Node Web コンテナの公開領域から配信する。

作業:

- `web/public/airrace3d/Build` に WebGL build を配置する。
- `web/public/airrace3d/TemplateData` にテンプレート資産を配置する。
- `web/public/airrace3d/StreamingAssets/AirRace/Bundles` に AssetBundle を配置する。
- Unity build pipeline の sync 先を `web/public/airrace3d/StreamingAssets/AirRace` に切り替える。
- WebGL build の出力先またはコピー先を `web/public/airrace3d` に寄せる。

完了条件:

- Node 側 `/airrace3d/` だけで Unity WebGL が起動する。
- 機体 AssetBundle と round world AssetBundle が Node 側からロードされる。

### Phase 4: 開発導線の簡略化

目的:

- 日々の変更を Rust 再ビルドなしで回せる状態にする。

作業:

- JSON/API 変更時は Node コンテナだけ再起動、または dev mode hot reload にする。
- WebGL build/AssetBundle 更新時はファイルコピーだけで反映する。
- リリーススクリプトを Node Web コンテナ構成に合わせて更新する。
- README に新しい開発手順を書く。

完了条件:

- コース JSON、カタログ JSON、WebGL build 差し替えで Rust build が不要になる。
- Unity の通常確認が Node Web コンテナだけで完結する。

### Phase 5: Rust realtime 化の準備

目的:

- Rust を Web 配信から切り離し、将来のマルチユーザー基盤へ寄せる。

作業:

- Rust の `/api/airrace3d/*` と `/airrace3d/*` を deprecated 扱いにする。
- 新規 `/ws/airrace3d` のプロトコル草案を切る。
- ルーム、プレイヤー、tick、snapshot、input command のモデルを設計する。
- Node Web 側から Rust realtime URL を設定値として渡す仕組みを作る。

完了条件:

- Web 配信は Node、realtime は Rust という責務分離がコード上でも明確になる。

## 推奨技術

- Runtime: Node.js 22 LTS 系
- Framework: Fastify
- Language: TypeScript
- Static files: `@fastify/static`
- Dev reload: `tsx watch` または `nodemon`
- Validation: 必要になった時点で JSON Schema + Ajv

## 判断メモ

PHP/Slim でも今回の軽 API と静的配信は実現できる。ただし、今後コース編集 UI、カタログ検証、WebSocket デバッグ画面、管理ツールを増やす可能性があるため、Node.js/TypeScript の方が育てやすい。

Rust は非同期処理、WebSocket、低遅延同期、状態管理で引き続き強い。捨てるのではなく、頻繁に変わる配信物から切り離して、将来のゲームエンジン/realtime server に集中させる。
