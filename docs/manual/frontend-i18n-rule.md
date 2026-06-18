# Frontend I18n Rule

Timekeeper frontend の user-facing copy は、Rust コードへ直接書かず、`rust-i18n`
の translation key 経由で表示する。

## Scope

このルールは `frontend/src` と `apps/web` の UI 表示文言に適用する。

- ボタン、ラベル、見出し、説明文、空状態、確認ダイアログ、toast/message は translation key 経由にする
- `ja` と `en` の locale file を同じ変更で更新する
- page 固有の文言は `pages.<page>.*` に置く
- shared component 固有の文言は `components.<component>.*` に置く
- admin 配下の部品固有文言は `admin_components.<name>.*` に置く
- 複数画面で再利用する文言は `common.*` に寄せる
- frontend 内部生成の API/state error は `api.*` または `state.*` に寄せる
- locale switcher 自体の文言は `app.locale.*` に置く

## Exceptions

次は translation key 化の対象外としてよい。

- test fixture、mock data、snapshot に含まれる意図的な文字列
- code comment、developer-facing log、debug-only output
- backend から返る自然言語 error payload をそのまま表示する既存境界
- 日付、曜日、数値、単位の format そのもの

例外を追加する場合は、理由が読める場所に残す。

## Key Naming

translation key は階層型で、leaf は `lower_snake_case` にする。

```text
common.actions.save
components.empty_state.title
pages.login.form.email_label
admin_components.user_list.empty_message
api.errors.network_unavailable
```

新しい key を追加するときは、先に `common.*` で共有できるか確認する。
共有できない場合だけ、component/page/admin/API/state の最小スコープへ置く。

## Tests

UI 構造や挙動を検証する test は、翻訳済み文言の完全一致へ過度に依存しない。
文言自体を固定したい test では、locale を明示して `t!()` の解決結果を使う。

文言追加や移行を行った場合は、少なくとも次を確認する。

```bash
cargo fmt --all --check
cargo test -p timekeeper-frontend --lib <focused_target>
```

docs または harness source of truth を更新した場合は、次も実行する。

```bash
bash scripts/harness.sh docs-check
```
