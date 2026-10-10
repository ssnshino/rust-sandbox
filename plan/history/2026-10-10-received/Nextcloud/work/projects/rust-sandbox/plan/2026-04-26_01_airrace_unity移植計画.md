# ベクター・エアレース Unity 3D 移植計画

作成日: 2026-04-26 JST

---

## ねらい

Web版 `airrace` をいきなり全部移植するのではなく、まず `ROUND 1` だけを対象にした Unity 版の縦切りプロトタイプを作る。

最初の完成条件は次に固定する。

- `ROUND 1` を 1 本通して遊べる
- `PC` `スマホ` `PS4/PS5 コントローラー` の 3 系統で操作できる
- `速度メーター` `ミニマップ` `ダメージ` `タイマー` `高度計` を現行仕様に近い形で表示する
- ゲート通過、クラッシュ、クリアまでの基本ループが成立する

ここでは `Grand Prix 全体` や `ゴースト` `CPU多機数レース` までは追わず、Unity での土台づくりを最優先にする。

---

## 推奨方針

### ベースプロジェクト

推奨は `flightPong` を完全流用するのではなく、`flightPong` を足場にしつつ `airrace` 用の新しい Unity プロジェクトとして切る方針。

理由:

- `flightPong` は入力確認や最小シーンの参考にはなる
- ただしゲーム内容が別なので、そのまま育てると責務が濁りやすい
- `airrace` は HUD、コース、飛行、ゲート判定、ミニマップが主役で、構造を最初から分けた方が後で楽

実務的には次のどちらか。

1. `flightPong` をコピーして `airraceUnity` として開始
2. 新規 Unity プロジェクトを切って、必要な入力設定だけ `flightPong` から持ってくる

今回のおすすめは `1`。
最初の入力確認が早いから。

### 実装の大前提

- `Unity Input System` を採用する
- 最初は `1 Scene` で作る
- 最初は `1機のみ`
- `ROUND 1` 専用固定コース
- 見た目はローポリ/簡易プリミティブでよい
- 物理は厳密な航空力学ではなく、現行 `airrace` の操作感を寄せる

---

## Unity 版の最初の完成定義

### 必須

- タイトルなしでよいので、起動したら `ROUND 1` シーンへ入る
- 自機を前進させ、左右旋回と上下移動ができる
- BOOST で加速できる
- ゲートを順番に通過できる
- 地面激突でクラッシュする
- 全ゲート通過後にクリアになる
- 次の HUD を表示する
  - 上中央: 大きい速度数字
  - 下中央: 速度メーター
  - 右側: 縦テープ式高度計
  - 左上: ミニマップ
  - 右上: `ROUND / TIME / DAMAGE`

### 今回はまだ不要

- 5機選択
- ポイント制
- CPUライバル
- ゴースト保存/再生
- リザルト画面の豪華演出
- ラウンドJSON配信
- 各国背景演出

---

## Phase 構成

## Phase 0: 土台決め

目的:

- Unity 側の開始地点を固定する

やること:

- `flightPong` ベースで新ブランチまたは新フォルダを切る
- Unity バージョンを固定する
- `Input System` の採用を確定する
- シーン構成を `Boot` ではなく `Round1Scene` 1本に絞る
- フォルダ構成を先に切る

推奨フォルダ:

- `Assets/AirRace/Scripts/Core`
- `Assets/AirRace/Scripts/Input`
- `Assets/AirRace/Scripts/Flight`
- `Assets/AirRace/Scripts/Course`
- `Assets/AirRace/Scripts/HUD`
- `Assets/AirRace/Prefabs`
- `Assets/AirRace/Scenes`

完了条件:

- Unity プロジェクトが起動し、`Round1Scene` を再生できる
- Input System 有効化まで終わっている

---

## Phase 1: 操作縦切り

目的:

- 3系統入力で自機を動かせるようにする

やること:

- `PC`
  - `A/D` または `←/→`: 左右旋回
  - `W/S` または `↑/↓`: 上下操作
  - `Space`: BOOST
- `スマホ`
  - 左下プニコン
  - 右下 BOOST ボタン
- `PS4/PS5`
  - 左スティック左右: 旋回
  - 左スティック上下: 上下
  - `R2` または `×/R1`: BOOST

実装メモ:

- 入力は `AirRaceInputRouter` に集約する
- 機体制御は `PlayerFlightController` で受ける
- 画面UIから直接機体を触らない

