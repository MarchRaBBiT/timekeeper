use crate::components::layout::{ErrorMessage, LoadingSpinner};
use crate::pages::requests::types::{RequestKind, RequestSummary};
use leptos::*;

#[component]
pub fn RequestsList(
    summaries: Signal<Vec<RequestSummary>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    on_select: Callback<RequestSummary>,
    on_edit: Callback<RequestSummary>,
    on_cancel: Callback<RequestSummary>,
    message: RwSignal<crate::pages::requests::utils::MessageState>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow-premium rounded-3xl overflow-hidden border border-gray-100/50">
            <div class="px-8 py-6 border-b border-gray-100">
                <h3 class="text-xl font-display font-bold text-slate-900">{"申請一覧"}</h3>
                <Show when=move || message.get().error.is_some()>
                    <div class="mt-4">
                        <ErrorMessage message={message.get().error.clone().unwrap_or_default()} />
                    </div>
                </Show>
                <Show when=move || message.get().success.is_some()>
                    <div class="mt-4 flex items-center gap-2 p-3 rounded-xl bg-brand-50 border border-brand-100 text-brand-700 animate-pop-in">
                        <i class="fas fa-check-circle"></i>
                        <span class="text-sm font-medium">{message.get().success.clone().unwrap_or_default()}</span>
                    </div>
                </Show>
            </div>
            
            <Show when=move || error.get().is_some()>
                <div class="p-8">
                    <ErrorMessage message={error.get().unwrap_or_default()} />
                </div>
            </Show>
            
            <Show when=move || loading.get()>
                <div class="p-12 flex flex-col items-center justify-center gap-4 text-slate-400">
                    <LoadingSpinner />
                    <span class="text-sm font-medium tracking-widest uppercase">{"データを取得中..."}</span>
                </div>
            </Show>
            
            <Show when=move || !loading.get() && summaries.get().is_empty() && error.get().is_none()>
                <div class="p-16 text-center">
                    <div class="w-16 h-16 bg-slate-50 rounded-full flex items-center justify-center mx-auto mb-4 text-slate-300">
                        <i class="fas fa-inbox text-2xl"></i>
                    </div>
                    <p class="text-slate-500 font-medium font-sans">{"表示できる申請がありません"} </p>
                    <p class="text-xs text-slate-400 mt-1">{"左または上のフォームから新しい申請を送信できます"}</p>
                </div>
            </Show>
            
            <Show when=move || !summaries.get().is_empty()>
                <div class="overflow-x-auto">
                    <table class="min-w-full divide-y divide-gray-100">
                        <thead>
                            <tr class="bg-slate-50/50">
                                <th class="px-8 py-4 text-left text-xs font-bold text-slate-400 uppercase tracking-widest">{"種類"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-slate-400 uppercase tracking-widest">{"期間 / 日付"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-slate-400 uppercase tracking-widest">{"補足"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-slate-400 uppercase tracking-widest">{"ステータス"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-slate-400 uppercase tracking-widest">{"提出日"}</th>
                                <th class="px-8 py-4 text-left text-xs font-bold text-slate-400 uppercase tracking-widest">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-50 bg-white">
                            <For
                                each=move || summaries.get()
                                key=|summary| summary.id.clone()
                                children=move |summary: RequestSummary| {
                                    let summary = store_value(summary);
                                    let summary_value = summary.get_value();
                                    let status = summary_value.status.clone();
                                    let status_for_pending = status.clone();
                                    let status_for_not_pending = status.clone();
                                    let primary_label =
                                        summary_value.primary_label.clone().unwrap_or_else(|| "-".into());
                                    let secondary_label =
                                        summary_value.secondary_label.clone().unwrap_or_else(|| "-".into());
                                    let submitted_at =
                                        summary_value.submitted_at.clone().unwrap_or_else(|| "-".into());
                                    
                                    let status_style = match status.as_str() {
                                        "approved" => "bg-green-100 text-green-700",
                                        "rejected" => "bg-red-100 text-red-700",
                                        "pending" => "bg-brand-100 text-brand-700",
                                        _ => "bg-slate-100 text-slate-700",
                                    };

                                    view! {
                                        <tr class="hover:bg-slate-50/80 transition-colors group cursor-pointer" on:click=move |_| on_select.call(summary.get_value())>
                                            <td class="px-8 py-5 whitespace-nowrap">
                                                <span class="inline-flex items-center px-2 py-0.5 rounded-md text-[10px] font-bold bg-slate-100 text-slate-600 uppercase">
                                                    {match summary_value.kind {
                                                        RequestKind::Leave => "休暇",
                                                        RequestKind::Overtime => "残業",
                                                    }}
                                                </span>
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap text-sm font-bold text-slate-900">
                                                {primary_label.clone()}
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap text-sm text-slate-500">
                                                {secondary_label.clone()}
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap">
                                                <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-[10px] font-black uppercase tracking-wider {}", status_style)>
                                                    {status.clone()}
                                                </span>
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap text-sm font-medium text-slate-400">
                                                {submitted_at.clone()}
                                            </td>
                                            <td class="px-8 py-5 whitespace-nowrap text-sm text-gray-900">
                                                <Show when=move || status_for_pending == "pending">
                                                    <div class="flex gap-4">
                                                        <button
                                                            class="text-brand-600 hover:text-brand-700 font-bold flex items-center gap-1 transition-colors"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                on_edit.call(summary.get_value());
                                                            }
                                                        >
                                                            <i class="fas fa-edit text-xs"></i>
                                                            {"編集"}
                                                        </button>
                                                        <button
                                                            class="text-red-500 hover:text-red-600 font-bold flex items-center gap-1 transition-colors"
                                                            on:click=move |ev| {
                                                                ev.stop_propagation();
                                                                on_cancel.call(summary.get_value());
                                                            }
                                                        >
                                                            <i class="fas fa-times-circle text-xs"></i>
                                                            {"取消"}
                                                        </button>
                                                    </div>
                                                </Show>
                                                <Show when=move || status_for_not_pending != "pending">
                                                    <span class="text-slate-300">
                                                        <i class="fas fa-lock text-xs"></i>
                                                    </span>
                                                </Show>
                                            </td>
                                        </tr>
                                    }
                                }
                            />
                        </tbody>
                    </table>
                </div>
            </Show>
        </div>
    }
}
