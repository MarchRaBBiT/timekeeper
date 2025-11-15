use crate::{
    components::{cards::*, forms::*, layout::*},
    state::attendance::{refresh_today_context, use_attendance},
};
use leptos::*;
use log::error;

#[component]
pub fn DashboardPage() -> impl IntoView {
    let (attendance_state, set_attendance_state) = use_attendance();

    create_effect(move |_| {
        let set_state = set_attendance_state.clone();
        spawn_local(async move {
            if let Err(err) = refresh_today_context(set_state).await {
                error!("Failed to refresh attendance context: {}", err);
            }
        });
    });

    view! {
        <Layout>
            <div class="space-y-6">
                <div>
                    <h1 class="text-2xl font-bold text-gray-900">{"ダッシュボード"}</h1>
                    <p class="mt-1 text-sm text-gray-600">{"最新の勤怠状況と活動サマリーを確認できます"}</p>
                </div>

                <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <AttendanceCard
                        attendance_state=attendance_state
                        set_attendance_state=set_attendance_state
                    />
                    <SummaryCard/>
                </div>

                <div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
                    <RequestCard/>
                    <UserCard/>
                </div>

                <div class="bg-white shadow rounded-lg p-6">
                    <h3 class="text-lg font-medium text-gray-900 mb-4">{"勤怠操作"}</h3>
                    <AttendanceActionButtons
                        attendance_state=attendance_state
                        set_attendance_state=set_attendance_state
                    />
                </div>
            </div>
        </Layout>
    }
}
