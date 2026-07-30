use chrono::Datelike;
use leptos::*;
use timekeeper_contract::admin_attendance_report::AdminAttendanceReportResponse;

use super::repository::AttendanceReportRepository;
use super::view_model::AttendanceReportFilters;
use crate::api::ApiClient;

#[derive(Debug, Clone)]
pub enum AttendanceReportViewState {
    Loading,
    Error,
    Loaded(AdminAttendanceReportResponse),
}

#[component]
pub fn AttendanceReportSection(
    #[prop(default = AttendanceReportViewState::Loading)] state: AttendanceReportViewState,
) -> impl IntoView {
    let body = match state {
        AttendanceReportViewState::Loading => {
            view! { <p data-testid="attendance-report-loading">{rust_i18n::t!("admin_components.attendance_report.loading")}</p> }.into_view()
        }
        AttendanceReportViewState::Error => {
            view! { <p data-testid="attendance-report-error">{rust_i18n::t!("admin_components.attendance_report.error")}</p> }.into_view()
        }
        AttendanceReportViewState::Loaded(response) if response.items.is_empty() => {
            view! { <p data-testid="attendance-report-empty">{rust_i18n::t!("admin_components.attendance_report.empty")}</p> }.into_view()
        }
        AttendanceReportViewState::Loaded(response) => view! {
            <div data-testid="attendance-report-results">
                <p>{format!("{} / {}", response.page, response.total)}</p>
                <ul>
                    {response.items.into_iter().map(|item| view! {
                        <li>{item.user_name}</li>
                    }).collect_view()}
                </ul>
            </div>
        }.into_view(),
    };
    view! {
        <section data-testid="attendance-report">
            <h2>{rust_i18n::t!("admin_components.attendance_report.title")}</h2>
            {body}
        </section>
    }
}

#[component]
pub fn ConnectedAttendanceReportSection() -> impl IntoView {
    let now = chrono::Utc::now().date_naive();
    let repository = AttendanceReportRepository::new(std::rc::Rc::new(
        use_context::<ApiClient>().unwrap_or_else(ApiClient::new),
    ));
    let filters = create_rw_signal(AttendanceReportFilters {
        year: now.year(),
        month: now.month(),
        department_id: None,
        page: 1,
        per_page: 25,
    });
    let draft_month = create_rw_signal(format!("{:04}-{:02}", now.year(), now.month()));
    let draft_department = create_rw_signal(String::new());
    let resource = create_resource(
        move || filters.get().to_query(),
        move |query| {
            let repository = repository.clone();
            async move { repository.fetch(&query).await }
        },
    );
    view! {
        <div data-testid="attendance-report-filters">
            <label>
                {rust_i18n::t!("admin_components.attendance_report.filters.month")}
                <input
                    type="month"
                    data-testid="attendance-report-month"
                    prop:value=move || draft_month.get()
                    on:input=move |event| draft_month.set(event_target_value(&event))
                />
            </label>
            <label>
                {rust_i18n::t!("admin_components.attendance_report.filters.department")}
                <input
                    type="text"
                    data-testid="attendance-report-department"
                    prop:value=move || draft_department.get()
                    on:input=move |event| draft_department.set(event_target_value(&event))
                />
            </label>
            <button data-testid="attendance-report-apply" on:click=move |_| {
                let month = draft_month.get();
                let parsed = month.split_once('-').and_then(|(year, month)| {
                    Some((year.parse::<i32>().ok()?, month.parse::<u32>().ok()?))
                });
                if let Some((year, month)) = parsed {
                    filters.update(|current| {
                        current.year = year;
                        current.month = month;
                        current.department_id = Some(draft_department.get());
                        current.page = 1;
                    });
                }
            }>{rust_i18n::t!("admin_components.attendance_report.filters.apply")}</button>
        </div>
        <Suspense fallback=move || view! { <AttendanceReportSection/> }>
            {move || resource.get().map(|result| match result {
                Ok(response) => {
                    let total = response.total;
                    view! {
                        <AttendanceReportSection state=AttendanceReportViewState::Loaded(response)/>
                        <button data-testid="attendance-report-previous" on:click=move |_| {
                            filters.update(|current| *current = current.previous_page());
                        }>{rust_i18n::t!("admin_components.attendance_report.pagination.previous")}</button>
                        <button data-testid="attendance-report-next" on:click=move |_| {
                            filters.update(|current| *current = current.next_page(total));
                        }>{rust_i18n::t!("admin_components.attendance_report.pagination.next")}</button>
                    }.into_view()
                },
                Err(_) => view! {
                    <AttendanceReportSection state=AttendanceReportViewState::Error/>
                }.into_view(),
            })}
        </Suspense>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::{helpers::set_test_locale, ssr::render_to_string};

    #[test]
    fn renders_loading_error_and_empty_states() {
        let _locale = set_test_locale("ja");
        let loading = render_to_string(|| view! { <AttendanceReportSection/> });
        assert!(loading.contains("attendance-report-loading"));
        let error = render_to_string(|| {
            view! {
                <AttendanceReportSection state=AttendanceReportViewState::Error/>
            }
        });
        assert!(error.contains("attendance-report-error"));
        let empty = render_to_string(|| {
            view! {
                <AttendanceReportSection state=AttendanceReportViewState::Loaded(
                    AdminAttendanceReportResponse {
                        year: 2026, month: 7, page: 1, per_page: 25, total: 0, items: vec![],
                    }
                )/>
            }
        });
        assert!(empty.contains("attendance-report-empty"));
    }

    #[test]
    fn connected_section_renders_month_department_and_apply_controls() {
        let _locale = set_test_locale("ja");
        let html = render_to_string(|| view! { <ConnectedAttendanceReportSection/> });
        assert!(html.contains("attendance-report-month"));
        assert!(html.contains("attendance-report-department"));
        assert!(html.contains("attendance-report-apply"));
    }
}
