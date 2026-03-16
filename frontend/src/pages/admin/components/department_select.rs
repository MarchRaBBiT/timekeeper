use crate::{
    api::{ApiError, DepartmentResponse},
    components::{error::InlineErrorMessage, layout::LoadingSpinner},
};
use leptos::{ev, *};

pub type DepartmentsResource = Resource<bool, Result<Vec<DepartmentResponse>, ApiError>>;

#[component]
pub fn AdminDepartmentSelect(
    departments: DepartmentsResource,
    selected: RwSignal<String>,
    #[prop(optional_no_strip)] label: Option<String>,
    #[prop(default = rust_i18n::t!("admin_components.department_select.placeholder").into_owned())]
    placeholder: String,
    #[prop(into, default = MaybeSignal::Static(false))] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    let fetch_error = create_rw_signal(None::<ApiError>);
    let loading = departments.loading();

    {
        create_effect(move |_| {
            if let Some(result) = departments.get() {
                match result {
                    Ok(_) => fetch_error.set(None),
                    Err(err) => fetch_error.set(Some(err)),
                }
            }
        });
    }

    let on_change = {
        move |ev: ev::Event| {
            selected.set(event_target_value(&ev));
        }
    };

    let has_label = label.as_ref().map(|l| !l.is_empty()).unwrap_or(false);
    let label_value = label.unwrap_or_default();

    let on_retry = {
        move |_| {
            fetch_error.set(None);
            departments.refetch();
        }
    };

    let options_view = move || {
        if loading.get() {
            return view! { <option value="" disabled>{rust_i18n::t!("admin_components.department_select.loading")}</option> }
                .into_view();
        }
        match departments.get() {
            None => {
                view! { <option value="" disabled>{rust_i18n::t!("admin_components.department_select.loading")}</option> }.into_view()
            }
            Some(Err(_)) => {
                view! { <option value="" disabled>{rust_i18n::t!("admin_components.department_select.fetch_failed")}</option> }
                    .into_view()
            }
            Some(Ok(list)) if list.is_empty() => {
                view! { <option value="" disabled>{rust_i18n::t!("admin_components.department_select.empty")}</option> }.into_view()
            }
            Some(Ok(list)) => {
                let mut sorted = list.clone();
                sorted.sort_by(|a, b| a.name.cmp(&b.name));
                view! {
                    <For
                        each=move || sorted.clone()
                        key=|dept| dept.id.clone()
                        children=move |dept| {
                            view! { <option value=dept.id.clone()>{dept.name.clone()}</option> }
                        }
                    />
                }
                .into_view()
            }
        }
    };

    view! {
        <div class="space-y-1">
            <Show when=move || has_label>
                <label class="block text-sm font-medium text-fg-muted">
                    {label_value.clone()}
                </label>
            </Show>
            <select
                class="w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1 disabled:opacity-50"
                on:change=on_change
                prop:value=selected
                disabled=disabled
            >
                <option value="">{placeholder.clone()}</option>
                {move || options_view()}
            </select>
            <Show when=move || fetch_error.get().is_some()>
                <div class="flex items-center gap-2">
                    <InlineErrorMessage error={fetch_error.into()} />
                    <button
                        class="text-sm text-link hover:text-link-hover hover:underline disabled:opacity-50"
                        on:click=on_retry
                        disabled=move || loading.get()
                    >
                        {rust_i18n::t!("common.actions.retry")}
                    </button>
                    <Show when=move || loading.get()>
                        <LoadingSpinner />
                    </Show>
                </div>
            </Show>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::DepartmentResponse;
    use crate::test_support::{helpers::set_test_locale, ssr::render_to_string};
    use chrono::Utc;

    fn dept(id: &str, name: &str) -> DepartmentResponse {
        DepartmentResponse {
            id: id.into(),
            name: name.into(),
            parent_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn department_select_renders_empty_state() {
        let _locale = set_test_locale("en");
        let html = render_to_string(move || {
            let departments = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            departments.set(Ok(Vec::new()));
            let selected = create_rw_signal(String::new());
            view! { <AdminDepartmentSelect departments=departments selected=selected /> }
        });
        assert!(html.contains("No departments found"));
    }

    #[test]
    fn department_select_renders_error_state() {
        let _locale = set_test_locale("en");
        let html = render_to_string(move || {
            let departments = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            departments.set(Err(ApiError::validation("fetch error")));
            let selected = create_rw_signal(String::new());
            view! { <AdminDepartmentSelect departments=departments selected=selected /> }
        });
        assert!(html.contains("Failed to load departments"));
    }

    #[test]
    fn department_select_renders_options() {
        let html = render_to_string(move || {
            let departments = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            departments.set(Ok(vec![dept("d1", "Engineering"), dept("d2", "Sales")]));
            let selected = create_rw_signal(String::new());
            view! { <AdminDepartmentSelect departments=departments selected=selected /> }
        });
        assert!(html.contains("Engineering"));
        assert!(html.contains("Sales"));
    }
}
