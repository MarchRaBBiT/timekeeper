use crate::components::layout::Layout;
use leptos::*;

#[component]
pub fn AttendanceFrame(children: Children) -> impl IntoView {
    view! { <Layout>{children()}</Layout> }
}

#[component]
pub fn UnauthorizedMessage() -> impl IntoView {
    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 text-center">
            <p class="text-sm text-fg">{rust_i18n::t!("pages.attendance.unauthorized")}</p>
        </div>
    }
}
