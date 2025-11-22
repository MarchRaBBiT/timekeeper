# Swagger UIスタブ

`utoipa-swagger-ui` のビルド時にインターネットへアクセスできない環境でもテストが通るよう、最小構成の Swagger UI スタブを同梱しています。ZIP アーカイブは追跡対象外なので、必要に応じて各自で生成してください。

ローカルで使用する場合は、次の手順で ZIP を作成した上で環境変数を指定してテストを実行します。

```bash
cd backend
cd tests/support
zip -r swagger-ui.zip swagger-ui
cd ../..
SWAGGER_UI_DOWNLOAD_URL="file://$(pwd)/tests/support/swagger-ui.zip" cargo test --test auth_api
```

スタブの内容を更新する場合は、`tests/support/swagger-ui/dist` 配下を編集してから再度 ZIP を生成してください。
