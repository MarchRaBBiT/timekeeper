# EP-20260213-attendance-correction-request

## Goal
- フロントエンドに勤怠修正依頼機能を追加し、承認後に補正値を正として集計・表示・CSVへ反映する

## Scope
- In: `backend`（migration/model/repository/handler/router）、`frontend`（API/types/requests画面）、関連テスト
- Out: メール通知、既存過去データ移行、上長ロール新設

## Done Criteria (Observable)
- [x] 従業員が1日単位の勤怠修正依頼（出勤/退勤/休憩明細+理由）を作成/更新/取消できる
- [x] 管理者が勤怠修正依頼を一覧/詳細確認し承認/却下できる
- [x] 承認時に原本差分を検知して衝突エラーを返せる
- [x] 承認済み補正値が勤怠履歴・月次サマリ・CSVに反映される
- [x] requests画面から勤怠修正依頼を操作できる

## Constraints / Non-goals
- 原本打刻（attendance / break_records）は上書きしない
- 値クリア（未設定化）は許可しない
- SQLx migrationは新規追加のみ

## Task Breakdown
1. [x] DBマイグレーション追加（修正依頼・補正値テーブル）
2. [x] backend モデル/リポジトリ/ハンドラー/ルーティング実装
3. [x] 勤怠集計・履歴・CSVへ補正優先ロジック導入
4. [x] frontend API/types/requests画面に勤怠修正依頼UIを追加
5. [x] backend/frontend テスト追加と回帰確認

## Validation Plan
- [x] `cargo fmt --all`
- [x] `cargo test -p timekeeper-backend --lib`
- [x] `cargo test -p timekeeper-backend --test requests_api`
- [x] `cargo test -p timekeeper-frontend --lib requests`

## JJ Snapshot Log
- [x] `jj status`
- [x] backend/frontend 対象テスト pass
- [ ] `jj commit -m "feat(requests): add attendance correction request workflow"`

## Progress Notes
- 2026-02-13: 計画作成
- 2026-02-13: migration/handler/repository/frontend requests UI まで実装し、`cargo fmt --all`、`cargo test -p timekeeper-backend --lib`、`cargo test -p timekeeper-backend --test requests_api -- --nocapture`、`cargo test -p timekeeper-frontend requests:: -- --nocapture` を通過。
