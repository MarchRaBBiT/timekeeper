# フロントエンド: ボタンホバー効果の統一

## 目的
- 監査ログページで実装されたホバー効果（`hover:bg-indigo-700` など）を他の管理画面ページにも展開し、UI の一貫性と UX を向上させる。

## 背景
- 監査ログページでは `hover:bg-indigo-700` が適用されているが、他のページでは未適用
- ホバー効果はユーザーにインタラクティブ要素であることを明示するため UX 向上に寄与

## 作業タスク
- [ ] `admin_export/panel.rs` のボタンに `hover:bg-indigo-700` を追加
- [ ] `admin_users/components/detail.rs` のボタンに `hover:bg-indigo-700` を追加
- [ ] `admin_users/components/invite_form.rs` のボタンに `hover:bg-blue-700` を追加
- [ ] `attendance/components/form.rs` のボタンにホバー効果を追加
- [ ] `mfa/components/setup.rs` のボタンにホバー効果を追加
- [ ] `requests/components/overtime_form.rs` のボタンにホバー効果を追加
- [ ] 他のページでも同様のパターンを確認し適用

## 受入条件
- すべてのプライマリボタンに一貫したホバー効果が適用されている
- ビルドエラーがない

## テスト
- 目視確認（各ページのボタンにマウスオーバーしてホバー効果を確認）

## メモ
- 参考実装: `frontend/src/pages/admin_audit_logs/panel.rs` の L86
