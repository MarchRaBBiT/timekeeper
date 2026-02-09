use crate::{
    api::ApiError,
    components::{
        error::InlineErrorMessage,
        layout::{Layout, SuccessMessage},
    },
    pages::mfa::{
        components::{setup::SetupSection, verify::VerificationSection},
        utils,
    },
};
use leptos::{ev::SubmitEvent, Callback, *};

fn prepare_registration(pending: bool) -> bool {
    !pending
}

fn prepare_activation_submission(pending: bool, code: &str) -> Result<Option<String>, ApiError> {
    if pending {
        return Ok(None);
    }
    let trimmed = utils::validate_totp_code(code)?;
    Ok(Some(trimmed))
}

#[component]
pub fn MfaRegisterPanel() -> impl IntoView {
    let vm = crate::pages::mfa::view_model::use_mfa_view_model();

    let register_loading = vm.register_action.pending();
    let activate_loading = vm.activate_action.pending();

    let start_registration = {
        move || {
            if !prepare_registration(register_loading.get()) {
                return;
            }
            vm.messages.clear();
            vm.setup_info.set(None);
            vm.register_action.dispatch(());
        }
    };

    let handle_activate = {
        move |ev: SubmitEvent| {
            ev.prevent_default();
            let code_value = vm.totp_code.get();
            let prepared = match prepare_activation_submission(activate_loading.get(), &code_value)
            {
                Ok(Some(code)) => code,
                Ok(None) => return,
                Err(msg) => {
                    vm.messages.set_error(msg);
                    return;
                }
            };

            vm.messages.clear();
            vm.activate_action.dispatch(prepared);
        }
    };
    let handle_activate = Callback::new(handle_activate);

    view! {
        <Layout>
            <div class="mx-auto max-w-3xl space-y-6">
                <SetupSection
                    status=vm.status.read_only()
                    status_loading=vm.status_loading.read_only()
                    register_loading=register_loading.into()
                    on_register=start_registration
                    on_refresh=move || vm.fetch_status_action.dispatch(())
                />
                <Show when=move || vm.messages.success.get().is_some() fallback=|| ()>
                    <SuccessMessage message={vm.messages.success.get().unwrap_or_default()} />
                </Show>
                <Show when=move || vm.messages.error.get().is_some() fallback=|| ()>
                    <InlineErrorMessage error={vm.messages.error.into()} />
                </Show>
                <VerificationSection
                    setup_info=vm.setup_info.read_only()
                    activate_loading=activate_loading.into()
                    on_submit=handle_activate
                    on_input=vm.totp_code.write_only()
                />
            </div>
        </Layout>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::api::test_support::mock::*;
    use crate::test_support::helpers::{admin_user, provide_auth};
    use crate::test_support::ssr::render_to_string;

    #[test]
    fn mfa_register_panel_renders_sections() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/auth/mfa");
            then.status(200).json_body(serde_json::json!({
                "enabled": false,
                "pending": false
            }));
        });

        let server = server.clone();
        let html = render_to_string(move || {
            provide_auth(Some(admin_user(true)));
            provide_context(crate::api::ApiClient::new_with_base_url(
                &server.url("/api"),
            ));
            view! { <MfaRegisterPanel /> }
        });
        assert!(html.contains("MFA 設定"));
    }

    #[test]
    fn helper_prepare_registration_and_activation_cover_branches() {
        assert!(prepare_registration(false));
        assert!(!prepare_registration(true));

        assert!(prepare_activation_submission(true, "123456")
            .expect("pending should short-circuit")
            .is_none());
        assert!(prepare_activation_submission(false, "123").is_err());
        assert_eq!(
            prepare_activation_submission(false, " 654321 ")
                .expect("valid code")
                .expect("dispatch payload"),
            "654321"
        );
    }
}
