use crate::{
    components::error::InlineErrorMessage,
    components::layout::LoadingSpinner,
    pages::dashboard::{
        repository::DashboardSummary,
        utils::{format_days, format_hours},
    },
};
use leptos::*;

#[component]
pub fn SummarySection(
    summary: Resource<(), Result<DashboardSummary, crate::api::ApiError>>,
) -> impl IntoView {
    view! {
        <div class="bg-surface-elevated rounded-2xl shadow-sm border border-border p-6 space-y-4">
            <div>
                <h3 class="text-base font-display font-bold text-fg">{rust_i18n::t!("pages.dashboard.summary.title")}</h3>
                <p class="text-sm text-fg-muted">{rust_i18n::t!("pages.dashboard.summary.description")}</p>
            </div>
            <div>
                {move || match summary.get() {
                    None => view! {
                        <div class="flex items-center gap-2 text-sm text-fg-muted">
                            <LoadingSpinner />
                            <span>{rust_i18n::t!("pages.dashboard.summary.loading")}</span>
                        </div>
                    }.into_view(),
                    Some(Err(err)) => {
                        let error_signal = create_rw_signal(Some(err));
                        view! { <InlineErrorMessage error={error_signal.into()} /> }.into_view()
                    }
                    Some(Ok(data)) => view! {
                        <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
                            <Metric label=rust_i18n::t!("pages.dashboard.summary.metrics.total_hours").into_owned() value={format_hours(data.total_work_hours)} />
                            <Metric label=rust_i18n::t!("pages.dashboard.summary.metrics.total_days").into_owned() value={format_days(data.total_work_days)} />
                            <Metric label=rust_i18n::t!("pages.dashboard.summary.metrics.average_hours").into_owned() value={format_hours(data.average_daily_hours)} />
                        </div>
                    }.into_view(),
                }}
            </div>
        </div>
    }
}

#[component]
fn Metric(label: String, value: String) -> impl IntoView {
    view! {
        <div class="relative overflow-hidden p-6 rounded-2xl bg-surface-elevated border border-border shadow-premium hover:shadow-premium-hover transition-all duration-300 group">
            <div class="absolute top-0 right-0 -mr-4 -mt-4 w-24 h-24 bg-primary-subtle rounded-full opacity-50 group-hover:scale-110 transition-transform"></div>
            <p class="relative z-10 text-xs font-display font-bold text-action-primary-bg uppercase tracking-widest">{label}</p>
            <p class="relative z-10 mt-3 text-3xl font-display font-extrabold text-fg">{value}</p>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::pages::dashboard::repository::DashboardSummary;
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn summary_section_renders_metrics() {
        let html = render_to_string(move || {
            let resource = Resource::new(
                || (),
                |_| async move {
                    Ok::<DashboardSummary, crate::api::ApiError>(DashboardSummary {
                        total_work_hours: Some(160.0),
                        total_work_days: Some(20),
                        average_daily_hours: Some(8.0),
                    })
                },
            );
            resource.set(Ok(DashboardSummary {
                total_work_hours: Some(160.0),
                total_work_days: Some(20),
                average_daily_hours: Some(8.0),
            }));
            view! { <SummarySection summary=resource /> }
        });
        assert!(html.contains("160.00"));
    }

    #[test]
    fn summary_section_renders_error() {
        let html = render_to_string(move || {
            let resource = Resource::new(
                || (),
                |_| async move {
                    Ok::<DashboardSummary, crate::api::ApiError>(DashboardSummary {
                        total_work_hours: None,
                        total_work_days: None,
                        average_daily_hours: None,
                    })
                },
            );
            resource.set(Err(crate::api::ApiError::unknown("summary failed")));
            view! { <SummarySection summary=resource /> }
        });
        assert!(html.contains("summary failed"));
    }
}
