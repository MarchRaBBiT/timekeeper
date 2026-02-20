use crate::{
    components::error::InlineErrorMessage,
    components::layout::LoadingSpinner,
    pages::dashboard::repository::{DashboardAlert, DashboardAlertLevel, DashboardSummary},
};
use leptos::*;

type AlertsResource = Resource<
    Option<Result<DashboardSummary, crate::api::ApiError>>,
    Result<Vec<DashboardAlert>, crate::api::ApiError>,
>;

#[component]
pub fn AlertsSection(alerts: AlertsResource) -> impl IntoView {
    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <div class="flex flex-col gap-2">
                <h3 class="text-base font-semibold text-fg">{"アラート"}</h3>
                <p class="text-sm text-fg-muted">{"勤務や申請に関する注意事項"}</p>
            </div>
            {move || match alerts.get() {
                None => view! {
                    <div class="flex items-center gap-2 text-sm text-fg-muted">
                        <LoadingSpinner />
                        <span>{"アラート情報を読み込み中..."}</span>
                    </div>
                }.into_view(),
                Some(Err(err)) => {
                    let error_signal = create_rw_signal(Some(err));
                    view! { <InlineErrorMessage error={error_signal.into()} /> }.into_view()
                }
                Some(Ok(list)) => view! {
                    <ul class="space-y-2">
                        <For
                            each=move || list.clone()
                            key=|alert| alert.message.clone()
                            children=move |alert: DashboardAlert| {
                                let badge = render_badge(&alert.level);
                                view! {
                                    <li class="flex items-start gap-2 text-sm text-fg">
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
        DashboardAlertLevel::Info => ("bg-status-info-bg text-status-info-text", "情報"),
        DashboardAlertLevel::Warning => ("bg-status-warning-bg text-status-warning-text", "警告"),
        DashboardAlertLevel::Error => ("bg-status-error-bg text-status-error-text", "エラー"),
    };
    view! {
        <span class=format!("px-2 py-0.5 rounded-full text-xs font-semibold {}", color)>
            {text}
        </span>
    }
    .into_view()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::pages::dashboard::repository::{DashboardAlert, DashboardAlertLevel};
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn alerts_section_renders_alerts() {
        let html = render_to_string(move || {
            let resource = Resource::new(
                || {
                    Some(Ok(DashboardSummary {
                        total_work_hours: None,
                        total_work_days: None,
                        average_daily_hours: None,
                    }))
                },
                |_| async move { Ok::<Vec<DashboardAlert>, crate::api::ApiError>(Vec::new()) },
            );
            resource.set(Ok(vec![DashboardAlert {
                level: DashboardAlertLevel::Warning,
                message: "注意".into(),
            }]));
            view! { <AlertsSection alerts=resource /> }
        });
        assert!(html.contains("アラート"));
        assert!(html.contains("注意"));
        assert!(html.contains("警告"));
    }

    #[test]
    fn alerts_section_renders_error() {
        let html = render_to_string(move || {
            let resource = Resource::new(
                || {
                    Some(Ok(DashboardSummary {
                        total_work_hours: None,
                        total_work_days: None,
                        average_daily_hours: None,
                    }))
                },
                |_| async move { Ok::<Vec<DashboardAlert>, crate::api::ApiError>(Vec::new()) },
            );
            resource.set(Err(crate::api::ApiError::unknown("alert failed")));
            view! { <AlertsSection alerts=resource /> }
        });
        assert!(html.contains("alert failed"));
    }
}
