use crate::api::{ApiClient, UserResponse};
use crate::components::layout::*;
use crate::pages::admin::components::user_select::{
    AdminUserSelect, UserSelectValue, UsersResource,
};
use crate::state::auth::use_auth;
use crate::utils::trigger_csv_download;
use chrono::NaiveDate;
use leptos::*;

#[derive(Clone, Default)]
struct ExportFilters {
    username: String,
    from_date: String,
    to_date: String,
}

impl ExportFilters {
    fn normalized_str(value: &str) -> Option<&str> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    fn username_param(&self) -> Option<&str> {
        Self::normalized_str(&self.username)
    }

    fn from_param(&self) -> Option<&str> {
        Self::normalized_str(&self.from_date)
    }

    fn to_param(&self) -> Option<&str> {
        Self::normalized_str(&self.to_date)
    }

    fn validate(&self) -> Result<(), String> {
        let from_str = self.from_date.trim();
        let to_str = self.to_date.trim();

        let parse_date = |value: &str| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| "日付は YYYY-MM-DD 形式で入力してください。".to_string())
        };

        let from = if from_str.is_empty() {
            None
        } else {
            Some(parse_date(from_str)?)
        };
        let to = if to_str.is_empty() {
            None
        } else {
            Some(parse_date(to_str)?)
        };

        if let (Some(f), Some(t)) = (from, to) {
            if f > t {
                return Err("From は To 以前の日付を指定してください。".into());
            }
        }

        Ok(())
    }
}

fn needs_specific_user_selection(use_specific_user: bool, username: &str) -> bool {
    use_specific_user && username.trim().is_empty()
}

#[component]
pub fn AdminExportPage() -> impl IntoView {
    let (auth, _set_auth) = use_auth();
    let auth_for_guard = auth.clone();
    let is_admin = create_memo(move |_| {
        auth_for_guard
            .get()
            .user
            .as_ref()
            .map(|user| user.is_system_admin || user.role.eq_ignore_ascii_case("admin"))
            .unwrap_or(false)
    });

    view! {
        <Layout>
            <Show
                when=move || is_admin.get()
                fallback=move || {
                    view! {
                        <div class="space-y-6">
                            <div class="bg-white shadow rounded-lg p-6">
                                <p class="text-sm text-gray-700">
                                    {"このページは管理者以上の権限が必要です。"}
                                </p>
                            </div>
                        </div>
                    }
                }
            >
                <AdminExportPanel />
            </Show>
        </Layout>
    }
}

