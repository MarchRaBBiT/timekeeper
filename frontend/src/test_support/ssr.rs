use leptos::*;

pub fn with_runtime<T>(f: impl FnOnce() -> T) -> T {
    let runtime = leptos::create_runtime();
    let result = f();
    runtime.dispose();
    result
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
