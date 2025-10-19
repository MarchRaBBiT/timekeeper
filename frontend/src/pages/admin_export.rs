use crate::api::ApiClient;
use crate::components::layout::*;
use crate::utils::trigger_csv_download;
use leptos::*;

#[component]
pub fn AdminExportPage() -> impl IntoView {
    let downloading = create_rw_signal(false);
    let error = create_rw_signal(Option::<String>::None);
    let preview = create_rw_signal(Option::<String>::None);
    let filename_state = create_rw_signal(String::new());
    let username = create_rw_signal(String::new());
    let from_date = create_rw_signal(String::new());
    let to_date = create_rw_signal(String::new());

    let on_export = move |_| {
        error.set(None);
        preview.set(None);
        downloading.set(true);
        leptos::spawn_local(async move {
            let api = ApiClient::new();
            let u = username.get();
            let f = from_date.get();
            let t = to_date.get();
            let u_opt = if u.is_empty() { None } else { Some(u.as_str()) };
            let f_opt = if f.is_empty() { None } else { Some(f.as_str()) };
            let t_opt = if t.is_empty() { None } else { Some(t.as_str()) };
            match api.export_data_filtered(u_opt, f_opt, t_opt).await {
                Ok(v) => {
                    let fname = v
                        .get("filename")
                        .and_then(|s| s.as_str())
                        .unwrap_or("export.csv");
                    let csv = v.get("csv_data").and_then(|c| c.as_str()).unwrap_or("");
                    filename_state.set(fname.to_string());
                    preview.set(Some(csv.chars().take(2000).collect()));
                    let _ = trigger_csv_download(fname, csv);
                    downloading.set(false);
                }
                Err(e) => {
                    error.set(Some(e));
                    downloading.set(false);
                }
            }
        });
    };

    view! {
        <Layout>
            <div class="space-y-6">
                <div class="bg-white shadow rounded-lg p-6">
                    <h2 class="text-lg font-medium text-gray-900 mb-4">{"データエクスポート (CSV)"}</h2>
                    <div class="grid grid-cols-1 md:grid-cols-3 gap-3 mb-4">
                        <div>
                            <label class="block text-sm text-gray-700">{"ユーザー名 (任意)"}</label>
                            <input class="mt-1 w-full border rounded px-2 py-1" placeholder="username" on:input=move |ev| username.set(event_target_value(&ev)) />
                        </div>
                        <div>
                            <label class="block text-sm text-gray-700">{"From (YYYY-MM-DD)"}</label>
                            <input type="date" class="mt-1 w-full border rounded px-2 py-1" on:input=move |ev| from_date.set(event_target_value(&ev)) />
                        </div>
                        <div>
                            <label class="block text-sm text-gray-700">{"To (YYYY-MM-DD)"}</label>
                            <input type="date" class="mt-1 w-full border rounded px-2 py-1" on:input=move |ev| to_date.set(event_target_value(&ev)) />
                        </div>
                    </div>
                    <Show when=move || error.get().is_some()>
                        <ErrorMessage message={error.get().unwrap_or_default()} />
                    </Show>
                    <button class="px-4 py-2 bg-indigo-600 text-white rounded disabled:opacity-50" on:click=on_export disabled=downloading.get()>
                        {move || if downloading.get() { "エクスポート中..." } else { "CSVをエクスポート" }}
                    </button>
                    <Show when=move || preview.get().is_some()>
                        <div class="mt-4">
                            <h3 class="text-sm text-gray-700">{"プレビュー (先頭2KB) - ファイル名: "}{move || filename_state.get()}</h3>
                            <pre class="mt-2 text-xs bg-gray-50 p-3 rounded overflow-auto max-h-64 whitespace-pre-wrap">{move || preview.get().unwrap_or_default()}</pre>
                        </div>
                    </Show>
                </div>
            </div>
        </Layout>
    }
}
