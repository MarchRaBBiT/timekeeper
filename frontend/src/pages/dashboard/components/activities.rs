use crate::{
    components::error::InlineErrorMessage,
    components::layout::LoadingSpinner,
    pages::dashboard::{repository::DashboardActivity, utils::ActivityStatusFilter},
};
use leptos::*;

#[component]
pub fn ActivitiesSection(
    activities: Resource<ActivityStatusFilter, Result<Vec<DashboardActivity>, crate::api::ApiError>>,
) -> impl IntoView {
    view! {
        <div class="bg-white shadow rounded-lg p-6 space-y-4">
            <div>
                <h3 class="text-base font-semibold text-gray-900">{"申請・活動のサマリー"}</h3>
                <p class="text-sm text-gray-600">{"承認待ちの申請数や最新の状態を確認できます"}</p>
            </div>
            {move || match activities.get() {
                None => view! {
                    <div class="flex items-center gap-2 text-sm text-gray-500">
                        <LoadingSpinner />
                        <span>{"最新の申請情報を読み込み中..."}</span>
                    </div>
                }.into_view(),
                Some(Err(err)) => {
                    let error_signal = create_rw_signal(Some(err));
                    view! { <InlineErrorMessage error={error_signal.into()} /> }.into_view()
                }
                Some(Ok(list)) => view! {
                    <ul class="divide-y divide-gray-100">
                        <For
                            each=move || list.clone()
                            key=|item| item.title.clone()
                            children=move |item: DashboardActivity| {
                                view! {
                                    <li class="py-2 flex items-center justify-between text-sm text-gray-800">
                                        <span>{item.title}</span>
                                        <span class="text-gray-600">{item.detail.unwrap_or_else(|| "-".into())}</span>
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
