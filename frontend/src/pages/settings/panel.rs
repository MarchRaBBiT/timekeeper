use crate::{
    api::{
        ApiError, CreateDataSubjectRequest, DataSubjectRequestResponse, DataSubjectRequestType,
        SessionResponse,
    },
    components::{
        error::InlineErrorMessage,
        layout::{ErrorMessage, Layout, SuccessMessage},
    },
    pages::{
        mfa::{
            components::{setup::SetupSection, verify::VerificationSection},
            utils,
        },
        settings::view_model::use_settings_view_model,
    },
    state::auth::use_auth,
};
use chrono::{DateTime, Utc};
use leptos::{ev::SubmitEvent, Callback, *};

const PASSWORD_POLICY_MIN_LENGTH: usize = 12;
const COMMON_WEAK_PASSWORDS: &[&str] = &[
    "123456",
    "12345678",
    "123456789",
    "1234567890",
    "admin",
    "admin123",
    "changeme",
    "iloveyou",
    "letmein",
    "passw0rd",
    "password",
    "password1",
    "password123",
    "qwerty",
    "qwerty123",
    "welcome",
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SettingsTab {
    Password,
    Mfa,
    Sessions,
    SubjectRequest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PasswordStrengthState {
    label_key: &'static str,
    badge_class: &'static str,
    hint: String,
}

fn tr(key: &'static str) -> String {
    rust_i18n::t!(key).into_owned()
}

fn map_change_password_error(error: &ApiError) -> ApiError {
    let message = error.error.as_str();
    if message == "Current password is incorrect" {
        ApiError::validation(tr("pages.settings.password.errors.current_incorrect"))
    } else if message.contains("at least") && message.contains("characters") {
        ApiError::validation(tr("pages.settings.password.errors.min_length"))
    } else if message.contains("uppercase") {
        ApiError::validation(tr("pages.settings.password.errors.uppercase"))
    } else if message.contains("lowercase") {
        ApiError::validation(tr("pages.settings.password.errors.lowercase"))
    } else if message.contains("number") {
        ApiError::validation(tr("pages.settings.password.errors.number"))
    } else if message.contains("symbol") {
        ApiError::validation(tr("pages.settings.password.errors.symbol"))
    } else if message.contains("too common") {
        ApiError::validation(tr("pages.settings.password.errors.too_common"))
    } else if message == "New password must differ from current password" {
        ApiError::validation(tr("pages.settings.password.errors.must_differ"))
    } else {
        ApiError::unknown(tr("pages.settings.password.errors.update_failed"))
    }
}

fn collect_password_policy_gaps(password: &str) -> Vec<&'static str> {
    let mut gaps = Vec::new();
    if password.len() < PASSWORD_POLICY_MIN_LENGTH {
        gaps.push("pages.settings.password.policy.requirements.min_length");
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        gaps.push("pages.settings.password.policy.requirements.uppercase");
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        gaps.push("pages.settings.password.policy.requirements.lowercase");
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        gaps.push("pages.settings.password.policy.requirements.number");
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        gaps.push("pages.settings.password.policy.requirements.symbol");
    }
    if is_common_weak_password(password) {
        gaps.push("pages.settings.password.policy.requirements.avoid_common");
    }
    gaps
}

fn first_password_policy_error(password: &str) -> Option<ApiError> {
    if password.len() < PASSWORD_POLICY_MIN_LENGTH {
        Some(ApiError::validation(tr(
            "pages.settings.password.errors.min_length",
        )))
    } else if !password.chars().any(|c| c.is_uppercase()) {
        Some(ApiError::validation(tr(
            "pages.settings.password.errors.uppercase",
        )))
    } else if !password.chars().any(|c| c.is_lowercase()) {
        Some(ApiError::validation(tr(
            "pages.settings.password.errors.lowercase",
        )))
    } else if !password.chars().any(|c| c.is_ascii_digit()) {
        Some(ApiError::validation(tr(
            "pages.settings.password.errors.number",
        )))
    } else if !password.chars().any(|c| !c.is_alphanumeric()) {
        Some(ApiError::validation(tr(
            "pages.settings.password.errors.symbol",
        )))
    } else if is_common_weak_password(password) {
        Some(ApiError::validation(tr(
            "pages.settings.password.errors.too_common",
        )))
    } else {
        None
    }
}

fn is_common_weak_password(password: &str) -> bool {
    let normalized = password
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    COMMON_WEAK_PASSWORDS.contains(&normalized.as_str())
}

fn password_strength_state(password: &str) -> Option<PasswordStrengthState> {
    if password.is_empty() {
        return None;
    }

    let gaps = collect_password_policy_gaps(password);
    let passed = 6usize.saturating_sub(gaps.len());
    let (label_key, badge_class) = match passed {
        0..=2 => (
            "pages.settings.password.strength.labels.weak",
            "bg-action-danger-bg/10 text-action-danger-bg",
        ),
        3..=4 => (
            "pages.settings.password.strength.labels.fair",
            "bg-warning/15 text-warning",
        ),
        5 => (
            "pages.settings.password.strength.labels.good",
            "bg-info/15 text-info",
        ),
        _ => (
            "pages.settings.password.strength.labels.strong",
            "bg-success/15 text-success",
        ),
    };
    let hint = if gaps.is_empty() {
        tr("pages.settings.password.strength.hints.complete")
    } else {
        let requirements = gaps
            .into_iter()
            .map(|key| rust_i18n::t!(key).into_owned())
            .collect::<Vec<_>>()
            .join(" / ");
        rust_i18n::t!(
            "pages.settings.password.strength.hints.missing",
            requirements = requirements
        )
        .into_owned()
    };

    Some(PasswordStrengthState {
        label_key,
        badge_class,
        hint,
    })
}

fn format_password_expiry_warning(days: i64) -> String {
    if days <= 0 {
        tr("pages.settings.password.expiry.expires_today")
    } else {
        rust_i18n::t!("pages.settings.password.expiry.remaining_days", days = days).into_owned()
    }
}

fn password_policy_helper_text() -> String {
    tr("pages.settings.password.policy.helper")
}

fn validate_password_submission(
    new_password: &str,
    confirm_password: &str,
) -> Result<(), ApiError> {
    if let Some(error) = first_password_policy_error(new_password) {
        return Err(error);
    }
    if new_password != confirm_password {
        return Err(ApiError::validation(tr(
            "pages.settings.password.errors.confirm_mismatch",
        )));
    }
    Ok(())
}

fn parse_subject_request_type(value: &str) -> Result<DataSubjectRequestType, &'static str> {
    match value {
        "access" => Ok(DataSubjectRequestType::Access),
        "rectify" => Ok(DataSubjectRequestType::Rectify),
        "delete" => Ok(DataSubjectRequestType::Delete),
        "stop" => Ok(DataSubjectRequestType::Stop),
        _ => Err("pages.settings.subject_request.errors.type_required"),
    }
}

fn normalize_subject_details(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn prepare_password_change_submission(
    pending: bool,
    current: String,
    new_password: String,
    confirm_password: String,
) -> Result<Option<(String, String)>, ApiError> {
    if pending {
        return Ok(None);
    }

    validate_password_submission(&new_password, &confirm_password)?;
    Ok(Some((current, new_password)))
}

fn prepare_mfa_activation_submission(
    pending: bool,
    code: &str,
) -> Result<Option<String>, ApiError> {
    if pending {
        return Ok(None);
    }
    let trimmed = utils::validate_totp_code(code)?;
    Ok(Some(trimmed))
}

fn prepare_subject_request_submission(
    pending: bool,
    request_type: &str,
    details: &str,
) -> Result<Option<CreateDataSubjectRequest>, String> {
    if pending {
        return Ok(None);
    }

    let parsed_type = parse_subject_request_type(request_type).map_err(tr)?;
    Ok(Some(CreateDataSubjectRequest {
        request_type: parsed_type,
        details: normalize_subject_details(details),
    }))
}

fn password_change_feedback(result: Result<(), ApiError>) -> (Option<String>, Option<ApiError>) {
    match result {
        Ok(_) => (Some(tr("pages.settings.password.feedback.updated")), None),
        Err(err) => (None, Some(map_change_password_error(&err))),
    }
}

fn subject_create_feedback<T>(result: Result<T, ApiError>) -> (Option<String>, Option<String>) {
    match result {
        Ok(_) => (
            Some(tr("pages.settings.subject_request.feedback.submitted")),
            None,
        ),
        Err(err) => (None, Some(err.to_string())),
    }
}

fn subject_cancel_feedback(result: Result<(), ApiError>) -> (Option<String>, Option<String>) {
    match result {
        Ok(_) => (
            Some(tr("pages.settings.subject_request.feedback.cancelled")),
            None,
        ),
        Err(err) => (None, Some(err.to_string())),
    }
}

fn is_subject_cancel_disabled(cancel_loading: bool, can_cancel: bool) -> bool {
    cancel_loading || !can_cancel
}

fn should_start_registration(register_loading: bool) -> bool {
    !register_loading
}

fn apply_password_change_effect(
    result: Result<(), ApiError>,
) -> (Option<String>, Option<ApiError>, bool) {
    let (success_msg, error_msg) = password_change_feedback(result);
    let should_clear_inputs = error_msg.is_none();
    (success_msg, error_msg, should_clear_inputs)
}

fn apply_subject_create_effect<T>(
    result: Result<T, ApiError>,
) -> (Option<String>, Option<String>, bool) {
    let (success_msg, error_msg) = subject_create_feedback(result);
    let should_reload = error_msg.is_none();
    (success_msg, error_msg, should_reload)
}

fn apply_subject_cancel_effect(
    result: Result<(), ApiError>,
) -> (Option<String>, Option<String>, bool) {
    let (success_msg, error_msg) = subject_cancel_feedback(result);
    let should_reload = error_msg.is_none();
    (success_msg, error_msg, should_reload)
}

fn set_write_signal<T>(signal: WriteSignal<T>, value: T) {
    signal.set(value);
}

fn set_rw_signal<T>(signal: RwSignal<T>, value: T) {
    signal.set(value);
}

fn password_submit_label_key(is_loading: bool) -> &'static str {
    if is_loading {
        "pages.settings.password.actions.updating"
    } else {
        "pages.settings.password.actions.submit"
    }
}

