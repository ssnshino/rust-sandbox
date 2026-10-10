# 現行仕様の参照案

- Rust/axum側のPong、Breakout、Dungeon、Truckers、Canvas AirRaceと、Unity WebGL配布側を区別する。
- 配布資料の現行候補は source/repos/upstream/unity-rust-games/README.md と同 docs/airrace3d_webgl_assetbundle_release.md。Node/Fastifyの web/public/airrace3d、web/public/colony-racer と /api/airrace3d/*、/api/colony-racer/* を参照する。
- Rust個別ゲームの仕様は source/repos/upstream/rust-sandbox/docs の各仕様を参照。Truckersはv2を優先しv1も保存する。
- source/repos/upstream/unity-rust-games/docs/index.md は古い文書を削除済みと記載するが、今回は受領文書を削除せず保管する。
- ソースコード、WebGL、AssetBundle、カタログの役割を分ける。この文書ZIPにUnity実装本体はない。

## 固定commitの確認根拠

- [README.md](https://github.com/ssnshino/rust-sandbox/blob/4b7dcc079c311a05f2e6f6429fe00402992aa7e8/README.md): Node Web配布、AirRace/Colonyの公開ルートとUnity元repo分離を確認。
- [docs/airrace3d_webgl_assetbundle_release.md](https://github.com/ssnshino/rust-sandbox/blob/4b7dcc079c311a05f2e6f6429fe00402992aa7e8/docs/airrace3d_webgl_assetbundle_release.md): 既存配布手順は移動せず維持。現行入口から参照する。
- [docs/index.md](https://github.com/ssnshino/rust-sandbox/blob/4b7dcc079c311a05f2e6f6429fe00402992aa7e8/docs/index.md): 既存2文書へのリンクを維持し、本統合では旧docsを削除しない。
