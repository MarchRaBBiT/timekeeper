# LAN手動確認テストベッド

## 目的

同一LAN上のPC・タブレット・スマートフォンから、Timekeeperの画面と主要業務フローを人手で確認するための隔離されたPodman Compose環境です。本番運用には使用しません。

LANへ公開するのはフロントエンドのHTTPSポートだけです。backend、PostgreSQL、Redisはコンテナネットワーク内だけで待ち受けます。ブラウザからのAPI通信はフロントエンドと同じoriginの`/api`を経由します。

## 起動

ホストのLAN名が名前解決できる場合:

```bash
LAN_TESTBED_HOST=timekeeper.local bash scripts/lan-testbed.sh up
```

名前解決を用意しない場合は、LAN IPをホスト名として指定できます。

```bash
LAN_TESTBED_HOST=192.168.1.20 LAN_TESTBED_IP=192.168.1.20 \
  bash scripts/lan-testbed.sh up
```

スクリプトは非loopback IPv4を自動検出します。複数NICやVPNがある場合は`LAN_TESTBED_IP`を明示してください。既定ポートが競合する場合は`LAN_TESTBED_HTTPS_PORT`を変更できます。

起動後に表示される`https://<host>:8443`をLAN端末で開きます。証明書は30日間の自己署名証明書で、指定したホスト名とIPv4をSANに含みます。`target/lan-testbed/tls/localhost.crt`をテスト端末へコピーして信頼済み証明書として登録するのが推奨です。一時的にブラウザ例外を使う場合は、警告内容と接続先を確認し、このテストベッドに限って許可してください。

ブラウザでは必ず起動時に指定した`LAN_TESTBED_HOST`を使ってください。CORS/CSRF allowlistはこの単一originに限定されるため、ホスト名を指定して起動した環境へIP直打ちでアクセスすると更新操作は拒否されます。IP直打ちで確認する場合は、上の例のように`LAN_TESTBED_HOST`にも同じIPを指定して起動します。ホスト名/IPを変更する場合、既存証明書との不一致をスクリプトが検出して停止するため、`target/lan-testbed/tls`を削除して再起動してください。

- 初期ユーザー: `admin`
- 初期パスワード: `admin123`
- ホスト上の疎通確認: `bash scripts/lan-testbed.sh check`
- 状態確認: `bash scripts/lan-testbed.sh status`
- ログ: `bash scripts/lan-testbed.sh logs`
- 停止（データ保持）: `bash scripts/lan-testbed.sh down`
- 初期化（専用DB/Redis volume削除）: `bash scripts/lan-testbed.sh reset`

`reset`はテストベッド専用volumeのデータを削除します。通常の`docker-compose.yml`のvolumeは対象にしません。

## LAN端末での確認項目

1. PCとスマートフォンの両方でログインできる
2. 従業員を作成し、そのユーザーでログインできる
3. 出勤、休憩開始・終了、退勤が画面へ反映される
4. 休暇・残業・勤怠修正を申請し、管理者が承認または却下できる
5. 月表示、管理者画面、CSV出力を確認できる
6. ページ再読込後もログイン状態が維持される
7. 異なるLAN端末から同じデータ更新を確認できる

確認中のデータは専用volumeへ残るため、複数端末のシナリオを継続できます。やり直す場合のみ`reset`を使用してください。

## セキュリティ上の制約

- 信頼できる隔離LANでのみ起動してください。
- 初期パスワードは公開情報です。共有LANではログイン直後に変更してください。
- firewallでTCP 8443（または指定したHTTPSポート）をテスト端末のサブネットだけに許可してください。
- backend、PostgreSQL、Redisにはホスト側ポートがありません。別のcompose設定で公開しないでください。
- レート制限は安全な既定値です。高負荷E2Eで必要な場合のみ`LAN_TESTBED_RATE_LIMIT_*`を明示的に上書きし、本番設定へ流用しないでください。
- `.lan-testbed.env`には専用JWT secretが保存され、Gitの対象外です。
