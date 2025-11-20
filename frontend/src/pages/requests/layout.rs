use crate::components::layout::Layout;
use leptos::*;

#[component]
pub fn RequestsLayout(children: Children) -> impl IntoView {
    view! {
        <Layout>
            <div class="space-y-6">
                <div>
                    <h1 class="text-2xl font-bold text-gray-900">{"申請管理"}</h1>
                    <p class="mt-1 text-sm text-gray-600">
                        {"休暇と残業申請を作成し、申請状況をまとめて確認できます。"}
                    </p>
                </div>
                {children()}
            </div>
        </Layout>
    }
}
