# Unity games dev配布候補

対象はssnshino/rust-sandboxのdev、既存compose.dev.yamlのairrace-web。基準SHAは6dab17792279ae227ce5f240a66ec572c7283ce7。main向けdeploy.yml、games.lab、compose.yamlは変更しない。Unityビルドはしない。

## CI
verify-unity-games-dev.ymlはdev向けPR/更新でNode 22のnpm ci（install script無効）、check/build、組込テスト、両ゲームのcompiled WebGL/catalog/Bundle検証を行う。docs-only（md/txt）はpath filterで除外する。workflow_dispatchは手動検証用。
manifestには配布repo commit SHA、全ファイルのサイズ・raw SHA256・HTTP復号後SHA256・catalog版を記録する。Unity元ソースSHAは証拠がない限りnull。WebGLを再ビルドしたとは報告しない。

## deployは初期OFF
次の設定を、BASEのlive確認と利用者の明示承認後に用意する。値はこのPRへ書かない。新しい認証/永続アクセス設定は別の承認対象。

過去のdev配布workflowと同じrepo内のsecrets.DEPLOY_DEV_HOST / secrets.DEPLOY_DEV_PORT / secrets.DEPLOY_SSH_KEYを名前参照で再利用する。値の読取り・コピー・新しい鍵作成は不要。既存接続先がdev配布先BASEと同一で、この鍵の利用権限が対象を含むことを有効化前に確認する。異なるhostなら再利用を止め、接続方法を改めて確認する。既存dev定義と同じusername shinoを使い、SSH portは既存DEPLOY_DEV_PORTを参照する。過去のdev配布はSSH段階で失敗しており、secretの存在や名称一致だけでは接続成功を保証しない。mainの配置ディレクトリをdevへ流用しない。

新規Repository variablesは3つのみ:
- DEV_DEPLOY_ENABLED: 最初は未設定またはfalse。有効化承認後のみtrue。
- DEV_DEPLOY_REPO_PATH: read確認したdev repoルートの絶対パス。資料の候補は/home/shino/source/repos/upstream/unity-rust-gamesだが、未確認のまま設定しない。
- DEV_DEPLOY_KNOWN_HOSTS: 別経路でfingerprint確認済みknown_hosts行。StrictHostKeyCheckingを無効化しない。

配布targetはunity-games.wos.ktsys.jpにコード内固定し、接続後のdev branch / origin / Compose検証も必須。secret新設やrunner新設は本案に不要。既存鍵に新たなサーバ権限を付与する場合は別途承認が必要。

BASE側必須: bash、git、flock、Python3、Docker Compose v2、既存GitHub read認証、既存wos-proxy-network。dev branch、正しいorigin、追跡/未追跡含めcleanであること。runnerからSSH到達可能であること。BASEのdev checkoutは2026-10-10 read-only確認でclean、654f476c740f5e87778dce14ed301f0c2e7e817f、airrace-web稼働を確認。GitHub runnerから既存dev secretによる到達と鍵の利用権限は未確認。

## 動作と停止条件
- deployジョブはdev、enabled=true、target一致でのみ動く。PRにはsecretを渡さない。
- Node build/テスト/artifact検証を同ジョブで再実行してからSSH。
- SHA固定、origin/dev一致、fast-forward、clean tree、サーバlockを要求する。
- live checkout更新前に、対象SHAのCompose設定を一時ファイルで展開し、host集合・service/container・port・build path・mount・network・privileged禁止を確認する。
- airrace-webだけを--no-depsで更新する。
- /healthzを有界で待ち、全公開artifactの復号後hash、gzip/MIME、API catalog一致を確認する。
- 失敗は失敗として停止。自動reset/削除/rollbackをしない。直前SHAと要求SHAをログに残し、復旧は別途確認する。
- dev一致確認はfetch時点。確認後の新pushまで排除できる保証ではない。
- 既存bind mountのため更新は完全atomicではない。全工程中の無停止や自動rollbackを保証しない。
- WebGLブラウザ実行・ゲーム操作・Unity sourceとartifactの由来照合は別の未実施ゲート。HTTP smokeだけでゲーム完成としない。
- 公開artifactの一致は確認できるが、同一artifactのままserver codeのみ更新した場合の公開revision証明endpointは本案に含めない。

## 検証コマンド
node --test tests/game-delivery.test.mjs
bash -n scripts/deploy-unity-games-dev.sh
cd web && npm ci --ignore-scripts --no-audit --no-fund && npm run check && npm run build
SOURCE_SHA=<40桁commit> node scripts/verify-game-delivery.mjs . delivery-manifest.json
node scripts/smoke-game-delivery.mjs delivery-manifest.json http://127.0.0.1:3000 .

## 今回の検証結果
Node v24.19.0環境で組込テスト16件PASS、script構文とworkflow YAML parse、既定OFF gateを確認。workflowはNode22指定なので実CIで再検証する。
既存12ファイル（main deploy.yml、compose.dev.yaml、AGENTS、webソース/package/lock）はGit blob照合で無変更。READMEは既存本文を保持して本手順へのリンクだけを追記。
npm依存取得は初回cache書込みに失敗、/tmp cache再試行のプロセス確認がキャンセルされ、成功未確認。Nodeアプリcheck/build、本物WebGLバイナリ全件検証、BASE実配備、公開/ブラウザ確認は未実施。
