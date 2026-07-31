# EP-20260730-lan-manual-testbed

## Goal

- 同一LAN上の別端末からTimekeeperへHTTPS接続し、人手で主要業務フローを確認できる専用テストベッドを提供する
- database/RedisをLANへ公開せず、明示origin、Secure cookie、自己署名証明書を使う

## Scope

- `docker-compose.lan-test.yml`
- `scripts/lan-testbed.sh`
- `scripts/tests/lan_testbed_contract.sh`
- `frontend/docker-entrypoint.sh`
- `docs/manual/lan-testbed.md`

## Completion Criteria

- [x] 専用composeの構成契約テストがある
- [x] frontendだけが指定LAN IPへ公開され、backend/DB/Redisは非公開
- [x] 指定したLANホスト名/IPが証明書SANとCORS allowlistへ反映される
- [x] Secure cookieと明示originを維持したままE2E用rate limitを設定する
- [x] Podmanでbuild/startし、HTTPS経由のtimezone APIがgreen
- [x] `docs-check`、`fmt-check`、`lint`、関連テストがgreen
- [x] security/code reviewでCRITICAL/HIGHがない
- [x] commitを作成する

## Progress Notes

- 2026-07-30: 現行compose、nginx proxy、cookie/CORS/CSRF設定を確認。LANへfrontendのみ公開する専用構成を採用。
- 2026-07-30: 契約テストを先に追加し、専用compose未作成によるREDを確認。
- 2026-07-30: security reviewのHigh（既存composeの全IF公開、固定secret、非Secure Cookie、LAN SAN欠如）を専用compose、ランダムJWT secret、Secure Cookie、動的SANで回避。code reviewのHigh（永続証明書と変更後host/IPの不一致）を起動前SAN照合で修正。
- 2026-07-30: `rabbitsrv.local` / `192.168.11.2` でPodman build/start完了。公開は`192.168.11.2:8443 -> frontend:443`のみで、backend/PostgreSQL/Redisにhost portなし。証明書SANと`GET /api/config/timezone` (`Asia/Tokyo`)を実測。
- 2026-07-30: `scripts/tests/lan_testbed_contract.sh`、`bash -n`、`docs-check`、`fmt-check`、`lint`、`git diff --check`がgreen。
