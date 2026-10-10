# AirRace Unity 3D スナップショット

作成日時: 2026-04-26 23:51:37 JST

## 状態

- Unity project: `/Volumes/SSD250GBUSB/source/unity/airraceUnity`
- Unity source commit: `93066d3 Port original AirRace HUD styling`
- rust-sandbox commit: `2f341a7 Update AirRace 3D original HUD build`
- WebGL build version: `20260426_prev_ui_port_fix`
- Dev URL: `https://rust-sandbox.wos.ktsys.jp/airrace3d/`

## 現在できていること

- Unity WebGL版 `/airrace3d/` が起動する。
- PC、スマホ、PS4/PS5コントローラー入力の基本検証が済んでいる。
- ROUND1向けの3Dワールド、地面、ゲート、パイロン、航路ガイド、滑走路のたたき台がある。
- HUDは前作寄せへ戻し、上部速度数字、航路ガイドウインドウ、下中央アナログ速度メーター、右側高度計、ミニマップ、FPS表示の土台がある。
- WebGLローダーはスマホ起動ログとChrome WebGL2注意表示を持つ。

## 次回の先頭タスク

1. 上部速度メーターを上へ、航路ガイドウインドウを下へずらし、重なりを解消する。
2. Uターン矢印を前作のドット感に寄せて作り直す。
3. 下中央アナログ速度メーターの左右反転を修正する。
4. デバッグウインドウを左下のまま少し上へ移動し、表示切れをなくす。
5. 背景グラデーションの水平線を前作に近づけ、ロール・ピッチに追従させる方式を検討する。
6. FPSが実測値か確認し、右下へ移動する。

## 注意

Unity版は、まだ前作UIの完全移植途中。
次回も「前作の画面を移植する」が最優先で、Unity独自の見た目はその後に足す。
