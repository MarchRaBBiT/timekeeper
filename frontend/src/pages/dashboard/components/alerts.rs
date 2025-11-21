use crate::{
    components::layout::{ErrorMessage, LoadingSpinner},
    pages::dashboard::repository::{DashboardAlert, DashboardAlertLevel, DashboardSummary},
};
use leptos::*;

#[component]
pub fn AlertsSection(
    alerts: Resource<Option<Result<DashboardSummary, String>>, Result<Vec<DashboardAlert>, String>>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-2">
                <h3 class="text-base font-semibold text-gray-900">{"アラート"}</h3>
                <p class="text-sm text-gray-600">{"勤務や申請に関する注意事項"}</p>
            </div>
            {move || match alerts.get() {
                None => view! {
                    <div class="flex items-center gap-2 text-sm text-gray-500">
                        <LoadingSpinner />
                        <span>{"アラート情報を読み込み中..."}</span>
                    </div>
                }.into_view(),
                Some(Err(err)) => view! { <ErrorMessage message={err.clone()} /> }.into_view(),
                Some(Ok(list)) => view! {
                    <ul class="space-y-2">
                        <For
                            each=move || list.clone()
                            key=|alert| alert.message.clone()
                            children=move |alert: DashboardAlert| {
                                let badge = render_badge(&alert.level);
                                view! {
                                    <li class="flex items-start gap-2 text-sm text-gray-800">
                                        {badge}
                                        <span>{alert.message}</span>
                                    </li>
                                }
                            }
                        />
                    </ul>
                }.into_view(),
            }}
        </div>
    }
}

fn render_badge(level: &DashboardAlertLevel) -> View {
    let (color, text) = match level {
        DashboardAlertLevel::Info => ("bg-blue-100 text-blue-800", "INFO"),
        DashboardAlertLevel::Warning => ("bg-amber-100 text-amber-800", "WARN"),
        DashboardAlertLevel::Error => ("bg-red-100 text-red-800", "ERROR"),
    };
    view! {
        <span class=format!("px-2 py-0.5 rounded-full text-xs font-semibold {}", color)>
            {text}
        </span>
    }
    .into_view()
}
