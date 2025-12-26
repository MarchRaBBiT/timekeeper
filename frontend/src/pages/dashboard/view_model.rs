use crate::api::ApiClient;
use crate::pages::dashboard::{repository, utils::ActivityStatusFilter};
use crate::state::attendance::{refresh_today_context, use_attendance, AttendanceState};
use leptos::*;

#[derive(Clone, Copy)]
pub struct DashboardViewModel {
    pub summary_resource: Resource<(), Result<repository::DashboardSummary, String>>,
    pub alerts_resource: Resource<
        Option<Result<repository::DashboardSummary, String>>,
        Result<Vec<repository::DashboardAlert>, String>,
    >,
    pub activities_resource:
        Resource<ActivityStatusFilter, Result<Vec<repository::DashboardActivity>, String>>,
    pub activity_filter: RwSignal<ActivityStatusFilter>,
    pub attendance_state: (ReadSignal<AttendanceState>, WriteSignal<AttendanceState>),
}

impl DashboardViewModel {
    pub fn new() -> Self {
        let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
        let (attendance_read, attendance_write) = use_attendance();

        let api_clone = api.clone();
        let summary_resource = create_resource(
            || (),
            move |_| {
                let api = api_clone.clone();
                async move { repository::fetch_summary(&api).await }
            },
        );

        let alerts_resource = create_resource(
            move || summary_resource.get(),
            move |summary_opt| async move {
                if let Some(Ok(summary)) = summary_opt {
                    Ok(repository::build_alerts(&summary))
                } else {
                    Ok(Vec::new())
                }
            },
        );

        let activity_filter = create_rw_signal(ActivityStatusFilter::All);
        let api_clone = api.clone();
        let activities_resource = create_resource(
            move || activity_filter.get(),
            move |filter| {
                let api = api_clone.clone();
                async move { repository::fetch_recent_activities(&api, filter).await }
            },
        );

        {
            let api = api.clone();
            create_effect(move |_| {
                let api = api.clone();
                spawn_local(async move {
                    let _ = refresh_today_context(&api, attendance_write).await;
                });
            });
        }

        Self {
            summary_resource,
            alerts_resource,
            activities_resource,
            activity_filter,
            attendance_state: (attendance_read, attendance_write),
        }
    }
}

pub fn use_dashboard_view_model() -> DashboardViewModel {
    match use_context::<DashboardViewModel>() {
        Some(vm) => vm,
        None => {
            let vm = DashboardViewModel::new();
            provide_context(vm.clone());
            vm
        }
    }
}
