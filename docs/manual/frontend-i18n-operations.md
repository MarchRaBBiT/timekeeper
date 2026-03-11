# Frontend i18n Operations

## Purpose

frontend の UI 文言追加・変更時に、直書きへ戻さないための運用ルールを定義する。

現時点では frontend の運用ドキュメントは日本語を正本とする。英語版を追加する場合は、同一内容の companion doc を並走させる。

## Rules

- user-facing UI copy は Rust ソースへ直書きせず、必ず translation key 経由で参照する
- 新しい UI copy を追加する場合は、先に `frontend/locales/ja.yml` と `frontend/locales/en.yml` に key を追加する
- key は用途単位で階層化し、page 固有なら `pages.<page>.*`、shared component 固有なら `components.<component>.*`、共通操作や汎用ラベルなら `common.*` を使う
- locale 切替で変わるべき文言は `rust_i18n::t!(...)` または translation helper から取得する
- backend から自然言語で返る error payload はこのルールの例外とし、frontend 内で勝手に再翻訳しない
- 日付、曜日、単位など今回の i18n scope 外フォーマットは既存仕様を維持し、copy だけを key 化する

## Review Checklist

- 新規・変更した UI 文言に対応する key が `ja.yml` と `en.yml` の両方にある
- view / view_model / helper に user-facing copy の直書きが増えていない
- test は可能なら translation key の解決結果か構造を見ており、locale 固定値への過度な依存を増やしていない
- docs と運用ルールに反する例外がある場合は、PR 説明か ExecPlan に理由を残す

## Exceptions

- test data、test assertion、コメント、ログ、fixture、backend 由来メッセージの受け渡しは直書きが残りうる
- 一時的に直書きが必要な場合でも、その PR 内で follow-up plan か TODO owner を明記する
