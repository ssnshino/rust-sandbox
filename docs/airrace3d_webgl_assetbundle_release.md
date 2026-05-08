# AirRace3D WebGL / AssetBundle Release

作成日: 2026-05-04 JST

## 目的

AirRace3D の Unity WebGL 本体、`StreamingAssets`、AssetBundle、JSON catalog を Node Web コンテナの公開領域へ反映するための手順。

現在の正規公開先は `web/public/airrace3d`。

```text
unity-rust-games/
  web/public/airrace3d/
    index.html
    Build/
    TemplateData/
    StreamingAssets/
      AirRace/
        aircraft_prefab_catalog.json
        round_world_catalog.json
        world_object_catalog.json
        Bundles/
```

旧 Rust 側の `app/src/airrace3d` は参照先として残っていても、今後の Web 公開の正は `web/public/airrace3d` とする。

## 重要な前提

Unity の WebGL ビルドだけでは、Factory prefab の変更は AssetBundle に反映されない。

以下を変更した場合は AssetBundle と catalog の再生成が必要。

- `Assets/AirRace/Factory/AircraftPrefabs/*`
- `Assets/AirRace/Factory/RoundWorldPrefabs/*`
- `Assets/AirRace/Factory/WorldPrefabs/*`
- `Assets/AirRace/Factory/Materials/*`
- `AirRaceAircraftPrefabAuthoring`
- `AirRaceRoundWorldAuthoring`
- `AirRaceWorldObjectAuthoring`

## Unity Editor から反映する

AssetBundle と catalog だけを更新する場合:

```text
AirRace/Bundles/Build Catalogs + Bundles (WebGL) + Sync Rust
```

WebGL 本体も含めて公開物を全部更新する場合:

```text
AirRace/WebGL/Build Rust Server Package
```

`Build Rust Server Package` は内部で以下をまとめて実行する。

1. Factory prefab/material から WebGL 用 AssetBundle を生成
2. `aircraft_prefab_catalog.json`、`round_world_catalog.json`、`world_object_catalog.json` を生成
3. `web/public/airrace3d/StreamingAssets/AirRace` へ同期
4. `web/public/airrace3d` へ WebGL build を出力

## コマンドで WebGL 一括ビルドする

Unity Editor を閉じてから実行する。同じ Unity project を Editor と batchmode で同時に開くことはできない。

```bash
AIRRACE_WEBGL_BUILD_PATH=/Users/shino/source/repos/upstream/unity-rust-games/web/public/airrace3d \
AIRRACE_STREAMING_SYNC_ROOT=/Users/shino/source/repos/upstream/unity-rust-games/web/public/airrace3d/StreamingAssets/AirRace \
/Applications/Unity/Hub/Editor/6000.4.3f1/Unity.app/Contents/MacOS/Unity \
  -batchmode \
  -quit \
  -projectPath /Volumes/SSD250GBUSB/source/unity/airraceUnity \
  -executeMethod AirRaceWebGLBuilder.BuildRustServerPackage \
  -buildTarget WebGL \
  -logFile /tmp/airrace3d_webgl_build.log
```

成功確認:

```bash
rg "Build Finished, Result: Success." /tmp/airrace3d_webgl_build.log
```

## 生成物を確認する

WebGL HTML が存在すること:

```bash
test -f /Users/shino/source/repos/upstream/unity-rust-games/web/public/airrace3d/index.html
```

catalog version を確認する:

```bash
cd /Users/shino/source/repos/upstream/unity-rust-games
node -e "const fs=require('fs'); for (const f of ['aircraft_prefab_catalog.json','round_world_catalog.json','world_object_catalog.json']) { const j=JSON.parse(fs.readFileSync('web/public/airrace3d/StreamingAssets/AirRace/'+f,'utf8')); console.log(f, j.version); }"
```

`index.html` が実在する Build ファイルを指しているか確認する:

```bash
cd /Users/shino/source/repos/upstream/unity-rust-games
node -e "const fs=require('fs'); const html=fs.readFileSync('web/public/airrace3d/index.html','utf8'); for (const m of html.matchAll(/Build\\/([^\\\"]+)/g)) { const p='web/public/airrace3d/Build/'+m[1]; console.log(fs.existsSync(p)?'OK':'MISSING',m[1]); }"
```

## ローカル Node で確認する

```bash
cd /Users/shino/source/repos/upstream/unity-rust-games/web
npm run build
PORT=18193 \
HOST=127.0.0.1 \
AIRRACE3D_ROOT=/Users/shino/source/repos/upstream/unity-rust-games/web/public/airrace3d \
AIRRACE_DATA_ROOT=/Users/shino/source/repos/upstream/unity-rust-games/app/src/airrace \
node dist/server.js
```

別ターミナルで確認:

```bash
curl -fsS http://127.0.0.1:18193/healthz
curl -fsS http://127.0.0.1:18193/api/airrace3d/aircraft-prefab-catalog
curl -fsSI http://127.0.0.1:18193/airrace3d/
```

## dev へ反映して base で公開する

`unity-rust-games` 側で生成物を commit/push する。

```bash
cd /Users/shino/source/repos/upstream/unity-rust-games
git status --short
git add web/public/airrace3d
git commit -m "chore(airrace3d): rebuild WebGL"
git push origin dev
```

base サーバで pull して Node Web コンテナを再作成する。

```bash
ssh base 'cd /home/shino/source/repos/upstream/unity-rust-games && git fetch origin --prune && git pull --ff-only origin dev && docker compose -f compose.dev.yaml up -d --build airrace-web'
```

公開確認:

```bash
curl -fsS https://unity-games.wos.ktsys.jp/healthz
curl -fsSI https://unity-games.wos.ktsys.jp/
curl -fsS https://unity-games.wos.ktsys.jp/api/airrace3d/aircraft-prefab-catalog
curl -fsSI https://unity-games.wos.ktsys.jp/airrace3d/
```

ブラウザ確認時は cache を避けるため、catalog version などを query に付ける。

```text
https://unity-games.wos.ktsys.jp/airrace3d/?v=YYYYMMDDHHMMSS
```

## URL の注意

正規のUnityゲームポータル:

```text
https://unity-games.wos.ktsys.jp/
https://unity-games.wos.ktsys.jp/airrace3d/
```

旧確認用alias:

```text
https://airrace-web.wos.ktsys.jp/airrace3d/
```

古い Rust コンテナ:

```text
https://unity-rust-games.wos.ktsys.jp/airrace3d/
```

`unity-rust-games.wos.ktsys.jp` は旧 WebGL や旧 catalog を返す可能性がある。AirRace3D の最新確認は `unity-games.wos.ktsys.jp` を使う。

## よくある症状

Factory で Round1 を直したのに WebGL で変わらない:

- AssetBundle を再生成していない可能性が高い。
- `AirRace/WebGL/Build Rust Server Package` を実行する。

WebGL が古い loader を読んでいる:

- `index.html` の `loaderUrl` が `web/public/airrace3d/Build` の実ファイルと一致しているか確認する。
- ブラウザ cache を避けて `?v=...` 付きで開く。

スカイランサーなど初期機体だけ表示されない:

- WebGL 本体が古い可能性がある。
- `airrace-web` 側の loader hash と最新 commit の生成物を確認する。
