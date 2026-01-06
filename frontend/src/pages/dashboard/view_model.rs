use crate::api::{ApiClient, ApiError};
use crate::pages::dashboard::{repository, utils::ActivityStatusFilter};
use crate::state::attendance::{
    self as attendance_state, refresh_today_context, use_attendance, AttendanceState,
    ClockEventKind, ClockEventPayload,
};
use leptos::{ev::MouseEvent, *};

#[derive(Clone, Copy)]
pub struct DashboardViewModel {
    pub summary_resource: Resource<(), Result<repository::DashboardSummary, ApiError>>,
    pub alerts_resource: Resource<
        Option<Result<repository::DashboardSummary, ApiError>>,
        Result<Vec<repository::DashboardAlert>, ApiError>,
    >,
    pub activities_resource:
        Resource<ActivityStatusFilter, Result<Vec<repository::DashboardActivity>, ApiError>>,
    pub activity_filter: RwSignal<ActivityStatusFilter>,
    pub attendance_state: (ReadSignal<AttendanceState>, WriteSignal<AttendanceState>),
    pub clock_action: Action<ClockEventPayload, Result<(), ApiError>>,
    pub clock_message: RwSignal<Option<ApiError>>,
    pub last_clock_event: RwSignal<Option<ClockEventKind>>,
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

        let api_clone = api.clone();
        let clock_action = create_action(move |payload: &ClockEventPayload| {
            let api = api_clone.clone();
            let set_attendance_state = attendance_write;
            let payload = payload.clone();
            async move {
                match payload.kind {
                    ClockEventKind::ClockIn => {
                        attendance_state::clock_in(&api, set_attendance_state).await?
                    }
                    ClockEventKind::ClockOut => {
                        attendance_state::clock_out(&api, set_attendance_state).await?
                    }
                    ClockEventKind::BreakStart => {
                        let attendance_id = payload
                            .attendance_id
                            .as_deref()
                            .ok_or_else(|| ApiError::unknown("出勤レコードが見つかりません。"))?;
                        attendance_state::start_break(&api, attendance_id).await?
                    }
                    ClockEventKind::BreakEnd => {
                        let break_id = payload
                            .break_id
                            .as_deref()
                            .ok_or_else(|| ApiError::unknown("休憩レコードが見つかりません。"))?;
                        attendance_state::end_break(&api, break_id).await?
                    }
                };
                attendance_state::refresh_today_context(&api, set_attendance_state).await
            }
        });

        let clock_message = create_rw_signal(None);
        let last_clock_event = create_rw_signal(None);

        {
            create_effect(move |_| {
                if let Some(result) = clock_action.value().get() {
                    match result {
                        Ok(_) => {
                            let success = match last_clock_event.get_untracked() {
                                Some(ClockEventKind::ClockIn) => "出勤しました。",
                                Some(ClockEventKind::BreakStart) => "休憩を開始しました。",
                                Some(ClockEventKind::BreakEnd) => "休憩を終了しました。",
                                Some(ClockEventKind::ClockOut) => "退勤しました。",
                                None => "操作が完了しました。",
                            };
                            clock_message.set(Some(ApiError::validation(success)));
                        }
                        Err(err) => clock_message.set(Some(err)),
                    }
                }
            });
        }

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
            clock_action,
            clock_message,
            last_clock_event,
        }
    }

    pub fn handle_clock_in(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            clock_message.set(None);
            last_event.set(Some(ClockEventKind::ClockIn));
            clock_action.dispatch(ClockEventPayload::clock_in());
        }
    }

    pub fn handle_clock_out(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            clock_message.set(None);
            last_event.set(Some(ClockEventKind::ClockOut));
            clock_action.dispatch(ClockEventPayload::clock_out());
        }
    }

    pub fn handle_break_start(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        let (state, _) = self.attendance_state;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            let Some(status) = state.get().today_status.clone() else {
                clock_message.set(Some(ApiError::validation("ステータスを取得できません。")));
                return;
            };
            if status.status != "clocked_in" {
                clock_message.set(Some(ApiError::validation("出勤中のみ休憩を開始できます。")));
                return;
            }
            let Some(att_id) = status.attendance_id.clone() else {
                clock_message.set(Some(ApiError::validation("出勤レコードが見つかりません。")));
                return;
            };
            clock_message.set(None);
            last_event.set(Some(ClockEventKind::BreakStart));
            clock_action.dispatch(ClockEventPayload::break_start(att_id));
        }
    }

    pub fn handle_break_end(&self) -> impl Fn(MouseEvent) {
        let clock_action = self.clock_action;
        let clock_message = self.clock_message;
        let last_event = self.last_clock_event;
        let (state, _) = self.attendance_state;
        move |_| {
            if clock_action.pending().get_untracked() {
                return;
            }
            let Some(status) = state.get().today_status.clone() else {
                clock_message.set(Some(ApiError::validation("ステータスを取得できません。")));
                return;
            };
            if status.status != "on_break" {
                clock_message.set(Some(ApiError::validation("休憩中のみ休憩を終了できます。")));
                return;
            }
            let Some(break_id) = status.active_break_id.clone() else {
                clock_message.set(Some(ApiError::validation("休憩レコードが見つかりません。")));
                return;
            };
            clock_message.set(None);
            last_event.set(Some(ClockEventKind::BreakEnd));
            clock_action.dispatch(ClockEventPayload::break_end(break_id));
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
