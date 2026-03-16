use crate::{
    api::{ApiError, CreateUser, UserResponse},
    components::{error::InlineErrorMessage, layout::SuccessMessage},
    pages::admin::components::department_select::{AdminDepartmentSelect, DepartmentsResource},
    pages::admin_users::utils::{localized_role_label, InviteFormState, MessageState},
};
use leptos::{ev, *};
use wasm_bindgen::JsCast;

fn prepare_invite_submission(
    is_system_admin: bool,
    form_state: InviteFormState,
) -> Result<CreateUser, ApiError> {
    if !is_system_admin {
        return Err(ApiError::unknown(
            rust_i18n::t!("pages.admin_users.invite_form.errors.system_admin_only").to_string(),
        ));
    }
    if !form_state.is_valid() {
        return Err(ApiError::validation(
            rust_i18n::t!("pages.admin_users.invite_form.errors.required_fields").to_string(),
        ));
    }
    Ok(form_state.to_request())
}

#[component]
pub fn InviteForm(
    form_state: InviteFormState,
    messages: MessageState,
    invite_action: Action<CreateUser, Result<UserResponse, ApiError>>,
    is_system_admin: Memo<bool>,
    departments: DepartmentsResource,
) -> impl IntoView {
    let pending = invite_action.pending();
    let on_submit = {
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            messages.clear();
            match prepare_invite_submission(is_system_admin.get_untracked(), form_state) {
                Ok(payload) => invite_action.dispatch(payload),
                Err(err) => messages.set_error(err),
            }
        }
    };

    view! {
        <div class="bg-surface-elevated shadow rounded-lg p-6 space-y-4">
            <div>
                <h2 class="text-lg font-medium text-fg">
                    {rust_i18n::t!("pages.admin_users.invite_form.title")}
                </h2>
                <p class="text-sm text-fg-muted">
                    {rust_i18n::t!("pages.admin_users.invite_form.description")}
                </p>
            </div>

            <Show when=move || messages.error.get().is_some()>
                <InlineErrorMessage error={messages.error.into()} />
            </Show>
            <Show when=move || messages.success.get().is_some()>
                <SuccessMessage message={messages.success.get().unwrap_or_default()} />
            </Show>

            <form class="grid grid-cols-1 lg:grid-cols-2 gap-4" on:submit=on_submit>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">
                        {rust_i18n::t!("pages.admin_users.invite_form.fields.username")}
                    </label>
                    <input
                        class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                        placeholder={rust_i18n::t!(
                            "pages.admin_users.invite_form.placeholders.username"
                        )}
                        prop:value=form_state.username
                        on:input=move |ev| form_state.username.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">
                        {rust_i18n::t!("pages.admin_users.invite_form.fields.full_name")}
                    </label>
                    <input
                        class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                        placeholder={rust_i18n::t!(
                            "pages.admin_users.invite_form.placeholders.full_name"
                        )}
                        prop:value=form_state.full_name
                        on:input=move |ev| form_state.full_name.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">
                        {rust_i18n::t!("pages.admin_users.invite_form.fields.email")}
                    </label>
                    <input
                        type="email"
                        class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                        placeholder={rust_i18n::t!(
                            "pages.admin_users.invite_form.placeholders.email"
                        )}
                        prop:value=form_state.email
                        on:input=move |ev| form_state.email.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">
                        {rust_i18n::t!("pages.admin_users.invite_form.fields.password")}
                    </label>
                    <input
                        type="password"
                        class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                        prop:value=form_state.password
                        on:input=move |ev| form_state.password.set(event_target_value(&ev))
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">
                        {rust_i18n::t!("pages.admin_users.invite_form.fields.role")}
                    </label>
                    <select
                        class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-2 py-1"
                        prop:value=form_state.role
                        on:change=move |ev| form_state.role.set(event_target_value(&ev))
                    >
                        <option value="employee">{localized_role_label("employee")}</option>
                        <option value="manager">{localized_role_label("manager")}</option>
                    </select>
                </div>
                <div>
                    <label class="block text-sm font-medium text-fg-muted">
                        {rust_i18n::t!("pages.admin_users.invite_form.fields.department")}
                    </label>
                    <AdminDepartmentSelect
                        departments=departments
                        selected=form_state.department_id
                        disabled=pending
                    />
                </div>
                <div class="flex items-center space-x-2 h-full pt-6">
                    <input
                        type="checkbox"
                        class="h-4 w-4 text-action-primary-bg border-form-control-border rounded"
                        prop:checked=form_state.is_system_admin
                        on:change=move |ev| {
                            if let Some(target) =
                                ev.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                            {
                                form_state.is_system_admin.set(target.checked());
                            }
                        }
                    />
                    <span class="text-sm text-fg">
                        {rust_i18n::t!("pages.admin_users.invite_form.fields.system_admin")}
                    </span>
                </div>
                <div class="lg:col-span-2">
                    <button
                        type="submit"
                        disabled=move || pending.get()
                        class="px-4 py-2 bg-action-primary-bg text-action-primary-text rounded disabled:opacity-50"
                    >
                        {move || {
                            if pending.get() {
                                rust_i18n::t!("pages.admin_users.invite_form.actions.submitting")
                            } else {
                                rust_i18n::t!("pages.admin_users.invite_form.actions.submit")
                            }
                        }}
                    </button>
                </div>
            </form>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::{helpers::set_test_locale, ssr::render_to_string};

    #[test]
    fn invite_form_renders() {
        let _locale = set_test_locale("ja");
        let html = render_to_string(move || {
            let form_state = InviteFormState::default();
            let messages = MessageState::default();
            let invite_action =
                create_action(|_: &CreateUser| async move { Err(ApiError::validation("failed")) });
            let is_system_admin = create_memo(|_| true);
            let departments = Resource::new(|| true, |_| async move { Ok(Vec::new()) });
            view! {
                <InviteForm
                    form_state=form_state
                    messages=messages
                    invite_action=invite_action
                    is_system_admin=is_system_admin
                    departments=departments
                />
            }
        });
        assert!(html.contains(rust_i18n::t!("pages.admin_users.invite_form.title").as_ref()));
        assert!(
            html.contains(rust_i18n::t!("pages.admin_users.invite_form.actions.submit").as_ref())
        );
    }

    #[test]
    fn prepare_invite_submission_validates_permissions_and_required_fields() {
        let _locale = set_test_locale("ja");
        let runtime = create_runtime();
        let form_state = InviteFormState::default();

        let non_admin_err =
            prepare_invite_submission(false, form_state).expect_err("must require admin");
        assert_eq!(
            non_admin_err.error,
            rust_i18n::t!("pages.admin_users.invite_form.errors.system_admin_only")
        );

        let form_state = InviteFormState::default();
        let invalid_err =
            prepare_invite_submission(true, form_state).expect_err("must require all fields");
        assert_eq!(invalid_err.code, "VALIDATION_ERROR");

        let form_state = InviteFormState::default();
        form_state.username.set("alice".into());
        form_state.full_name.set("Alice Example".into());
        form_state.email.set("alice@example.com".into());
        form_state.password.set("Password123".into());
        form_state.role.set("admin".into());
        form_state.is_system_admin.set(true);

        let payload =
            prepare_invite_submission(true, form_state).expect("valid payload should be returned");
        assert_eq!(payload.username, "alice");
        assert_eq!(payload.full_name, "Alice Example");
        assert_eq!(payload.email, "alice@example.com");
        assert_eq!(payload.role, "admin");
        assert!(payload.is_system_admin);

        runtime.dispose();
    }
}
