# Swagger UIスタブのローカルテスト方法

デフォルトでは`utoipa-swagger-ui`のビルド時にSwagger UIの配布物をインターネットから取得します。ネットワークに接続できない環境でビルドやテストを行う場合は、次の手順でローカルスタブのZIPを生成し、環境変数経由で参照してください。

1. `cd backend`
2. `cd tests/support/swagger-ui-dist && zip -r ../swagger-ui.zip swagger-ui-stub`
3. `cd ../..`
4. `SWAGGER_UI_DOWNLOAD_URL="file://$(pwd)/tests/support/swagger-ui.zip" cargo test`

上記のZIPは追跡対象外にしているため、必要に応じて各開発者の環境で作成してください。
