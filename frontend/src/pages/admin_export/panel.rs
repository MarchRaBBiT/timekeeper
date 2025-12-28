use super::view_model::{
    needs_specific_user_selection, use_admin_export_view_model, ExportFilters,
};
use crate::components::forms::DatePicker;
use crate::components::layout::*;
use crate::pages::admin::components::user_select::{AdminUserSelect, UserSelectValue};
use crate::state::auth::use_auth;
use leptos::*;

#[component]
pub fn AdminExportPage() -> impl IntoView {
    let (auth, _set_auth) = use_auth();
    let is_admin = create_memo(move |_| {
        auth.get()
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
    let vm = use_admin_export_view_model();

    let users_error = Signal::derive(move || vm.users_resource.get().and_then(|res| res.err()));
    let specific_user_disabled = Signal::derive(move || !vm.use_specific_user.get());
    let specific_user_required = Signal::derive(move || {
        needs_specific_user_selection(vm.use_specific_user.get(), &vm.username.get())
    });

    let on_export = {
        move |_| {
            let filters = ExportFilters {
                username: if vm.use_specific_user.get_untracked() {
                    vm.username.get_untracked()
                } else {
                    String::new()
                },
                from_date: vm.from_date.get_untracked(),
                to_date: vm.to_date.get_untracked(),
            };
            if needs_specific_user_selection(
                vm.use_specific_user.get_untracked(),
                &filters.username,
            ) {
                vm.error
                    .set(Some("指定ユーザーを選択してください。".into()));
                vm.preview.set(None);
                return;
            }
            if let Err(msg) = filters.validate() {
                vm.error.set(Some(msg));
                vm.preview.set(None);
                return;
            }
            vm.error.set(None);
            vm.preview.set(None);
            vm.export_action.dispatch(filters);
        }
    };

    let downloading = vm.export_action.pending();

    view! {
        <div class="space-y-6">
            <div class="bg-white shadow rounded-lg p-6">
                <h2 class="text-lg font-medium text-gray-900 mb-4">{"データエクスポート (CSV)"}</h2>
                <div class="grid grid-cols-1 lg:grid-cols-3 gap-3 mb-4">
                    <div>
                        <label class="block text-sm text-gray-700">{"ユーザー"}</label>
                        <div class="mt-1 space-y-2">
                            <div class="flex items-center gap-4 text-sm text-gray-700">
                                <label class="flex items-center gap-1">
                                    <input
                                        type="radio"
                                        name="export_user_filter"
                                        value="all"
                                        checked=move || !vm.use_specific_user.get()
                                        on:change=move |_| {
                                            vm.use_specific_user.set(false);
                                            vm.username.set(String::new());
                                            vm.error.set(None);
                                            vm.preview.set(None);
                                        }
                                    />
                                    {"全ユーザー"}
                                </label>
                                <label class="flex items-center gap-1">
                                    <input
                                        type="radio"
                                        name="export_user_filter"
                                        value="specific"
                                        checked=move || vm.use_specific_user.get()
                                        on:change=move |_| {
                                            vm.use_specific_user.set(true);
                                            vm.error.set(None);
                                            vm.preview.set(None);
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
                                        prop:value={move || vm.username.get()}
                                        on:input=move |ev| vm.username.set(event_target_value(&ev))
                                        disabled=specific_user_disabled
                                    />
                                    <ErrorMessage message={users_error.get().unwrap_or_default()} />
                                </div>
                            </Show>
                            <Show when=move || users_error.get().is_none()>
                                <AdminUserSelect
                                    users=vm.users_resource
                                    selected=vm.username
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
                    <DatePicker
                        label="From"
                        value=vm.from_date
                    />
                    <DatePicker
                        label="To"
                        value=vm.to_date
                    />
                </div>
                <Show when=move || vm.error.get().is_some()>
                    <ErrorMessage message={vm.error.get().unwrap_or_default()} />
                </Show>
                <button
                    class="px-4 py-2 bg-indigo-600 text-white rounded disabled:opacity-50"
                    on:click=on_export
                    disabled=move || downloading.get() || specific_user_required.get()
                >
                    <span class="inline-flex items-center gap-2">
                        <Show when=move || downloading.get()>
                            <span class="h-4 w-4 animate-spin rounded-full border-2 border-white/70 border-t-transparent"></span>
                        </Show>
                        {move || if downloading.get() { "エクスポート中..." } else { "CSVをエクスポート" }}
                    </span>
                </button>
                <Show when=move || vm.preview.get().is_some()>
                    <div class="mt-4">
                        <h3 class="text-sm text-gray-700">
                            {"プレビュー (先頭2KB) - ファイル名: "}
                            {move || vm.filename.get()}
                        </h3>
                        <pre class="mt-2 text-xs bg-gray-50 p-3 rounded overflow-auto max-h-64 whitespace-pre-wrap">
                            {move || vm.preview.get().unwrap_or_default()}
                        </pre>
                    </div>
                </Show>
            </div>
        </div>
    }
}