fn subject_submit_label_key(is_loading: bool) -> &'static str {
    if is_loading {
        "pages.settings.subject_request.actions.submitting"
    } else {
        "pages.settings.subject_request.actions.submit"
    }
}

fn apply_optional_password_change_effect_result(
    result: Option<Result<(), ApiError>>,
    set_password_success_msg: WriteSignal<Option<String>>,
    set_password_error_msg: WriteSignal<Option<ApiError>>,
    set_current_password: WriteSignal<String>,
    set_new_password: WriteSignal<String>,
    set_confirm_password: WriteSignal<String>,
) {
    if let Some(result) = result {
        let (success_msg, error_msg, should_clear_inputs) = apply_password_change_effect(result);
        set_password_success_msg.set(success_msg);
        set_password_error_msg.set(error_msg);
        if should_clear_inputs {
            // Clear inputs only on success.
            set_current_password.set(String::new());
            set_new_password.set(String::new());
            set_confirm_password.set(String::new());
        }
    }
}

fn resolve_password_dispatch_payload(
    pending: bool,
    current: String,
    new_password: String,
    confirm_password: String,
    set_password_success_msg: WriteSignal<Option<String>>,
    set_password_error_msg: WriteSignal<Option<ApiError>>,
) -> Option<(String, String)> {
    match prepare_password_change_submission(pending, current, new_password, confirm_password) {
        Ok(Some(payload)) => {
            set_password_error_msg.set(None);
            set_password_success_msg.set(None);
            Some(payload)
        }
        Ok(None) => None,
        Err(err) => {
            set_password_error_msg.set(Some(err));
            set_password_success_msg.set(None);
            None
        }
    }
}

fn dispatch_password_change_submission<F>(
    pending: bool,
    current: String,
    new_password: String,
    confirm_password: String,
    set_password_success_msg: WriteSignal<Option<String>>,
    set_password_error_msg: WriteSignal<Option<ApiError>>,
    dispatch_change_password: F,
) where
    F: FnOnce((String, String)),
{
    if let Some(payload) = resolve_password_dispatch_payload(
        pending,
        current,
        new_password,
        confirm_password,
        set_password_success_msg,
        set_password_error_msg,
    ) {
        dispatch_change_password(payload);
    }
}

fn start_registration_if_allowed<F1, F2, F3>(
    register_loading: bool,
    mut clear_messages: F1,
    mut clear_setup_info: F2,
    mut dispatch_register: F3,
) where
    F1: FnMut(),
    F2: FnMut(),
    F3: FnMut(),
{
    if !should_start_registration(register_loading) {
        return;
    }
    clear_messages();
    clear_setup_info();
    dispatch_register();
}

fn resolve_mfa_activation_dispatch_code(
    pending: bool,
    code: &str,
    messages: utils::MessageState,
) -> Option<String> {
    match prepare_mfa_activation_submission(pending, code) {
        Ok(Some(trimmed)) => {
            messages.clear();
            Some(trimmed)
        }
        Ok(None) => None,
        Err(err) => {
            messages.set_error(err);
            None
        }
    }
}

fn dispatch_mfa_activation_submission<F>(
    pending: bool,
    code: &str,
    messages: utils::MessageState,
    dispatch_activate: F,
) where
    F: FnOnce(String),
{
    if let Some(trimmed) = resolve_mfa_activation_dispatch_code(pending, code, messages) {
        dispatch_activate(trimmed);
    }
}

fn apply_optional_subject_create_effect_result(
    result: Option<Result<DataSubjectRequestResponse, ApiError>>,
    set_subject_success_msg: WriteSignal<Option<String>>,
    set_subject_error_msg: WriteSignal<Option<String>>,
    subject_details: RwSignal<String>,
    reload: RwSignal<u32>,
) {
    if let Some(result) = result {
        let (success_msg, error_msg, should_reload) = apply_subject_create_effect(result);
        set_subject_success_msg.set(success_msg);
        set_subject_error_msg.set(error_msg);
        if should_reload {
            subject_details.set(String::new());
            reload.update(|value| *value = value.wrapping_add(1));
        }
    }
}

fn apply_optional_subject_cancel_effect_result(
    result: Option<Result<(), ApiError>>,
    set_subject_success_msg: WriteSignal<Option<String>>,
    set_subject_error_msg: WriteSignal<Option<String>>,
    reload: RwSignal<u32>,
) {
    if let Some(result) = result {
        let (success_msg, error_msg, should_reload) = apply_subject_cancel_effect(result);
        set_subject_success_msg.set(success_msg);
        set_subject_error_msg.set(error_msg);
        if should_reload {
            reload.update(|value| *value = value.wrapping_add(1));
        }
    }
}

fn resolve_subject_request_dispatch_payload(
    pending: bool,
    request_type: &str,
    details: &str,
    set_subject_success_msg: WriteSignal<Option<String>>,
    set_subject_error_msg: WriteSignal<Option<String>>,
) -> Option<CreateDataSubjectRequest> {
    match prepare_subject_request_submission(pending, request_type, details) {
        Ok(Some(payload)) => {
            set_subject_error_msg.set(None);
            set_subject_success_msg.set(None);
            Some(payload)
        }
        Ok(None) => None,
        Err(msg) => {
            set_subject_error_msg.set(Some(msg));
            set_subject_success_msg.set(None);
            None
        }
    }
}

fn dispatch_subject_request_submission<F>(
    pending: bool,
    request_type: &str,
    details: &str,
    set_subject_success_msg: WriteSignal<Option<String>>,
    set_subject_error_msg: WriteSignal<Option<String>>,
    dispatch_create_subject: F,
) where
    F: FnOnce(CreateDataSubjectRequest),
{
    if let Some(payload) = resolve_subject_request_dispatch_payload(
        pending,
        request_type,
        details,
        set_subject_success_msg,
        set_subject_error_msg,
    ) {
        dispatch_create_subject(payload);
    }
}

fn subject_requests_error_from_resource(
    result: Option<Result<Vec<DataSubjectRequestResponse>, ApiError>>,
) -> Option<String> {
    result.and_then(|res| res.err()).map(|err| err.to_string())
}