#[component]
fn AdminExportPanel() -> impl IntoView {
    let users_resource: UsersResource = create_resource(
        || true,
        |_| async move { ApiClient::new().get_users().await },
    );
    let users_error = Signal::derive(move || users_resource.get().and_then(|res| res.err()));
    let error = create_rw_signal(None::<String>);
    let preview = create_rw_signal(None::<String>);
    let filename_state = create_rw_signal(String::new());
    let username = create_rw_signal(String::new());
    let from_date = create_rw_signal(String::new());
    let to_date = create_rw_signal(String::new());
    let use_specific_user = create_rw_signal(false);
    let specific_user_disabled = Signal::derive(move || !use_specific_user.get());
    let specific_user_required = Signal::derive(move || {
        needs_specific_user_selection(use_specific_user.get(), &username.get())
    });

    let export_action = create_action(move |filters: &ExportFilters| {
        let snapshot = filters.clone();
        async move {
            let api = ApiClient::new();
            api.export_data_filtered(
                snapshot.username_param(),
                snapshot.from_param(),
                snapshot.to_param(),
            )
            .await
        }
    });

    {
        let export_action = export_action.clone();
        let preview = preview.clone();
        let filename_state = filename_state.clone();
        let error = error.clone();
        create_effect(move |_| {
            if let Some(result) = export_action.value().get() {
                match result {
                    Ok(payload) => {
                        let fname = payload
                            .get("filename")
                            .and_then(|s| s.as_str())
                            .unwrap_or("export.csv");
                        let csv = payload
                            .get("csv_data")
                            .and_then(|c| c.as_str())
                            .unwrap_or("");
                        filename_state.set(fname.to_string());
                        preview.set(Some(csv.chars().take(2000).collect()));
                        let _ = trigger_csv_download(fname, csv);
                        error.set(None);
                    }
                    Err(message) => {
                        error.set(Some(format!("CSVの取得に失敗しました: {}", message)));
                        preview.set(None);
                    }
                }
            }
        });
    }

    let on_export = {
        let export_action = export_action.clone();
        let preview = preview.clone();
        let error = error.clone();
        let username = username.clone();
        let from_date = from_date.clone();
        let to_date = to_date.clone();
        let use_specific_user = use_specific_user.clone();
        move |_| {
            let filters = ExportFilters {
                username: if use_specific_user.get_untracked() {
                    username.get_untracked()
                } else {
                    String::new()
                },
                from_date: from_date.get_untracked(),
                to_date: to_date.get_untracked(),
            };
            if needs_specific_user_selection(use_specific_user.get_untracked(), &filters.username) {
                error.set(Some("指定ユーザーを選択してください。".into()));
                preview.set(None);
                return;
            }
            if let Err(msg) = filters.validate() {
                error.set(Some(msg));
                preview.set(None);
                return;
            }
            error.set(None);
            preview.set(None);
            export_action.dispatch(filters);
        }
    };

    let downloading = export_action.pending();

    view! {
        <div class="space-y-6">
            <div class="bg-white shadow rounded-lg p-6">
                <h2 class="text-lg font-medium text-gray-900 mb-4">{"データエクスポート (CSV)"}</h2>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3 mb-4">
                    <div>
                        <label class="block text-sm text-gray-700">{"ユーザー"}</label>
                        <div class="mt-1 space-y-2">
                            <div class="flex items-center gap-4 text-sm text-gray-700">
                                <label class="flex items-center gap-1">
                                    <input
                                        type="radio"
                                        name="export_user_filter"
                                        value="all"
                                        checked=move || !use_specific_user.get()
                                        on:change=move |_| {
                                            use_specific_user.set(false);
                                            username.set(String::new());
                                            error.set(None);
                                            preview.set(None);
                                        }
                                    />
                                    {"全ユーザー"}
                                </label>
                                <label class="flex items-center gap-1">
                                    <input
                                        type="radio"
                                        name="export_user_filter"
                                        value="specific"
                                        checked=move || use_specific_user.get()
                                        on:change=move |_| {
                                            use_specific_user.set(true);
                                            error.set(None);
                                            preview.set(None);
                                        }
                                    />
                                    {"指定ユーザー"}
                                </label>
                            </div>
                            <Show when=move || users_error.get().is_some()>
                                <div class="space-y-1">
                                    <input
                                        class="w-full border rounded px-2 py-1 disabled:opacity-50"
                                        placeholder="username を直接入力"
                                        prop:value={move || username.get()}
                                        on:input=move |ev| username.set(event_target_value(&ev))
                                        disabled=specific_user_disabled
                                    />
                                    <ErrorMessage message={users_error.get().unwrap_or_default()} />
                                </div>
                            </Show>
                            <Show when=move || users_error.get().is_none()>
                                <AdminUserSelect
                                    users=users_resource.clone()
                                    selected=username
                                    label=None
                                    placeholder="ユーザーを選択してください".into()
                                    value_kind=UserSelectValue::Username
                                    disabled=specific_user_disabled
                                />
                            </Show>
                            <Show when=move || specific_user_required.get()>
                                <p class="text-xs text-amber-600">
                                    {"指定ユーザーを選択してください。"}
                                </p>
                            </Show>
                        </div>
                    </div>
                    <div>
                        <label class="block text-sm text-gray-700">{"From (YYYY-MM-DD)"}</label>
                        <input
                            type="date"
                            class="mt-1 w-full border rounded px-2 py-1"
                            prop:value={move || from_date.get()}
                            on:input=move |ev| from_date.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class="block text-sm text-gray-700">{"To (YYYY-MM-DD)"}</label>
                        <input
                            type="date"
                            class="mt-1 w-full border rounded px-2 py-1"
                            prop:value={move || to_date.get()}
                            on:input=move |ev| to_date.set(event_target_value(&ev))
                        />
                    </div>
                </div>
                <Show when=move || error.get().is_some()>
                    <ErrorMessage message={error.get().unwrap_or_default()} />
                </Show>
                <button
                    class="px-4 py-2 bg-indigo-600 text-white rounded disabled:opacity-50"
                    on:click=on_export
                    disabled=move || downloading.get() || specific_user_required.get()
                >
                    {move || if downloading.get() { "エクスポート中..." } else { "CSVをエクスポート" }}
                </button>
                <Show when=move || preview.get().is_some()>
                    <div class="mt-4">
                        <h3 class="text-sm text-gray-700">
                            {"プレビュー (先頭2KB) - ファイル名: "}
                            {move || filename_state.get()}
                        </h3>
                        <pre class="mt-2 text-xs bg-gray-50 p-3 rounded overflow-auto max-h-64 whitespace-pre-wrap">
                            {move || preview.get().unwrap_or_default()}
                        </pre>
                    </div>
                </Show>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::needs_specific_user_selection;

    #[test]
    fn specific_user_requires_selection() {
        assert!(needs_specific_user_selection(true, ""));
        assert!(needs_specific_user_selection(true, "   "));
        assert!(!needs_specific_user_selection(true, "admin"));
        assert!(!needs_specific_user_selection(false, ""));
    }
}
