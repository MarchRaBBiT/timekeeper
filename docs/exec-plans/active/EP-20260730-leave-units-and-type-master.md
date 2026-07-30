# EP-20260730-leave-units-and-type-master

**Status:** 実装中  
**Parent task:** [attendance-domain-gap-tasks.md](../attendance-domain-gap-tasks.md) T-11

## Goal

既存の日単位休暇を後方互換のまま半休・時間単位へ拡張し、会社固有の休暇種別を System Admin が管理できるようにする。承認時の残高消化、取消時の release、勤怠表示まで分単位で一貫させる。

## Scope And Decisions

- migration `060` に休暇種別マスタ、既存 `annual / sick / personal / other` seed、参照済み種別を物理削除しない無効化規則を置く。
- migration `061` に休暇申請の取得単位と時間指定、および ledger の年5日義務算入を再現できる immutable metadata を置く。
- API の `leave_type` 文字列は維持する。既存 payload で取得単位が省略された場合は `day` とする。
- `LeaveType` のコード内4種 enum を master-backed codeへ移行する。custom種別も `balance_tracked=true` の場合は種別別ledgerを利用でき、残高・adjust・付与・申請承認consume・取消releaseの各入口でmasterをfail-closed検証する。各ledger管理APIの`leave_type_code`省略時は`annual`とする。
- `half_am / half_pm` は固定240分ではなく対象日の resolved workday の所定区間から算出する。奇数分は従業員に不利な過大消化を避けるため切り捨て、残余1分は午後側へ割り当てる。
- `hour` は単日、開始 < 終了、勤務予定区間内、分単位で指定する。複数日時間休は拒否する。
- 半休は年5日義務へ0.5日として算入し、時間休は算入しない。ledger eventだけから再現できる情報を保存する。
- 承認済み半休・時間休は実勤務と同日に共存し、全日休のように実績行を置換しない。

## TDD And Implementation

1. contract round-tripで既存day payload、4単位、custom typeをREDにする。
2. domain/app unit testで所定区間からの分換算、FIFO consume/release、義務算入、invalid combinationをREDにする。
3. migration 060–061、master repository、System Admin CRUD、薄いhandlerを実装する。
4. leave create/update/approve/cancelをmaster validationと共通のrequested-minutes計算へ接続する。
5. T-06 attendance/calendar/summary/export read pathへ単位・分・時間帯を追加する。
6. frontend request formをmaster read APIと単位別入力へ接続する。
7. OpenAPI、backend API catalog、leave entitlement設計を同期する。

## Observable Acceptance

- 既存の日単位申請payloadとresponseが互換である。
- 半休・時間休の申請、承認、残高減、取消release、勤怠表示がintegration testで通る。
- 未許可単位、無効化種別、不正時間帯、全休日申請、schedule未解決をfail-closedで拒否する。
- custom非残高連動種別は台帳消費なしで申請でき、custom残高連動種別は年次有給と分離したledgerで申請・承認consume・取消releaseできる。
- frontend host test、contract test、backend integration、lintがgreen。

## Validation

- [x] contract/domain/app RED → GREEN（2026-07-30: contract 14件、app leave ledger 24件）
- [x] focused backend integration（2026-07-30: custom tracked申請→承認consume→取消release、1件）
- [x] frontend host tests（既存requests host/API tests green）
- [x] `bash scripts/harness.sh docs-check`
- [x] `bash scripts/harness.sh lint`
- [x] 関連するbroader backend tests（leave request ledger 13 passed、backend lib 409 passed）

## Progress Notes

- 2026-07-30: `leave_type_code`（既定`annual`）を残高query・付与・adjust contractとapp commandへ追加。custom tracked種別をmasterの`balance_tracked`で検証し、申請作成/更新時の残高検証、承認consume、取消releaseまで実コードを伝播するよう一般化した。custom `wellness`と`annual`の残高分離をintegration testで確認した。
