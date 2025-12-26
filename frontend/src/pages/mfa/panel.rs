use crate::{
    components::layout::{ErrorMessage, Layout, SuccessMessage},
    pages::mfa::{
        components::{setup::SetupSection, verify::VerificationSection},
        utils,
    },
    state::auth::use_auth,
};
use leptos::{ev::SubmitEvent, Callback, *};

#[component]
pub fn MfaRegisterPanel() -> impl IntoView {
    let vm = crate::pages::mfa::view_model::use_mfa_view_model();

    let register_loading = vm.register_action.pending();
    let activate_loading = vm.activate_action.pending();

    let start_registration = {
        move || {
            if register_loading.get() {
                return;
            }
            vm.messages.update(|m| m.clear());
            vm.setup_info.set(None);
            vm.register_action.dispatch(());
        }
    };

    let handle_activate = {
        move |ev: SubmitEvent| {
            ev.prevent_default();
            if activate_loading.get() {
                return;
            }
            let code_value = vm.totp_code.get();
            let trimmed = match utils::validate_totp_code(&code_value) {
                Ok(code) => code,
                Err(msg) => {
                    vm.messages.update(|m| m.set_error(msg));
                    return;
                }
            };

            vm.messages.update(|m| m.clear());
            vm.activate_action.dispatch(trimmed);
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
                <Show when=move || vm.messages.get().success.get().is_some() fallback=|| ()>
                    <SuccessMessage message={vm.messages.get().success.get().unwrap_or_default()} />
                </Show>
                <Show when=move || vm.messages.get().error.get().is_some() fallback=|| ()>
                    <ErrorMessage message={vm.messages.get().error.get().unwrap_or_default()} />
                </Show>
                <VerificationSection
                    setup_info=vm.setup_info.read_only()
                    activate_loading=activate_loading.into()
                    on_submit=handle_activate
                    on_input=vm.totp_code
                />
            </div>
        </Layout>
    }
}
