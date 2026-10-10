# AirRace3D リリース自動化

## 目的
いつもの手順を1本化する。

- Unity build
- `commit -> dev merge -> push`
- `base` の `fetch/pull -> コンテナ再起動 -> 起動確認`
- `dev -> main` の PR 作成・マージ

## スクリプト
`/Volumes/SSD250GBUSB/source/unity/airraceUnity/tools/release_airrace3d_pipeline.sh`

## 使い方

### 1. まず dry-run
```bash
/Volumes/SSD250GBUSB/source/unity/airraceUnity/tools/release_airrace3d_pipeline.sh
```

### 2. 実行
```bash
/Volumes/SSD250GBUSB/source/unity/airraceUnity/tools/release_airrace3d_pipeline.sh --execute
```

### 3. main PR を作らず dev 反映だけで止める
```bash
/Volumes/SSD250GBUSB/source/unity/airraceUnity/tools/release_airrace3d_pipeline.sh --execute --skip-main-pr
```

## 主なオプション
- `--run-note "..."` : 実行メモ（commit / PR 文面に反映）
- `--unity-commit-msg "..."` : Unity 側 commit message 指定
- `--rust-commit-msg "..."` : rust-sandbox 側 commit message 指定
- `--pr-title "..."` / `--pr-body "..."` : `dev -> main` PR 文面指定

## 補足
- リポジトリや base 側パスは環境変数で上書きできる（スクリプト先頭参照）。
- 既存の open PR が `dev -> main` にある場合は、それをそのまま merge する。
