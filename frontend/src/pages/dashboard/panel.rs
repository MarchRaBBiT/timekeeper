use super::{
    components::{ActivitiesSection, AlertsSection, GlobalFilters, SummarySection},
    layout::DashboardFrame,
    repository,
};
use crate::{
    components::forms::AttendanceActionButtons,
    state::{
        attendance::{refresh_today_context, use_attendance},
        auth::use_auth,
    },
};
use leptos::*;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let (attendance_state, set_attendance_state) = use_attendance();
    let (auth, _) = use_auth();

    create_effect(move |_| {
        let set_state = set_attendance_state.clone();
        spawn_local(async move {
            let _ = refresh_today_context(set_state).await;
        });
    });

    let summary_resource = create_resource(|| (), |_| async { repository::fetch_summary().await });
    let alerts_resource = create_resource(|| (), |_| async { repository::fetch_alerts().await });
    let activities_resource = create_resource(
        || (),
        |_| async { repository::fetch_recent_activities().await },
    );

    view! {
        <DashboardFrame>
            <div class="space-y-6">
                <header class="space-y-1">
                    <h1 class="text-2xl font-bold text-gray-900">{"ダッシュボード"}</h1>
                    <p class="text-sm text-gray-600">
                        {"勤務状況と申請の概要をまとめて確認できます。下のカードやクイックアクションから操作してください。"}
                    </p>
                </header>

                <GlobalFilters />

                <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <SummarySection summary=summary_resource />
                    <AlertsSection alerts=alerts_resource />
                </div>

                <div class="grid grid-cols-1 gap-6 lg:grid-cols-3">
                    <div class="lg:col-span-2">
                        <ActivitiesSection activities=activities_resource />
                    </div>
                    <div class="bg-white shadow rounded-lg p-6 space-y-4">
                        <div>
                            <h3 class="text-base font-semibold text-gray-900">{"クイック操作"}</h3>
                            <p class="text-sm text-gray-600">{"勤怠打刻やユーザー情報を確認できます"}</p>
                        </div>
                        <div class="space-y-2 text-sm text-gray-700">
                            <div class="flex justify-between">
                                <span class="text-gray-500">{"ユーザー名"}</span>
                                <span class="font-medium text-gray-900">
                                    {move || auth.get().user.as_ref().map(|u| u.username.clone()).unwrap_or_else(|| "-".into())}
                                </span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-gray-500">{"氏名"}</span>
                                <span class="font-medium text-gray-900">
                                    {move || auth.get().user.as_ref().map(|u| u.full_name.clone()).unwrap_or_else(|| "-".into())}
                                </span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-gray-500">{"ロール"}</span>
                                <span class="font-medium text-gray-900">
                                    {move || auth.get().user.as_ref().map(|u| u.role.clone()).unwrap_or_else(|| "-".into())}
                                </span>
                            </div>
                        </div>
                        <div class="border-t pt-4">
                            <AttendanceActionButtons
                                attendance_state=attendance_state
                                set_attendance_state=set_attendance_state
                            />
                        </div>
                    </div>
                </div>
            </div>
        </DashboardFrame>
    }
}
