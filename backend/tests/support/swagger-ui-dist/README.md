# Swagger UIスタブのローカルテスト方法

デフォルトでは`utoipa-swagger-ui`のビルド時にSwagger UIの配布物をインターネットから取得します。ネットワークに接続できない環境でビルドやテストを行う場合は、次の手順でローカルスタブのZIPを生成し、環境変数経由で参照してください。

1. `cd backend`
2. `cd tests/support`
3. `zip -r swagger-ui.zip swagger-ui`
4. `cd ..`
5. `SWAGGER_UI_DOWNLOAD_URL="file://$(pwd)/tests/support/swagger-ui.zip" cargo test --test auth_api`

ZIP は追跡対象外です。必要に応じて各自の環境で生成してください。`swagger-ui-stub` はスタブの初期状態を保持するための控えで、内容を変更したい場合はここから `tests/support/swagger-ui` に反映した上で ZIP を再生成してください。