完了条件:

- 3入力系統で同じ挙動の自機操作ができる
- 入力切替で破綻しない

---

## Phase 2: ROUND 1 コース成立

目的:

- `ROUND 1` を 1 本飛べるようにする

やること:

- Web版 `ROUND 1` の固定レイアウトを Unity に移す
- ゲートPrefabを作る
- ゲートの通過順管理を作る
- 地面と空を最低限敷く
- クラッシュ条件を入れる
- クリア条件を入れる

実装メモ:

- 最初は `ScriptableObject` か `MonoBehaviour` 配列で固定コースを持てばいい
- JSON 読み込みは後回しでよい
- 地面は Plane 1枚、空は Skybox で十分

完了条件:

- `ROUND 1` のスタートからクリアまで一通り遊べる

---

## Phase 3: HUD 再現

目的:

- 現行 `airrace` の最低限の見やすさを Unity 側へ持つ

やること:

- 上中央の大きい速度数字
- 下中央の速度メーター
- 右側の縦テープ式高度計
- 左上のミニマップ
- 右上の `ROUND / TIME / DAMAGE`

実装メモ:

- `Canvas` は `Screen Space - Overlay`
- ミニマップは `TopDown Camera + RenderTexture` が素直
- 高度計は `RectMask2D` つきのスクロール表示で作る
- 速度メーターは最初は針なしでもよいが、最終的には現行に寄せる

完了条件:

- HUDだけ見てプレイ状態が分かる
- スマホでも情報が潰れない

---

## Phase 4: プレイ感の調整

目的:

- Web版の「気持ちよさ」を Unity 版でも再現する

やること:

- 旋回の軽さ
- 上下移動量
- BOOST の伸び
- カメラ追従
- 地面衝突の厳しさ
- ゲート判定の気持ちよさ

調整観点:

- 離陸直後に事故りすぎないか
- ROUND 1 がチュートリアルとして優しいか
- PC とスマホで難易度差が出すぎないか

完了条件:

- ROUND 1 を「普通に遊べる」
- 触っていて Web版に近い手触りがある

---

## 実装順のおすすめ

順番はこれがよさそう。

1. `Input System` で 3系統入力を通す
2. 自機を飛ばす
3. 固定ゲートを置く
4. 通過判定を作る
5. クラッシュとクリアを入れる
6. HUD をのせる
7. カメラと手触りを詰める

HUD を先に作りたくなるけど、先に飛ばせた方が調整が圧倒的に早い。

---

## クラスのたたき台

- `AirRaceGameController`
  - ラウンド進行、タイマー、クリア/失敗管理
- `AirRaceInputRouter`
  - PC / Touch / Gamepad 入力統合
- `PlayerFlightController`
  - 速度、旋回、上下、BOOST
- `GateSequenceController`
  - 次ゲート管理、通過判定
- `CrashJudge`
  - 地面や障害物との衝突
- `AirRaceHUDController`
  - 速度、高度、タイマー、ダメージ、ROUND 表示
- `MiniMapController`
  - ミニマップ描画
- `RoundCourseDefinition`
  - ROUND 1 のゲート配置定義

---

## 最初の見た目方針

最初は豪華にしすぎなくてよい。

- 機体: 単純なローポリ
- 地面: 単色 + 薄いグリッド
- 空: Skybox
- ゲート: 明るいリング
- パイロン: 円柱2本

大事なのは「3D化そのもの」より、

- 入力が破綻しない
- HUD が見やすい
- ROUND 1 が最後まで遊べる

この3つ。

---

## この計画の到達点

この計画でまず目指すのは、

`Unity で ROUND 1 を、PC/スマホ/PS4/PS5 の3系統入力つきで遊べる最初の完成版`

ここまで行けば、その次に

- Round 2 追加
- 機体選択
- ゴースト
- CPUライバル
- Grand Prix 化

をかなり安全に積める。

---

## 次の一手

この計画の次に着手する作業は次。

1. `flightPong` をベースにするか新規プロジェクトにするか最終決定
2. Unity 側に `Phase 0` のフォルダ構成を切る
3. `Input System` で `PC / Touch / Gamepad` の共通入力ルータを作る

おすすめは、次の実作業でそのまま `Phase 0 + Phase 1 の入力土台` まで進めること。
