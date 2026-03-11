use leptos::*;
use leptos_router::{Router, RouterIntegrationContext, ServerIntegration};

use crate::state::locale::LocaleProvider;

pub fn with_runtime<T>(f: impl FnOnce() -> T) -> T {
    let runtime = leptos::create_runtime();
    let result = f();
    runtime.dispose();
    result
}

pub fn with_local_runtime<T>(f: impl FnOnce() -> T) -> T {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, async move { f() })
}

pub fn with_local_runtime_async<F, Fut, T>(f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, f())
}

pub fn render_to_string<F, N>(view: F) -> String
where
    F: FnOnce() -> N + 'static,
    N: IntoView + 'static,
{
    leptos_reactive::suppress_resource_load(true);
    let html = with_runtime(|| view().into_view().render_to_string().to_string());
    leptos_reactive::suppress_resource_load(false);
    html
}

pub fn render_with_router_to_string<F, N>(path: &str, view: F) -> String
where
    F: FnOnce() -> N + 'static,
    N: IntoView + 'static,
{
    let path = path.to_string();
    render_to_string(move || {
        provide_context(RouterIntegrationContext::new(ServerIntegration { path }));
        view! {
            <LocaleProvider>
                <Router>
                    {view()}
                </Router>
            </LocaleProvider>
        }
    })
}
