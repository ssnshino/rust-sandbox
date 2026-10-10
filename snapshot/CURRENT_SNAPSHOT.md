# SNAPSHOT（資料証拠に基づく再開点）

- 配布・ビルド側の再開先候補は ssnshino/rust-sandbox の dev。親調査で確認した commit は 4b7dcc079c311a05f2e6f6429fe00402992aa7e8。
- ローカル名 unity-rust-games と GitHub名 rust-sandbox は対応している。旧ローカル clone rust-sandbox の HEAD 58e6f73 は別の過去状態で、最新配布状態と混同しない。
- unity-rust-games2 main a334324fc1cc775b44a662457b53ae9b09fc7b7c は AIRRACE3D2 staging。現行Unityソースの代替にしない。
- 既存ゲームコンテナを流用し、入口の大飯店統合は後工程。ここでは資料の引継ぎのみ。
- 旧仕様書の main push 自動デプロイや手動運用手順は履歴情報。今回実行しない。

受領ZIPの文書整合性は検証済み。過去ログの成功記録は今回の動作検証ではない。

## 2026-10-10 コード照合追記

GitHub dev `4b7dcc079c311a05f2e6f6429fe00402992aa7e8` を確認。資料との主要差異はdocs/head/CURRENT_SPEC.mdへ記録。コード・build・deployの変更なし。

## Draft PR作成直前のlive確認

2026-10-10: devは654f476c740f5e87778dce14ed301f0c2e7e817fへ前進。4b7dcc0からの差分はcompose.dev.yamlの起動をnpm run build && npm run startにする変更。この文書PRは最新devを親にし、既存修正を保持する。4b7dcc0は資料・ゲームコード照合時点の記録として残す。
