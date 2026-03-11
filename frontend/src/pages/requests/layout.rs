use crate::components::layout::Layout;
use leptos::*;

#[component]
pub fn RequestsLayout(children: Children) -> impl IntoView {
    view! {
        <Layout>
            <div class="space-y-6">
                <div>
                    <h1 class="text-2xl font-bold text-fg">{rust_i18n::t!("pages.requests.layout.title")}</h1>
                    <p class="mt-1 text-sm text-fg-muted">
                        {rust_i18n::t!("pages.requests.layout.description")}
                    </p>
                </div>
                {children()}
            </div>
        </Layout>
    }
}