fn subject_requests_from_resource(
    result: Option<Result<Vec<DataSubjectRequestResponse>, ApiError>>,
) -> Vec<DataSubjectRequestResponse> {
    result.and_then(|res| res.ok()).unwrap_or_default()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SubjectRequestRowData {
    can_cancel: bool,
    type_label: String,
    status_label: String,
    created_label: String,
    request_id: String,
}

fn build_subject_request_row_data(request: &DataSubjectRequestResponse) -> SubjectRequestRowData {
    SubjectRequestRowData {
        can_cancel: request.status == "pending",
        type_label: subject_request_type_label(&request.request_type),
        status_label: subject_request_status_label(&request.status),
        created_label: format_subject_datetime(request.created_at),
        request_id: request.id.clone(),
    }
}

fn render_subject_request_row(
    row: SubjectRequestRowData,
    cancel_loading: Signal<bool>,
    on_cancel: Callback<ev::MouseEvent>,
) -> impl IntoView {
    let SubjectRequestRowData {
        can_cancel,
        type_label,
        status_label,
        created_label,
        request_id,
    } = row;

    view! {
        <tr>
            <td class="px-4 py-2 whitespace-nowrap text-sm text-fg">{type_label}</td>
            <td class="px-4 py-2 whitespace-nowrap text-sm text-fg-muted">{status_label}</td>
            <td class="px-4 py-2 whitespace-nowrap text-sm text-fg-muted">{created_label}</td>
            <td class="px-4 py-2 whitespace-nowrap text-right text-sm">
                <button
                    class="text-action-danger-bg hover:text-action-danger-bg-hover disabled:opacity-50"
                    disabled=move || {
                        is_subject_cancel_disabled(cancel_loading.get(), can_cancel)
                    }
                    on:click=move |event| on_cancel.call(event)
                >
                    {rust_i18n::t!("pages.settings.subject_request.actions.cancel")}
                </button>
                <span class="sr-only">{request_id}</span>
            </td>
        </tr>
    }
}

#[component]
pub fn SettingsPage() -> impl IntoView {
    let vm = use_settings_view_model();
    let (auth_state, _set_auth_state) = use_auth();
    let active_tab = create_rw_signal(SettingsTab::Password);

    // --- Password Change State ---
    let (current_password, set_current_password) = create_signal(String::new());
    let (new_password, set_new_password) = create_signal(String::new());
    let (confirm_password, set_confirm_password) = create_signal(String::new());
    let password_strength = Signal::derive(move || password_strength_state(&new_password.get()));
    let password_expiry_warning_days = Signal::derive(move || {
        auth_state
            .get()
            .user
            .and_then(|user| user.password_expiry_warning_days)
    });

    let password_loading = vm.change_password_action.pending();
    let (password_success_msg, set_password_success_msg) = create_signal(Option::<String>::None);
    let (password_error_msg, set_password_error_msg) = create_signal(Option::<ApiError>::None);

    create_effect(move |_| {
        apply_optional_password_change_effect_result(
            vm.change_password_action.value().get(),
            set_password_success_msg,
            set_password_error_msg,
            set_current_password,
            set_new_password,
            set_confirm_password,
        );
    });

    let on_submit_password = move |ev: SubmitEvent| {
        ev.prevent_default();
        dispatch_password_change_submission(
            password_loading.get(),
            current_password.get(),
            new_password.get(),
            confirm_password.get(),
            set_password_success_msg,
            set_password_error_msg,
            |payload| vm.change_password_action.dispatch(payload),
        );
    };

    // --- MFA State (Reusing MfaViewModel) ---
    let mfa_vm = vm.mfa_view_model;
    let register_loading = mfa_vm.register_action.pending();
    let activate_loading = mfa_vm.activate_action.pending();

    // Logic adapted from MfaRegisterPanel for reuse
    let start_registration = {
        move || {
            start_registration_if_allowed(
                register_loading.get(),
                || mfa_vm.messages.clear(),
                || mfa_vm.setup_info.set(None),
                || mfa_vm.register_action.dispatch(()),
            );
        }
    };

    let handle_activate = {
        move |ev: SubmitEvent| {
            ev.prevent_default();
            dispatch_mfa_activation_submission(
                activate_loading.get(),
                &mfa_vm.totp_code.get(),
                mfa_vm.messages,
                |trimmed| mfa_vm.activate_action.dispatch(trimmed),
            );
        }
    };
    let handle_activate_cb = Callback::new(handle_activate);

    // --- Subject Request State ---
    let subject_vm = vm.subject_request_view_model;
    let subject_request_type = create_rw_signal("access".to_string());
    let subject_details = create_rw_signal(String::new());
    let subject_loading = subject_vm.create_action.pending();
    let cancel_loading = subject_vm.cancel_action.pending();
    let cancel_action = subject_vm.cancel_action;
    let subject_requests_resource = subject_vm.requests_resource;
    let subject_requests_error = Signal::derive(move || {
        subject_requests_error_from_resource(subject_requests_resource.get())
    });
    let subject_requests =
        Signal::derive(move || subject_requests_from_resource(subject_requests_resource.get()));
    let (subject_success_msg, set_subject_success_msg) = create_signal(Option::<String>::None);
    let (subject_error_msg, set_subject_error_msg) = create_signal(Option::<String>::None);

    create_effect(move |_| {
        apply_optional_subject_create_effect_result(
            subject_vm.create_action.value().get(),
            set_subject_success_msg,
            set_subject_error_msg,
            subject_details,
            subject_vm.reload,
        );
    });

    create_effect(move |_| {
        apply_optional_subject_cancel_effect_result(
            subject_vm.cancel_action.value().get(),
            set_subject_success_msg,
            set_subject_error_msg,
            subject_vm.reload,
        );
    });

    let on_submit_subject = move |ev: SubmitEvent| {
        ev.prevent_default();
        dispatch_subject_request_submission(
            subject_loading.get(),
            subject_request_type.get().as_str(),
            &subject_details.get(),
            set_subject_success_msg,
            set_subject_error_msg,
            |payload| subject_vm.create_action.dispatch(payload),
        );
    };

    // --- Session Management State ---
    let session_vm = vm.session_management_view_model;
    let session_loading = session_vm.revoke_action.pending();
    let sessions_resource = session_vm.sessions_resource;
    let sessions_error =
        Signal::derive(move || sessions_error_from_resource(sessions_resource.get()));
    let sessions = Signal::derive(move || sessions_from_resource(sessions_resource.get()));
    let (session_success_msg, set_session_success_msg) = create_signal(Option::<String>::None);
    let (session_error_msg, set_session_error_msg) = create_signal(Option::<String>::None);

    create_effect(move |_| {
        apply_optional_session_revoke_effect_result(
            session_vm.revoke_action.value().get(),
            set_session_success_msg,
            set_session_error_msg,
            session_vm.reload,
        );
    });

    view! {
        <Layout>
            <div class="mx-auto max-w-3xl space-y-8">
                <div class="flex p-1.5 gap-1.5 rounded-2xl bg-surface-muted border border-border shadow-inner">
                    <button
                        class=move || {
                            let base = "flex-1 px-4 py-2.5 rounded-xl text-sm font-display font-bold transition-all duration-200";
                            if active_tab.get() == SettingsTab::Password {
                                format!("{base} bg-surface-elevated text-fg shadow-sm transition-all duration-300")
                            } else {
                                format!("{base} text-fg-muted hover:text-fg")
                            }
                        }
                        on:click=move |_| active_tab.set(SettingsTab::Password)
                    >
                        {rust_i18n::t!("pages.settings.tabs.password")}
                    </button>
                    <button
                        class=move || {
                            let base = "flex-1 px-4 py-2.5 rounded-xl text-sm font-display font-bold transition-all duration-200";
                            if active_tab.get() == SettingsTab::Mfa {
                                format!("{base} bg-surface-elevated text-fg shadow-sm transition-all duration-300")
                            } else {
                                format!("{base} text-fg-muted hover:text-fg")
                            }
                        }
                        on:click=move |_| active_tab.set(SettingsTab::Mfa)
                    >
                        {rust_i18n::t!("pages.settings.tabs.mfa")}
                    </button>
                    <button
                        class=move || {
                            let base = "flex-1 px-4 py-2.5 rounded-xl text-sm font-display font-bold transition-all duration-200";
                            if active_tab.get() == SettingsTab::Sessions {
                                format!("{base} bg-surface-elevated text-fg shadow-sm transition-all duration-300")
                            } else {
                                format!("{base} text-fg-muted hover:text-fg")
                            }
                        }
                        on:click=move |_| active_tab.set(SettingsTab::Sessions)
                    >
                        {rust_i18n::t!("pages.settings.tabs.sessions")}
                    </button>
                    <button
                        class=move || {
                            let base = "flex-1 px-4 py-2.5 rounded-xl text-sm font-display font-bold transition-all duration-200";
                            if active_tab.get() == SettingsTab::SubjectRequest {
                                format!("{base} bg-surface-elevated text-fg shadow-sm transition-all duration-300")
                            } else {
                                format!("{base} text-fg-muted hover:text-fg")
                            }
                        }
                        on:click=move |_| active_tab.set(SettingsTab::SubjectRequest)
                    >
                        {rust_i18n::t!("pages.settings.tabs.subject_request")}
                    </button>
                </div>

                // --- Password Change Section ---
                <Show when=move || active_tab.get() == SettingsTab::Password>
                    <div class="bg-surface-elevated rounded-2xl shadow-sm border border-border p-6 space-y-4">
                        <h2 class="text-xl font-display font-bold text-fg border-b border-border pb-2">{rust_i18n::t!("pages.settings.password.title")}</h2>

                    <Show when=move || password_expiry_warning_days.get().is_some() fallback=|| ()>
                        <div class="rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-warning">
                            {move || {
                                format_password_expiry_warning(
                                    password_expiry_warning_days.get().unwrap_or_default(),
                                )
                            }}
                        </div>
                    </Show>

                    <Show when=move || password_success_msg.get().is_some() fallback=|| ()>
                        <SuccessMessage message={password_success_msg.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || password_error_msg.get().is_some() fallback=|| ()>
                        <InlineErrorMessage error={password_error_msg.into()} />
                    </Show>

                    <form class="space-y-4" on:submit=on_submit_password>
                        <div>
                            <label class="block text-sm font-medium text-fg-muted">{rust_i18n::t!("pages.settings.password.fields.current")}</label>
                            <input type="password" required
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                on:input=move |ev| {
                                    set_write_signal(set_current_password, event_target_value(&ev))
                                }
                                prop:value=current_password
                            />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-fg-muted">{rust_i18n::t!("pages.settings.password.fields.new")}</label>
                            <input type="password" required
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                on:input=move |ev| {
                                    set_write_signal(set_new_password, event_target_value(&ev))
                                }
                                prop:value=new_password
                            />
                            <p class="mt-2 text-xs text-fg-muted">{password_policy_helper_text()}</p>
                            <Show when=move || password_strength.get().is_some() fallback=|| ()>
                                <div class="mt-2 rounded-xl border border-border bg-surface-muted px-3 py-3 text-sm text-fg-muted">
                                    <div class="flex items-center justify-between gap-3">
                                        <span class="text-xs uppercase tracking-wide text-fg-muted">{rust_i18n::t!("pages.settings.password.strength.title")}</span>
                                        <span
                                            class=move || {
                                                password_strength
                                                    .get()
                                                    .map(|state| {
                                                        format!(
                                                            "inline-flex rounded-full px-2.5 py-1 text-xs font-semibold {}",
                                                            state.badge_class
                                                        )
                                                    })
                                                    .unwrap_or_default()
                                            }
                                        >
                                            {move || {
                                                password_strength
                                                    .get()
                                                    .map(|state| rust_i18n::t!(state.label_key).into_owned())
                                                    .unwrap_or_default()
                                            }}
                                        </span>
                                    </div>
                                    <p class="mt-2">{move || {
                                        password_strength
                                            .get()
                                            .map(|state| state.hint)
                                            .unwrap_or_default()
                                    }}</p>
                                </div>
                            </Show>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-fg-muted">{rust_i18n::t!("pages.settings.password.fields.confirm")}</label>
                            <input type="password" required
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                on:input=move |ev| {
                                    set_write_signal(set_confirm_password, event_target_value(&ev))
                                }
                                prop:value=confirm_password
                            />
                        </div>
                        <div class="flex justify-end">
                            <button type="submit"
                                class="px-4 py-2 bg-action-primary-bg text-action-primary-text rounded hover:bg-action-primary-bg-hover disabled:opacity-50"
                                disabled=move || password_loading.get()
                            >
                                {move || rust_i18n::t!(password_submit_label_key(password_loading.get())).into_owned()}
                            </button>
                        </div>
                    </form>
                    </div>
                </Show>

                // --- MFA Section ---
                // Reusing components from mfa page, but wrapped in our layout
                <Show when=move || active_tab.get() == SettingsTab::Mfa>
                    <div class="space-y-6">
                        <SetupSection
                            status=mfa_vm.status.read_only()
                            status_loading=mfa_vm.status_loading.read_only()
                            register_loading=register_loading.into()
                            on_register=start_registration
                            on_refresh=move || mfa_vm.fetch_status_action.dispatch(())
                        />
                        <Show when=move || mfa_vm.messages.success.get().is_some() fallback=|| ()>
                            <SuccessMessage message={mfa_vm.messages.success.get().unwrap_or_default()} />
                        </Show>
                        <Show when=move || mfa_vm.messages.error.get().is_some() fallback=|| ()>
                            <InlineErrorMessage error={mfa_vm.messages.error.into()} />
                        </Show>
                        <VerificationSection
                            setup_info=mfa_vm.setup_info.read_only()
                            activate_loading=activate_loading.into()
                            on_submit=handle_activate_cb
                            on_input=mfa_vm.totp_code.write_only()
                        />
                    </div>
                </Show>

                // --- Session Management Section ---
                <Show when=move || active_tab.get() == SettingsTab::Sessions>
                    <div class="bg-surface-elevated rounded-2xl shadow-sm border border-border p-6 space-y-4">
                        <div class="space-y-1">
                            <h2 class="text-xl font-display font-bold text-fg border-b border-border pb-2">{rust_i18n::t!("pages.settings.sessions.title")}</h2>
                            <p class="text-sm text-fg-muted">
                                {rust_i18n::t!("pages.settings.sessions.description")}
                            </p>
                        </div>
                        <Show when=move || session_success_msg.get().is_some() fallback=|| ()>
                            <SuccessMessage message={session_success_msg.get().unwrap_or_default()} />
                        </Show>
                        <Show when=move || session_error_msg.get().is_some() fallback=|| ()>
                            <ErrorMessage message={session_error_msg.get().unwrap_or_default()} />
                        </Show>
                        <Show when=move || sessions_error.get().is_some()>
                            <ErrorMessage message={sessions_error.get().unwrap_or_default()} />
                        </Show>
                        <div class="space-y-3">
                            {move || {
                                let items = sessions.get();
                                if items.is_empty() {
                                    view! {
                                        <div class="rounded-xl border border-dashed border-border px-4 py-6 text-sm text-fg-muted">
                                            {rust_i18n::t!("pages.settings.sessions.empty")}
                                        </div>
                                    }
                                        .into_view()
                                } else {
                                    view! {
                                        <div class="space-y-3">
                                            {items.into_iter().map(|session| {
                                                let session_id = session.id.clone();
                                                let is_current = session.is_current;
                                                let device_label = session_device_label(session.device_label.as_deref());
                                                let created_at = format_session_datetime(session.created_at);
                                                let last_seen_at = format_optional_session_datetime(session.last_seen_at);
                                                let expires_at = format_session_datetime(session.expires_at);
                                                let status_label = session_status_label(is_current);
                                                view! {
                                                    <div class="rounded-xl border border-border bg-surface-muted px-4 py-4 space-y-3">
                                                        <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                                                            <div>
                                                                <div class="text-sm font-semibold text-fg">{device_label}</div>
                                                                <div class="text-xs text-fg-muted">{status_label}</div>
                                                            </div>
                                                            <button
                                                                class="px-3 py-2 rounded bg-action-danger-bg text-action-danger-text hover:bg-action-danger-bg-hover disabled:opacity-50 disabled:cursor-not-allowed text-sm"
                                                                disabled=move || is_session_revoke_disabled(is_current, session_loading.get())
                                                                on:click=move |_| session_vm.revoke_action.dispatch(session_id.clone())
                                                            >
                                                                {session_action_label(is_current)}
                                                            </button>
                                                        </div>
                                                        <dl class="grid grid-cols-1 gap-2 text-sm text-fg-muted sm:grid-cols-3">
                                                            <div>
                                                                <dt class="font-medium">{rust_i18n::t!("pages.settings.sessions.fields.started_at")}</dt>
                                                                <dd class="text-fg">{created_at}</dd>
                                                            </div>
                                                            <div>
                                                                <dt class="font-medium">{rust_i18n::t!("pages.settings.sessions.fields.last_seen_at")}</dt>
                                                                <dd class="text-fg">{last_seen_at}</dd>
                                                            </div>
                                                            <div>
                                                                <dt class="font-medium">{rust_i18n::t!("pages.settings.sessions.fields.expires_at")}</dt>
                                                                <dd class="text-fg">{expires_at}</dd>
                                                            </div>
                                                        </dl>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }
                                        .into_view()
                                }
                            }}
                        </div>
                    </div>
                </Show>

                // --- Subject Request Section ---
                <Show when=move || active_tab.get() == SettingsTab::SubjectRequest>
                    <div class="bg-surface-elevated rounded-2xl shadow-sm border border-border p-6 space-y-4">
                        <h2 class="text-xl font-display font-bold text-fg border-b border-border pb-2">{rust_i18n::t!("pages.settings.subject_request.title")}</h2>
                    <Show when=move || subject_success_msg.get().is_some() fallback=|| ()>
                        <SuccessMessage message={subject_success_msg.get().unwrap_or_default()} />
                    </Show>
                    <Show when=move || subject_error_msg.get().is_some() fallback=|| ()>
                        <ErrorMessage message={subject_error_msg.get().unwrap_or_default()} />
                    </Show>
                    <form class="space-y-3" on:submit=on_submit_subject>
                        <div>
                            <label class="block text-sm font-medium text-fg-muted">{rust_i18n::t!("pages.settings.subject_request.fields.type")}</label>
                            <select
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                prop:value={move || subject_request_type.get()}
                                on:change=move |ev| {
                                    set_rw_signal(subject_request_type, event_target_value(&ev))
                                }
                            >
                                <option value="access">{rust_i18n::t!("pages.settings.subject_request.types.access")}</option>
                                <option value="rectify">{rust_i18n::t!("pages.settings.subject_request.types.rectify")}</option>
                                <option value="delete">{rust_i18n::t!("pages.settings.subject_request.types.delete")}</option>
                                <option value="stop">{rust_i18n::t!("pages.settings.subject_request.types.stop")}</option>
                            </select>
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-fg-muted">{rust_i18n::t!("pages.settings.subject_request.fields.details")}</label>
                            <textarea
                                class="mt-1 w-full border border-form-control-border bg-form-control-bg text-form-control-text rounded px-3 py-2"
                                rows="3"
                                prop:value={move || subject_details.get()}
                                on:input=move |ev| {
                                    set_rw_signal(subject_details, event_target_value(&ev))
                                }
                            ></textarea>
                        </div>
                        <div class="flex justify-end">
                            <button
                                type="submit"
                                class="px-4 py-2 bg-action-primary-bg text-action-primary-text rounded disabled:opacity-50"
                                disabled=move || subject_loading.get()
                            >
                                {move || rust_i18n::t!(subject_submit_label_key(subject_loading.get())).into_owned()}
                            </button>
                        </div>
                    </form>
                    <div>
                        <h3 class="text-sm font-medium text-fg-muted mb-2">{rust_i18n::t!("pages.settings.subject_request.history.title")}</h3>
                        <Show when=move || subject_requests_error.get().is_some()>
                            <ErrorMessage message={subject_requests_error.get().unwrap_or_default()} />
                        </Show>
                        <div class="overflow-x-auto">
                            <table class="min-w-full divide-y divide-border">
                                <thead class="bg-surface-muted">
                                    <tr>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{rust_i18n::t!("pages.settings.subject_request.history.columns.type")}</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{rust_i18n::t!("pages.settings.subject_request.history.columns.status")}</th>
                                        <th class="px-4 py-2 text-left text-xs font-medium text-fg-muted uppercase tracking-wider">{rust_i18n::t!("pages.settings.subject_request.history.columns.created_at")}</th>
                                        <th class="px-4 py-2 text-right text-xs font-medium text-fg-muted uppercase tracking-wider">{rust_i18n::t!("pages.settings.subject_request.history.columns.actions")}</th>
                                    </tr>
                                </thead>
                                <tbody class="bg-surface-elevated divide-y divide-border">
                                    {move || {
                                        subject_requests
                                            .get()
                                            .into_iter()
                                            .map(|request| {
                                                let row = build_subject_request_row_data(&request);
                                                let request_id = row.request_id.clone();
                                                let on_cancel = Callback::new(move |_| {
                                                    cancel_action.dispatch(request_id.clone())
                                                });
                                                render_subject_request_row(
                                                    row,
                                                    cancel_loading.into(),
                                                    on_cancel,
                                                )
                                            })
                                            .collect::<Vec<_>>()
                                    }}
                                </tbody>
                            </table>
                        </div>
                    </div>
                    </div>
                </Show>
            </div>
        </Layout>
    }
}

fn subject_request_type_key(value: &DataSubjectRequestType) -> &'static str {
    match value {
        DataSubjectRequestType::Access => "pages.settings.subject_request.types.access",
        DataSubjectRequestType::Rectify => "pages.settings.subject_request.types.rectify",
        DataSubjectRequestType::Delete => "pages.settings.subject_request.types.delete",
        DataSubjectRequestType::Stop => "pages.settings.subject_request.types.stop",
    }
}

fn subject_request_type_label(value: &DataSubjectRequestType) -> String {
    rust_i18n::t!(subject_request_type_key(value)).into_owned()
}

fn subject_request_status_label(value: &str) -> String {
    match value {
        "pending" => tr("pages.settings.subject_request.status.pending"),
        "approved" => tr("pages.settings.subject_request.status.approved"),
        "rejected" => tr("pages.settings.subject_request.status.rejected"),
        "cancelled" => tr("pages.settings.subject_request.status.cancelled"),
        _ => value.to_string(),
    }
}

fn format_subject_datetime(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%d %H:%M").to_string()
}

fn session_device_label(label: Option<&str>) -> String {
    label
        .map(str::to_string)
        .unwrap_or_else(|| tr("pages.settings.sessions.device.unknown"))
}

fn format_session_datetime(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%d %H:%M").to_string()
}

fn format_optional_session_datetime(value: Option<DateTime<Utc>>) -> String {
    value
        .map(format_session_datetime)
        .unwrap_or_else(|| tr("pages.settings.sessions.fields.not_recorded"))
}

fn session_status_label(is_current: bool) -> String {
    if is_current {
        tr("pages.settings.sessions.status.current")
    } else {
        tr("pages.settings.sessions.status.other")
    }
}

fn session_revoke_feedback(result: Result<(), ApiError>) -> (Option<String>, Option<String>, bool) {
    match result {
        Ok(_) => (
            Some(tr("pages.settings.sessions.feedback.revoked")),
            None,
            true,
        ),
        Err(err) => (None, Some(err.to_string()), false),
    }
}

fn session_action_label(is_current: bool) -> String {
    if is_current {
        tr("pages.settings.sessions.actions.current")
    } else {
        tr("pages.settings.sessions.actions.logout")
    }
}

fn apply_optional_session_revoke_effect_result(
    result: Option<Result<(), ApiError>>,
    set_success: WriteSignal<Option<String>>,
    set_error: WriteSignal<Option<String>>,
    reload: RwSignal<u32>,
) {
    if let Some(result) = result {
        let (success, error, should_reload) = session_revoke_feedback(result);
        set_success.set(success);
        set_error.set(error);
        if should_reload {
            reload.update(|value| *value = value.wrapping_add(1));
        }
    }
}

fn sessions_from_resource(
    value: Option<Result<Vec<SessionResponse>, ApiError>>,
) -> Vec<SessionResponse> {
    value.and_then(Result::ok).unwrap_or_default()
}

fn sessions_error_from_resource(
    value: Option<Result<Vec<SessionResponse>, ApiError>>,
) -> Option<String> {
    value.and_then(Result::err).map(|err| err.to_string())
}

fn is_session_revoke_disabled(is_current: bool, pending: bool) -> bool {
    is_current || pending
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::{
        format_password_expiry_warning, map_change_password_error, normalize_subject_details,
        parse_subject_request_type, password_strength_state, validate_password_submission,
    };
    use crate::api::ApiError;
    use crate::test_support::helpers::set_test_locale;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn map_change_password_error_handles_known_messages() {
        let _locale = set_test_locale("en");
        assert_eq!(
            map_change_password_error(&ApiError::unknown("Current password is incorrect")).error,
            rust_i18n::t!("pages.settings.password.errors.current_incorrect")
        );
        assert_eq!(
            map_change_password_error(&ApiError::unknown(
                "Password must be at least 12 characters long"
            ))
            .error,
            rust_i18n::t!("pages.settings.password.errors.min_length")
        );
        assert_eq!(
            map_change_password_error(&ApiError::unknown(
                "New password must differ from current password"
            ))
            .error,
            rust_i18n::t!("pages.settings.password.errors.must_differ")
        );
    }

    #[wasm_bindgen_test]
    fn map_change_password_error_masks_unknown_messages() {
        let _locale = set_test_locale("en");
        assert_eq!(
            map_change_password_error(&ApiError::unknown("Failed to update password")).error,
            rust_i18n::t!("pages.settings.password.errors.update_failed")
        );
    }

    #[wasm_bindgen_test]
    fn validate_password_submission_checks_constraints() {
        assert!(validate_password_submission("short", "short").is_err());
        assert!(validate_password_submission("ValidPass123!", "different").is_err());
        assert!(validate_password_submission("ValidPass123!", "ValidPass123!").is_ok());
    }

    #[wasm_bindgen_test]
    fn parse_subject_request_type_maps_values() {
        assert!(matches!(
            parse_subject_request_type("access"),
            Ok(crate::api::DataSubjectRequestType::Access)
        ));
        assert!(parse_subject_request_type("unknown").is_err());
    }

    #[wasm_bindgen_test]
    fn normalize_subject_details_trims_or_returns_none() {
        assert_eq!(
            normalize_subject_details("  memo  "),
            Some("memo".to_string())
        );
        assert_eq!(normalize_subject_details("   "), None);
    }

    #[wasm_bindgen_test]
    fn password_strength_state_reports_gaps_and_success() {
        let _locale = set_test_locale("en");
        let weak = password_strength_state("password").expect("strength state");
        assert_eq!(
            rust_i18n::t!(weak.label_key).into_owned(),
            rust_i18n::t!("pages.settings.password.strength.labels.weak")
        );
        assert!(weak.hint.contains(
            rust_i18n::t!("pages.settings.password.policy.requirements.min_length").as_ref()
        ));

        let strong = password_strength_state("ValidPass123!").expect("strength state");
        assert_eq!(
            rust_i18n::t!(strong.label_key).into_owned(),
            rust_i18n::t!("pages.settings.password.strength.labels.strong")
        );
        assert_eq!(
            strong.hint,
            rust_i18n::t!("pages.settings.password.strength.hints.complete")
        );

        assert_eq!(
            format_password_expiry_warning(3),
            rust_i18n::t!("pages.settings.password.expiry.remaining_days", days = 3)
        );
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::api::ApiClient;
    use crate::test_support::helpers::{provide_auth, regular_user, set_test_locale};
    use crate::test_support::ssr::{with_local_runtime_async, with_runtime};
    use leptos_router::{Router, RouterIntegrationContext, ServerIntegration};
    use serde_json::json;

    fn mock_server() -> MockServer {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/auth/mfa");
            then.status(200).json_body(json!({
                "enabled": false,
                "pending": false
            }));
        });
        server.mock(|when, then| {
            when.method(GET).path("/api/subject-requests/me");
            then.status(200).json_body(json!([]));
        });
        server
    }

    fn subject_request_response(
        id: &str,
        request_type: DataSubjectRequestType,
        status: &str,
    ) -> DataSubjectRequestResponse {
        let now = DateTime::parse_from_rfc3339("2026-01-16T12:34:56Z")
            .expect("valid datetime")
            .with_timezone(&Utc);
        DataSubjectRequestResponse {
            id: id.to_string(),
            user_id: "user-1".to_string(),
            request_type,
            status: status.to_string(),
            details: None,
            approved_by: None,
            approved_at: None,
            rejected_by: None,
            rejected_at: None,
            cancelled_at: None,
            decision_comment: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn settings_page_renders_sections() {
        with_local_runtime_async(|| async {
            let _locale = set_test_locale("en");
            let runtime = leptos::create_runtime();
            let server = mock_server();
            provide_context(ApiClient::new_with_base_url(&server.url("/api")));
            let mut user = regular_user();
            user.password_expiry_warning_days = Some(3);
            provide_auth(Some(user));
            provide_context(RouterIntegrationContext::new(ServerIntegration {
                path: "http://localhost/settings".to_string(),
            }));

            leptos_reactive::suppress_resource_load(true);
            let html = view! { <Router><SettingsPage /></Router> }
                .into_view()
                .render_to_string()
                .to_string();
            leptos_reactive::suppress_resource_load(false);

            assert!(html.contains(rust_i18n::t!("pages.settings.tabs.password").as_ref()));
            assert!(html.contains(rust_i18n::t!("pages.settings.tabs.subject_request").as_ref()));
            assert!(html.contains(rust_i18n::t!("pages.settings.tabs.mfa").as_ref()));
            assert!(html.contains(rust_i18n::t!("pages.settings.tabs.sessions").as_ref()));
            assert!(html.contains(rust_i18n::t!("pages.settings.password.fields.current").as_ref()));
            assert!(html.contains(
                rust_i18n::t!("pages.settings.password.expiry.remaining_days", days = 3).as_ref()
            ));
            assert!(!html
                .contains(rust_i18n::t!("pages.settings.subject_request.history.title").as_ref()));

            runtime.dispose();
        });
    }

    #[test]
    fn helper_functions_cover_subject_and_password_validation() {
        assert!(validate_password_submission("short", "short").is_err());
        assert!(validate_password_submission("ValidPass123!", "mismatch").is_err());
        assert!(validate_password_submission("ValidPass123!", "ValidPass123!").is_ok());

        assert!(matches!(
            parse_subject_request_type("rectify"),
            Ok(DataSubjectRequestType::Rectify)
        ));
        assert!(parse_subject_request_type("invalid").is_err());

        assert_eq!(
            normalize_subject_details("  test details  "),
            Some("test details".to_string())
        );
        assert_eq!(normalize_subject_details("   "), None);
    }

    #[test]
    fn helper_functions_cover_labels_and_datetime_format() {
        let _locale = set_test_locale("en");
        assert_eq!(
            subject_request_type_label(&DataSubjectRequestType::Access),
            rust_i18n::t!("pages.settings.subject_request.types.access")
        );
        assert_eq!(
            subject_request_type_label(&DataSubjectRequestType::Rectify),
            rust_i18n::t!("pages.settings.subject_request.types.rectify")
        );
        assert_eq!(
            subject_request_type_label(&DataSubjectRequestType::Delete),
            rust_i18n::t!("pages.settings.subject_request.types.delete")
        );
        assert_eq!(
            subject_request_type_label(&DataSubjectRequestType::Stop),
            rust_i18n::t!("pages.settings.subject_request.types.stop")
        );

        assert_eq!(
            subject_request_status_label("pending"),
            rust_i18n::t!("pages.settings.subject_request.status.pending")
        );
        assert_eq!(
            subject_request_status_label("approved"),
            rust_i18n::t!("pages.settings.subject_request.status.approved")
        );
        assert_eq!(
            subject_request_status_label("rejected"),
            rust_i18n::t!("pages.settings.subject_request.status.rejected")
        );
        assert_eq!(
            subject_request_status_label("cancelled"),
            rust_i18n::t!("pages.settings.subject_request.status.cancelled")
        );
        assert_eq!(subject_request_status_label("custom"), "custom");

        let dt = DateTime::parse_from_rfc3339("2026-01-16T12:34:56Z")
            .expect("valid datetime")
            .with_timezone(&Utc);
        assert_eq!(format_subject_datetime(dt), "2026-01-16 12:34");
        assert_eq!(session_device_label(Some("Chrome")), "Chrome");
        assert_eq!(
            session_device_label(None),
            rust_i18n::t!("pages.settings.sessions.device.unknown")
        );
        assert_eq!(format_session_datetime(dt), "2026-01-16 12:34");
        assert_eq!(
            format_optional_session_datetime(Some(dt)),
            "2026-01-16 12:34"
        );
        assert_eq!(
            format_optional_session_datetime(None),
            rust_i18n::t!("pages.settings.sessions.fields.not_recorded")
        );
        assert_eq!(
            session_status_label(true),
            rust_i18n::t!("pages.settings.sessions.status.current")
        );
        assert_eq!(
            session_status_label(false),
            rust_i18n::t!("pages.settings.sessions.status.other")
        );
        assert!(is_session_revoke_disabled(true, false));
        assert!(is_session_revoke_disabled(false, true));
        assert!(!is_session_revoke_disabled(false, false));
    }

    #[test]
    fn helper_functions_cover_password_error_mapping() {
        let _locale = set_test_locale("en");
        assert_eq!(
            map_change_password_error(&ApiError::unknown("Current password is incorrect")).error,
            rust_i18n::t!("pages.settings.password.errors.current_incorrect")
        );
        assert_eq!(
            map_change_password_error(&ApiError::unknown(
                "Password must be at least 12 characters long"
            ))
            .error,
            rust_i18n::t!("pages.settings.password.errors.min_length")
        );
        assert_eq!(
            map_change_password_error(&ApiError::unknown(
                "New password must differ from current password"
            ))
            .error,
            rust_i18n::t!("pages.settings.password.errors.must_differ")
        );
        assert_eq!(
            map_change_password_error(&ApiError::unknown("other")).error,
            rust_i18n::t!("pages.settings.password.errors.update_failed")
        );
    }

    #[test]
    fn helper_password_strength_and_expiry_warning_cover_branches() {
        let _locale = set_test_locale("en");
        let weak = password_strength_state("password").expect("state");
        assert_eq!(
            rust_i18n::t!(weak.label_key).into_owned(),
            rust_i18n::t!("pages.settings.password.strength.labels.weak")
        );
        assert!(!weak.hint.is_empty());

        let medium = password_strength_state("Validpass123").expect("state");
        assert_eq!(
            rust_i18n::t!(medium.label_key).into_owned(),
            rust_i18n::t!("pages.settings.password.strength.labels.good")
        );
        assert!(!medium.hint.is_empty());

        let strong = password_strength_state("ValidPass123!").expect("state");
        assert_eq!(
            rust_i18n::t!(strong.label_key).into_owned(),
            rust_i18n::t!("pages.settings.password.strength.labels.strong")
        );
        assert_eq!(
            strong.hint,
            rust_i18n::t!("pages.settings.password.strength.hints.complete")
        );

        assert_eq!(
            format_password_expiry_warning(0),
            rust_i18n::t!("pages.settings.password.expiry.expires_today")
        );
        assert_eq!(
            format_password_expiry_warning(5),
            rust_i18n::t!("pages.settings.password.expiry.remaining_days", days = 5)
        );
    }

    #[test]
    fn helper_password_submit_preparation_handles_pending_and_validation() {
        assert!(prepare_password_change_submission(
            true,
            "current".to_string(),
            "ValidPass123!".to_string(),
            "ValidPass123!".to_string(),
        )
        .expect("pending should be accepted")
        .is_none());

        assert!(prepare_password_change_submission(
            false,
            "current".to_string(),
            "short".to_string(),
            "short".to_string(),
        )
        .is_err());

        let payload = prepare_password_change_submission(
            false,
            "current".to_string(),
            "ValidPass123!".to_string(),
            "ValidPass123!".to_string(),
        )
        .expect("valid password payload")
        .expect("dispatch payload");
        assert_eq!(payload.0, "current");
        assert_eq!(payload.1, "ValidPass123!");
    }

    #[test]
    fn helper_mfa_activation_and_subject_submission_cover_branches() {
        assert!(prepare_mfa_activation_submission(true, "123456")
            .expect("pending should be accepted")
            .is_none());
        assert!(prepare_mfa_activation_submission(false, "123").is_err());
        assert_eq!(
            prepare_mfa_activation_submission(false, " 654321 ")
                .expect("valid mfa code")
                .expect("dispatch payload"),
            "654321"
        );

        assert!(prepare_subject_request_submission(true, "access", "memo")
            .expect("pending should be accepted")
            .is_none());
        assert!(prepare_subject_request_submission(false, "invalid", "memo").is_err());

        let payload = prepare_subject_request_submission(false, "delete", "  details ")
            .expect("valid subject request")
            .expect("dispatch payload");
        assert_eq!(payload.request_type, DataSubjectRequestType::Delete);
        assert_eq!(payload.details.as_deref(), Some("details"));

        let payload_blank_detail = prepare_subject_request_submission(false, "stop", "   ")
            .expect("valid subject request")
            .expect("dispatch payload");
        assert_eq!(
            payload_blank_detail.request_type,
            DataSubjectRequestType::Stop
        );
        assert_eq!(payload_blank_detail.details, None);
    }

    #[test]
    fn helper_feedback_mapping_covers_success_and_error() {
        let _locale = set_test_locale("en");
        let (ok_msg, ok_err) = password_change_feedback(Ok(()));
        assert_eq!(
            ok_msg.as_deref(),
            Some(rust_i18n::t!("pages.settings.password.feedback.updated").as_ref())
        );
        assert!(ok_err.is_none());

        let (fail_msg, fail_err) =
            password_change_feedback(Err(ApiError::unknown("Current password is incorrect")));
        assert!(fail_msg.is_none());
        assert_eq!(
            fail_err.expect("mapped error").error,
            rust_i18n::t!("pages.settings.password.errors.current_incorrect")
        );

        let (create_ok_msg, create_ok_err) = subject_create_feedback(Ok(()));
        assert_eq!(
            create_ok_msg.as_deref(),
            Some(rust_i18n::t!("pages.settings.subject_request.feedback.submitted").as_ref())
        );
        assert!(create_ok_err.is_none());

        let (create_fail_msg, create_fail_err) =
            subject_create_feedback::<()>(Err(ApiError::unknown("boom")));
        assert!(create_fail_msg.is_none());
        assert_eq!(create_fail_err.as_deref(), Some("boom"));

        let (cancel_ok_msg, cancel_ok_err) = subject_cancel_feedback(Ok(()));
        assert_eq!(
            cancel_ok_msg.as_deref(),
            Some(rust_i18n::t!("pages.settings.subject_request.feedback.cancelled").as_ref())
        );
        assert!(cancel_ok_err.is_none());

        let (session_ok_msg, session_ok_err, session_reload) = session_revoke_feedback(Ok(()));
        assert_eq!(
            session_ok_msg.as_deref(),
            Some(rust_i18n::t!("pages.settings.sessions.feedback.revoked").as_ref())
        );
        assert!(session_ok_err.is_none());
        assert!(session_reload);

        let (cancel_fail_msg, cancel_fail_err) =
            subject_cancel_feedback(Err(ApiError::unknown("cancel failed")));
        assert!(cancel_fail_msg.is_none());
        assert_eq!(cancel_fail_err.as_deref(), Some("cancel failed"));
    }

    #[test]
    fn helper_effect_state_decisions_cover_branches() {
        let _locale = set_test_locale("en");
        let (password_ok_msg, password_ok_err, clear_ok) = apply_password_change_effect(Ok(()));
        assert_eq!(
            password_ok_msg.as_deref(),
            Some(rust_i18n::t!("pages.settings.password.feedback.updated").as_ref())
        );
        assert!(password_ok_err.is_none());
        assert!(clear_ok);

        let (password_fail_msg, password_fail_err, clear_fail) =
            apply_password_change_effect(Err(ApiError::unknown("x")));
        assert!(password_fail_msg.is_none());
        assert!(password_fail_err.is_some());
        assert!(!clear_fail);

        let (subject_create_ok_msg, subject_create_ok_err, subject_create_reload_ok) =
            apply_subject_create_effect::<()>(Ok(()));
        assert_eq!(
            subject_create_ok_msg.as_deref(),
            Some(rust_i18n::t!("pages.settings.subject_request.feedback.submitted").as_ref())
        );
        assert!(subject_create_ok_err.is_none());
        assert!(subject_create_reload_ok);

        let (subject_create_fail_msg, subject_create_fail_err, subject_create_reload_fail) =
            apply_subject_create_effect::<()>(Err(ApiError::unknown("create failed")));
        assert!(subject_create_fail_msg.is_none());
        assert_eq!(subject_create_fail_err.as_deref(), Some("create failed"));
        assert!(!subject_create_reload_fail);

        let (subject_cancel_ok_msg, subject_cancel_ok_err, subject_cancel_reload_ok) =
            apply_subject_cancel_effect(Ok(()));
        assert_eq!(
            subject_cancel_ok_msg.as_deref(),
            Some(rust_i18n::t!("pages.settings.subject_request.feedback.cancelled").as_ref())
        );
        assert!(subject_cancel_ok_err.is_none());
        assert!(subject_cancel_reload_ok);

        let (subject_cancel_fail_msg, subject_cancel_fail_err, subject_cancel_reload_fail) =
            apply_subject_cancel_effect(Err(ApiError::unknown("cancel failed")));
        assert!(subject_cancel_fail_msg.is_none());
        assert_eq!(subject_cancel_fail_err.as_deref(), Some("cancel failed"));
        assert!(!subject_cancel_reload_fail);
    }

    #[test]
    fn helper_subject_request_type_parsing_covers_all_known_values() {
        assert!(matches!(
            parse_subject_request_type("access"),
            Ok(DataSubjectRequestType::Access)
        ));
        assert!(matches!(
            parse_subject_request_type("rectify"),
            Ok(DataSubjectRequestType::Rectify)
        ));
        assert!(matches!(
            parse_subject_request_type("delete"),
            Ok(DataSubjectRequestType::Delete)
        ));
        assert!(matches!(
            parse_subject_request_type("stop"),
            Ok(DataSubjectRequestType::Stop)
        ));
    }

    #[test]
    fn helper_subject_cancel_disable_logic_covers_branches() {
        assert!(is_subject_cancel_disabled(true, true));
        assert!(is_subject_cancel_disabled(false, false));
        assert!(!is_subject_cancel_disabled(false, true));
        assert!(should_start_registration(false));
        assert!(!should_start_registration(true));
    }

    #[test]
    fn helper_dispatch_helpers_cover_pending_validation_and_success() {
        with_runtime(|| {
            let _locale = set_test_locale("en");
            let (password_success_msg, set_password_success_msg) =
                create_signal(Some("previous".to_string()));
            let (password_error_msg, set_password_error_msg) =
                create_signal(Some(ApiError::unknown("previous-error")));
            let mut dispatched_password: Option<(String, String)> = None;
            dispatch_password_change_submission(
                false,
                "current".to_string(),
                "ValidPass123!".to_string(),
                "ValidPass123!".to_string(),
                set_password_success_msg,
                set_password_error_msg,
                |payload| dispatched_password = Some(payload),
            );
            assert_eq!(
                dispatched_password
                    .as_ref()
                    .map(|(current, _)| current.as_str()),
                Some("current")
            );
            assert_eq!(
                dispatched_password.as_ref().map(|(_, new)| new.as_str()),
                Some("ValidPass123!")
            );
            assert!(password_success_msg.get().is_none());
            assert!(password_error_msg.get().is_none());

            set_password_success_msg.set(Some("keep-success".to_string()));
            set_password_error_msg.set(Some(ApiError::unknown("keep-error")));
            dispatch_password_change_submission(
                true,
                "current".to_string(),
                "ValidPass123!".to_string(),
                "ValidPass123!".to_string(),
                set_password_success_msg,
                set_password_error_msg,
                |_| panic!("pending path must not dispatch"),
            );
            assert_eq!(password_success_msg.get().as_deref(), Some("keep-success"));
            assert_eq!(
                password_error_msg.get().map(|err| err.error),
                Some("keep-error".to_string())
            );

            dispatch_password_change_submission(
                false,
                "current".to_string(),
                "short".to_string(),
                "short".to_string(),
                set_password_success_msg,
                set_password_error_msg,
                |_| panic!("validation error path must not dispatch"),
            );
            assert!(password_success_msg.get().is_none());
            assert_eq!(
                password_error_msg.get().map(|err| err.error),
                Some(tr("pages.settings.password.errors.min_length"))
            );

            let mfa_messages = utils::MessageState::default();
            mfa_messages.set_success("mfa-ok".to_string());
            let mut dispatched_code: Option<String> = None;
            dispatch_mfa_activation_submission(false, " 654321 ", mfa_messages, |code| {
                dispatched_code = Some(code)
            });
            assert_eq!(dispatched_code.as_deref(), Some("654321"));
            assert!(mfa_messages.success.get().is_none());
            assert!(mfa_messages.error.get().is_none());

            dispatch_mfa_activation_submission(false, "123", mfa_messages, |_| {
                panic!("invalid mfa code must not dispatch")
            });
            assert!(mfa_messages.success.get().is_none());
            assert!(mfa_messages.error.get().is_some());

            mfa_messages.set_success("pending".to_string());
            dispatch_mfa_activation_submission(true, "654321", mfa_messages, |_| {
                panic!("pending mfa path must not dispatch")
            });
            assert_eq!(mfa_messages.success.get().as_deref(), Some("pending"));

            let (subject_success_msg, set_subject_success_msg) =
                create_signal(Some("old-success".to_string()));
            let (subject_error_msg, set_subject_error_msg) =
                create_signal(Some("old-error".to_string()));
            let mut dispatched_subject: Option<CreateDataSubjectRequest> = None;
            dispatch_subject_request_submission(
                false,
                "delete",
                "  details  ",
                set_subject_success_msg,
                set_subject_error_msg,
                |payload| dispatched_subject = Some(payload),
            );
            assert_eq!(
                dispatched_subject
                    .as_ref()
                    .map(|payload| &payload.request_type),
                Some(&DataSubjectRequestType::Delete)
            );
            assert_eq!(
                dispatched_subject
                    .as_ref()
                    .and_then(|payload| payload.details.as_deref()),
                Some("details")
            );
            assert!(subject_success_msg.get().is_none());
            assert!(subject_error_msg.get().is_none());

            dispatch_subject_request_submission(
                false,
                "invalid",
                "memo",
                set_subject_success_msg,
                set_subject_error_msg,
                |_| panic!("invalid subject request type must not dispatch"),
            );
            assert!(subject_success_msg.get().is_none());
            assert_eq!(
                subject_error_msg.get().as_deref(),
                Some(rust_i18n::t!("pages.settings.subject_request.errors.type_required").as_ref())
            );

            set_subject_success_msg.set(Some("keep".to_string()));
            set_subject_error_msg.set(Some("keep-error".to_string()));
            dispatch_subject_request_submission(
                true,
                "access",
                "memo",
                set_subject_success_msg,
                set_subject_error_msg,
                |_| panic!("pending subject path must not dispatch"),
            );
            assert_eq!(subject_success_msg.get().as_deref(), Some("keep"));
            assert_eq!(subject_error_msg.get().as_deref(), Some("keep-error"));
        });
    }

    #[test]
    fn helper_optional_effect_and_resource_projection_cover_paths() {
        with_runtime(|| {
            let _locale = set_test_locale("en");
            let (current_password, set_current_password) = create_signal("current".to_string());
            let (new_password, set_new_password) = create_signal("new".to_string());
            let (confirm_password, set_confirm_password) = create_signal("confirm".to_string());
            let (password_success_msg, set_password_success_msg) =
                create_signal(Some("old-success".to_string()));
            let (password_error_msg, set_password_error_msg) =
                create_signal(Some(ApiError::unknown("old-error")));

            apply_optional_password_change_effect_result(
                None,
                set_password_success_msg,
                set_password_error_msg,
                set_current_password,
                set_new_password,
                set_confirm_password,
            );
            assert_eq!(current_password.get(), "current");
            assert_eq!(new_password.get(), "new");
            assert_eq!(confirm_password.get(), "confirm");

            apply_optional_password_change_effect_result(
                Some(Ok(())),
                set_password_success_msg,
                set_password_error_msg,
                set_current_password,
                set_new_password,
                set_confirm_password,
            );
            assert_eq!(
                password_success_msg.get().as_deref(),
                Some(rust_i18n::t!("pages.settings.password.feedback.updated").as_ref())
            );
            assert!(password_error_msg.get().is_none());
            assert_eq!(current_password.get(), "");
            assert_eq!(new_password.get(), "");
            assert_eq!(confirm_password.get(), "");

            set_current_password.set("remain-current".to_string());
            set_new_password.set("remain-new".to_string());
            set_confirm_password.set("remain-confirm".to_string());
            apply_optional_password_change_effect_result(
                Some(Err(ApiError::unknown("change failed"))),
                set_password_success_msg,
                set_password_error_msg,
                set_current_password,
                set_new_password,
                set_confirm_password,
            );
            assert!(password_success_msg.get().is_none());
            assert!(password_error_msg.get().is_some());
            assert_eq!(current_password.get(), "remain-current");
            assert_eq!(new_password.get(), "remain-new");
            assert_eq!(confirm_password.get(), "remain-confirm");

            let (subject_success_msg, set_subject_success_msg) =
                create_signal(Some("subject-old-success".to_string()));
            let (subject_error_msg, set_subject_error_msg) =
                create_signal(Some("subject-old-error".to_string()));
            let subject_details = create_rw_signal("details".to_string());
            let subject_reload = create_rw_signal(10u32);

            apply_optional_subject_create_effect_result(
                None,
                set_subject_success_msg,
                set_subject_error_msg,
                subject_details,
                subject_reload,
            );
            assert_eq!(subject_reload.get(), 10);
            assert_eq!(subject_details.get(), "details");

            apply_optional_subject_create_effect_result(
                Some(Ok(subject_request_response(
                    "sr-ok",
                    DataSubjectRequestType::Access,
                    "pending",
                ))),
                set_subject_success_msg,
                set_subject_error_msg,
                subject_details,
                subject_reload,
            );
            assert_eq!(
                subject_success_msg.get().as_deref(),
                Some(rust_i18n::t!("pages.settings.subject_request.feedback.submitted").as_ref())
            );
            assert!(subject_error_msg.get().is_none());
            assert_eq!(subject_details.get(), "");
            assert_eq!(subject_reload.get(), 11);

            subject_details.set("keep".to_string());
            apply_optional_subject_create_effect_result(
                Some(Err(ApiError::unknown("create failed"))),
                set_subject_success_msg,
                set_subject_error_msg,
                subject_details,
                subject_reload,
            );
            assert!(subject_success_msg.get().is_none());
            assert_eq!(subject_error_msg.get().as_deref(), Some("create failed"));
            assert_eq!(subject_details.get(), "keep");
            assert_eq!(subject_reload.get(), 11);

            apply_optional_subject_cancel_effect_result(
                None,
                set_subject_success_msg,
                set_subject_error_msg,
                subject_reload,
            );
            assert_eq!(subject_reload.get(), 11);

            apply_optional_subject_cancel_effect_result(
                Some(Ok(())),
                set_subject_success_msg,
                set_subject_error_msg,
                subject_reload,
            );
            assert_eq!(
                subject_success_msg.get().as_deref(),
                Some(rust_i18n::t!("pages.settings.subject_request.feedback.cancelled").as_ref())
            );
            assert!(subject_error_msg.get().is_none());
            assert_eq!(subject_reload.get(), 12);

            apply_optional_subject_cancel_effect_result(
                Some(Err(ApiError::unknown("cancel failed"))),
                set_subject_success_msg,
                set_subject_error_msg,
                subject_reload,
            );
            assert!(subject_success_msg.get().is_none());
            assert_eq!(subject_error_msg.get().as_deref(), Some("cancel failed"));
            assert_eq!(subject_reload.get(), 12);

            let ok_resource = Some(Ok(vec![subject_request_response(
                "sr-1",
                DataSubjectRequestType::Delete,
                "approved",
            )]));
            assert!(subject_requests_error_from_resource(ok_resource.clone()).is_none());
            assert_eq!(subject_requests_from_resource(ok_resource).len(), 1);

            let err_resource = Some(Err(ApiError::unknown("load failed")));
            assert_eq!(
                subject_requests_error_from_resource(err_resource.clone()).as_deref(),
                Some("load failed")
            );
            assert!(subject_requests_from_resource(err_resource).is_empty());
            assert!(subject_requests_from_resource(None).is_empty());
        });
    }

    #[test]
    fn helper_row_data_and_render_cover_paths() {
        with_runtime(|| {
            let _locale = set_test_locale("en");
            let pending_request =
                subject_request_response("sr-pending", DataSubjectRequestType::Access, "pending");
            let pending_row = build_subject_request_row_data(&pending_request);
            assert!(pending_row.can_cancel);
            assert_eq!(
                pending_row.type_label,
                rust_i18n::t!("pages.settings.subject_request.types.access")
            );
            assert_eq!(
                pending_row.status_label,
                rust_i18n::t!("pages.settings.subject_request.status.pending")
            );
            assert_eq!(pending_row.created_label, "2026-01-16 12:34");

            let approved_request = subject_request_response(
                "sr-approved",
                DataSubjectRequestType::Rectify,
                "approved",
            );
            let approved_row = build_subject_request_row_data(&approved_request);
            assert!(!approved_row.can_cancel);
            assert_eq!(
                approved_row.type_label,
                rust_i18n::t!("pages.settings.subject_request.types.rectify")
            );
            assert_eq!(
                approved_row.status_label,
                rust_i18n::t!("pages.settings.subject_request.status.approved")
            );

            let (cancel_loading, set_cancel_loading) = create_signal(false);
            let html = render_subject_request_row(
                pending_row.clone(),
                cancel_loading.into(),
                Callback::new(|_| {}),
            )
            .into_view()
            .render_to_string()
            .to_string();
            assert!(html
                .contains(rust_i18n::t!("pages.settings.subject_request.types.access").as_ref()));
            assert!(html
                .contains(rust_i18n::t!("pages.settings.subject_request.status.pending").as_ref()));
            assert!(html.contains("sr-pending"));
            assert!(html
                .contains(rust_i18n::t!("pages.settings.subject_request.actions.cancel").as_ref()));

            set_cancel_loading.set(true);
            let html_with_loading = render_subject_request_row(
                pending_row,
                cancel_loading.into(),
                Callback::new(|_| {}),
            )
            .into_view()
            .render_to_string()
            .to_string();
            assert!(html_with_loading.contains("sr-pending"));
        });
    }

    #[test]
    fn helper_registration_label_and_signal_setters_cover_paths() {
        with_runtime(|| {
            let _locale = set_test_locale("en");
            let (write_value, set_write_value) = create_signal(String::new());
            set_write_signal(set_write_value, "updated".to_string());
            assert_eq!(write_value.get(), "updated");

            let rw_value = create_rw_signal(String::new());
            set_rw_signal(rw_value, "rw-updated".to_string());
            assert_eq!(rw_value.get(), "rw-updated");
        });

        assert_eq!(
            rust_i18n::t!(password_submit_label_key(true)),
            rust_i18n::t!("pages.settings.password.actions.updating")
        );
        assert_eq!(
            rust_i18n::t!(password_submit_label_key(false)),
            rust_i18n::t!("pages.settings.password.actions.submit")
        );
        assert_eq!(
            rust_i18n::t!(subject_submit_label_key(true)),
            rust_i18n::t!("pages.settings.subject_request.actions.submitting")
        );
        assert_eq!(
            rust_i18n::t!(subject_submit_label_key(false)),
            rust_i18n::t!("pages.settings.subject_request.actions.submit")
        );

        let mut cleared_messages = 0;
        let mut cleared_setup_info = 0;
        let mut dispatched_register = 0;
        start_registration_if_allowed(
            true,
            || cleared_messages += 1,
            || cleared_setup_info += 1,
            || dispatched_register += 1,
        );
        assert_eq!(cleared_messages, 0);
        assert_eq!(cleared_setup_info, 0);
        assert_eq!(dispatched_register, 0);

        start_registration_if_allowed(
            false,
            || cleared_messages += 1,
            || cleared_setup_info += 1,
            || dispatched_register += 1,
        );
        assert_eq!(cleared_messages, 1);
        assert_eq!(cleared_setup_info, 1);
        assert_eq!(dispatched_register, 1);
    }
}
