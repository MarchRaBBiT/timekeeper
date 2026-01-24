use crate::{
    components::error::InlineErrorMessage,
    components::layout::LoadingSpinner,
    pages::dashboard::{repository::DashboardActivity, utils::ActivityStatusFilter},
};
use leptos::*;

#[component]
pub fn ActivitiesSection(
    activities: Resource<
        ActivityStatusFilter,
        Result<Vec<DashboardActivity>, crate::api::ApiError>,
    >,
) -> impl IntoView {
    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <div>
                <h3 class="text-base font-semibold text-fg">{"申請・活動のサマリー"}</h3>
                <p class="text-sm text-fg-muted">{"承認待ちの申請数や最新の状態を確認できます"}</p>
            </div>
            {move || match activities.get() {
                None => view! {
                    <div class="flex items-center gap-2 text-sm text-fg-muted">
                        <LoadingSpinner />
                        <span>{"最新の申請情報を読み込み中..."}</span>
                    </div>
                }.into_view(),
                Some(Err(err)) => {
                    let error_signal = create_rw_signal(Some(err));
                    view! { <InlineErrorMessage error={error_signal.into()} /> }.into_view()
                }
                Some(Ok(list)) => view! {
                    <ul class="divide-y divide-border">
                        <For
                            each=move || list.clone()
                            key=|item| item.title.clone()
                            children=move |item: DashboardActivity| {
                                view! {
                                    <li class="py-2 flex items-center justify-between text-sm text-fg">
                                        <span>{item.title}</span>
                                        <span class="text-fg-muted">{item.detail.unwrap_or_else(|| "-".into())}</span>
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
