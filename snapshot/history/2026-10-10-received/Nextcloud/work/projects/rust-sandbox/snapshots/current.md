# rust-sandbox Snapshot

## 2026-04-25 JST

### 対象

- プロジェクト: rust-sandbox
- アプリ: ベクター・エアレース
- 作業場所: /home/shino/work/upstream/rust-sandbox
- ブランチ: main
- PR: #42
- main commit: `ab5c5fcce9e2092972ac2323f687572fa720beca`

### 現在地

Air Race は、単一HTMLの試作段階を抜けて `app/src/airrace/` 配下へ分割された。
タイトル画面とリザルト画面はスクロール可能な HTML ページになり、World Grand Prix の 10 ラウンド、本戦ポイント制、機体選択、ゴースト/CPU 混在レース、ミニマップ、参加者一覧、総合順位表示まで入っている。

### 今回までに反映した内容

- `ROUND 1-2` を導入ラウンド、`ROUND 3-10` を各国 World Grand Prix 本戦へ再構成した
- 自機を 5 機選択制にし、機体ごとに見た目と乗り味を分けた
- `FIELD TEST 8 / 12 / 16 / 24` を追加し、多機数レースの試験ができるようにした
- ラウンド定義を `airrace_rounds.json` から取得する方式へ切り替えた
- タイトル画面を通常HTML寄りに再構成し、名前入力と開始ROUND選択を追加した
- リザルト画面をスクロール化し、上位機体表示、1行1人の順位表、GP総合順位表示を追加した
- ミニマップ、参加者一覧、DAMAGE / GP / TIME 表示、メッセージカットインなど HUD を整理した
- ゴースト保存を再開し、同名重複除外、DNFゴースト保存、CPU補完を入れた
- `airrace` 関連ファイルを `app/src/airrace/` へ整理した

### 資料

- 計画: /home/shino/work/projects/rust-sandbox/plan/2026-04-24_01_airrace再構成計画.md
- 作業ログ: /home/shino/work/projects/rust-sandbox/working-logs/worklog_20260425.md
- 仕様書: /home/shino/work/upstream/rust-sandbox/docs/airrace.md

### 検証

- `node --check` で主要JSの構文確認
- `docker compose -f compose.dev.yaml restart rust-sandbox`
- `/airrace` `/airrace.js` `/airrace/js/render.js` `/api/airrace/round/3` などの配信内容確認

### 次に見るところ

- ROUND 6 付近での突然タイトル戻りの原因特定
- 誘導ガイド矢印の DOM 表示安定化
- 順位判定とゴースト精度の継続調整
- 各国コース演出の追加強化
