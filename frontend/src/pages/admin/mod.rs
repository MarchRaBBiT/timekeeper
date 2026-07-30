use crate::components::guard::RequireAuth;
use leptos::*;

pub mod attendance_report;
pub mod components;
pub mod layout;
pub mod panel;
pub mod repository;
pub mod utils;
pub mod view_model;

pub use panel::AdminPanel;

#[component]
pub fn AdminPage() -> impl IntoView {
    view! { <RequireAuth><AdminPanel /></RequireAuth> }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod i18n_tests {
    use crate::test_support::helpers::set_test_locale;

    const ADMIN_TRANSLATION_KEYS: &[&str] = &[
        "admin_components.attendance_report.title",
        "admin_components.attendance_report.loading",
        "admin_components.attendance_report.error",
        "admin_components.attendance_report.empty",
        "admin_components.attendance_report.filters.month",
        "admin_components.attendance_report.filters.department",
        "admin_components.attendance_report.filters.apply",
        "admin_components.attendance_report.pagination.previous",
        "admin_components.attendance_report.pagination.next",
        "admin_components.attendance.title",
        "admin_components.attendance.validation.required_fields",
        "admin_components.attendance.validation.date_format",
        "admin_components.attendance.validation.break_required",
        "admin_components.attendance.feedback.saved",
        "admin_components.attendance.feedback.break_ended",
        "admin_components.attendance.actions.save",
        "admin_components.attendance.actions.saving",
        "admin_components.attendance.actions.force_end",
        "admin_components.attendance.actions.force_ending",
        "admin_components.attendance.actions.add_break",
        "admin_components.attendance.actions.reload",
        "admin_components.attendance.fields.target_user",
        "admin_components.attendance.fields.breaks_optional",
        "admin_components.attendance.active_break.title",
        "admin_components.attendance.active_break.description",
        "admin_components.attendance.active_break.loading",
        "admin_components.attendance.active_break.fetch_failed",
        "admin_components.attendance.active_break.placeholder",
        "admin_components.attendance.active_break.empty",
        "admin_components.departments.title",
        "admin_components.departments.validation.name_required",
        "admin_components.departments.fields.name",
        "admin_components.departments.fields.parent",
        "admin_components.departments.placeholders.name",
        "admin_components.departments.actions.create",
        "admin_components.departments.actions.creating",
        "admin_components.departments.actions.delete",
        "admin_components.departments.empty",
        "admin_components.holidays.title",
        "admin_components.holidays.validation.optional_date_format",
        "admin_components.holidays.validation.date_order",
        "admin_components.holidays.validation.month_required",
        "admin_components.holidays.validation.month_format",
        "admin_components.holidays.validation.required_fields",
        "admin_components.holidays.validation.date_format",
        "admin_components.holidays.feedback.none_to_import",
        "admin_components.holidays.feedback.imported",
        "admin_components.holidays.feedback.created",
        "admin_components.holidays.feedback.deleted",
        "admin_components.holidays.pagination.loading",
        "admin_components.holidays.pagination.empty",
        "admin_components.holidays.pagination.summary",
        "admin_components.holidays.actions.create",
        "admin_components.holidays.actions.creating",
        "admin_components.holidays.actions.fetch_google",
        "admin_components.holidays.actions.fetching",
        "admin_components.holidays.actions.register_selected",
        "admin_components.holidays.filters.title",
        "admin_components.holidays.filters.description",
        "admin_components.holidays.filters.apply",
        "admin_components.holidays.filters.clear",
        "admin_components.holidays.filters.month_range",
        "admin_components.holidays.filters.apply_month",
        "admin_components.holidays.google_candidates",
        "admin_components.requests.title",
        "admin_components.requests.types.leave",
        "admin_components.requests.types.overtime",
        "admin_components.requests.loading",
        "admin_components.requests.empty.title",
        "admin_components.requests.empty.description",
        "admin_components.requests.detail.title",
        "admin_components.requests.detail.comment_optional",
        "admin_components.subject_requests.title",
        "admin_components.subject_requests.validation.comment_required",
        "admin_components.subject_requests.validation.request_missing",
        "admin_components.subject_requests.loading",
        "admin_components.subject_requests.detail.title",
        "admin_components.subject_requests.detail.comment",
        "admin_components.system_tools.title",
        "admin_components.system_tools.validation.user_required",
        "admin_components.system_tools.feedback.reset",
        "admin_components.system_tools.actions.reset",
        "admin_components.system_tools.actions.resetting",
        "admin_components.weekly_holidays.description",
        "admin_components.weekly_holidays.validation.weekday",
        "admin_components.weekly_holidays.validation.start_required",
        "admin_components.weekly_holidays.validation.start_format",
        "admin_components.weekly_holidays.validation.start_min",
        "admin_components.weekly_holidays.validation.end_format",
        "admin_components.weekly_holidays.validation.end_order",
        "admin_components.weekly_holidays.feedback.created",
        "admin_components.weekly_holidays.feedback.deleted",
        "admin_components.weekly_holidays.hints.system_admin",
        "admin_components.weekly_holidays.hints.admin",
        "admin_components.weekly_holidays.actions.create",
        "admin_components.weekly_holidays.actions.creating",
        "admin_components.weekly_holidays.loading",
        "admin_components.weekly_holidays.empty",
        "admin_components.weekly_holidays.confirm_delete",
    ];

    #[test]
    fn admin_translation_keys_resolve_in_supported_locales() {
        for locale in ["ja", "en"] {
            let _locale = set_test_locale(locale);
            for key in ADMIN_TRANSLATION_KEYS {
                let translated = rust_i18n::t!(*key);
                assert_ne!(translated, *key, "missing {locale} translation for {key}");
                assert!(
                    !translated.trim().is_empty(),
                    "empty {locale} translation for {key}"
                );
            }
        }
    }
}
