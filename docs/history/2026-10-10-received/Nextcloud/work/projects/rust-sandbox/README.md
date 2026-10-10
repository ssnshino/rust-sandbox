# rust-sandbox

Rust + axum で動く WebSocket ゲーム実験コンテナの運転席。

## 目的

- Rust / axum / WebSocket の実験場として、ゲームを実装・改良する
- 本番環境（ktsys-pubserver）への自動デプロイで動作確認できる状態を維持する

## 現在地

- 本番稼働中: https://games.lab.ktsys.jp
- 開発環境: http://rust-sandbox.wos.ktsys.jp
- GitHub: https://github.com/ssnshino/rust-sandbox（※実際のリポジトリ URL を確認して更新すること）
- CI/CD: main push → GitHub Actions → ktsys-pubserver 自動デプロイ

## 実装済みゲーム

| ゲーム | URL | 状態 |
|--------|-----|------|
| Pong（vs CPU / 2P） | `/pong` | 稼働中 |
| Breakout | `/breakout` | 稼働中 |
| あやかダンジョン | `/dungeon` | 稼働中 |

## コード本体

- upstream: `/home/shino/work/upstream/rust-sandbox/`

## 開発フロー

```bash
# dev ブランチで開発・動作確認
cd /home/shino/work/upstream/rust-sandbox
git checkout dev
docker compose -f compose.dev.yaml up --build -d

# main にマージ → 自動デプロイ
gh pr create --base main
```

## 参照先

- 仕様書: `/home/shino/work/upstream/rust-sandbox/docs/`
- upstream README: `/home/shino/work/upstream/rust-sandbox/README.md`
- 作業ログ: `working-logs/`（作業ごとに積み上げていく）
- 計画・メモ: `plan/`
  - `plan/2026-04-16_01_あやかダンジョン拡張計画.md`
- 作業メモ: `working-notes/`（今後追加）

## 次アクション

- [ ] GitHub リポジトリ URL を確認して上記に記入する
- [ ] `plan/2026-04-16_01_あやかダンジョン拡張計画.md` に沿って Phase 1 から着手する

## 更新履歴

| 日時 | 内容 |
|------|------|
| 2026-04-15 JST | プロジェクト新設 |
| 2026-04-15 JST | 作業ログ追加（working-logs/） |
