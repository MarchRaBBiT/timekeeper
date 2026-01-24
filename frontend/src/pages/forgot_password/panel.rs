use super::view_model::use_forgot_password_view_model;
use leptos::*;
use leptos_router::*;

#[component]
pub fn ForgotPasswordPanel() -> impl IntoView {
    let vm = use_forgot_password_view_model();
    let email = vm.email;
    let error = vm.error;
    let success = vm.success;
    let submit_action = vm.submit_action;

    let pending = submit_action.pending();

    view! {
        <div class="min-h-screen flex items-center justify-center bg-surface py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-fg">
                        "Reset your password"
                    </h2>
                    <p class="mt-2 text-center text-sm text-fg-muted">
                        "Enter your email address and we'll send you a link to reset your password."
                    </p>
                </div>

                {move || {
                    if let Some(msg) = success.get() {
                        view! {
                            <div class="rounded-md bg-status-success-bg p-4 text-status-success-text">
                                <div class="flex">
                                    <div class="flex-shrink-0">
                                        <svg
                                            class="h-5 w-5 text-status-success-text"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                                                clip-rule="evenodd"
                                            ></path>
                                        </svg>
                                    </div>
                                    <div class="ml-3">
                                        <h3 class="text-sm font-medium text-status-success-text">{msg}</h3>
                                        <div class="mt-2 text-sm text-status-success-text">
                                            <p>"Check your email for the reset link."</p>
                                        </div>
                                        <div class="mt-4">
                                            <div class="-mx-2 -my-1.5 flex">
                                                <A
                                                    href="/login"
                                                    class="px-2 py-1.5 rounded-md text-sm font-medium text-status-success-text hover:bg-status-success-bg focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-status-success-bg focus:ring-status-success-border"
                                                >
                                                    "Back to login"
                                                </A>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }
                            .into_view()
                    } else {
                        view! {
                            <form
                                class="mt-8 space-y-6"
                                on:submit=move |ev| {
                                    ev.prevent_default();
                                    submit_action.dispatch(email.get());
                                }
                            >
                                <div class="rounded-md shadow-sm -space-y-px">
                                    <div>
                                        <label for="email-address" class="sr-only">
                                            "Email address"
                                        </label>
                                        <input
                                            id="email-address"
                                            name="email"
                                            type="email"
                                            autocomplete="email"
                                            required
                                            class="appearance-none rounded-md relative block w-full px-3 py-2 border border-form-control-border bg-form-control-bg placeholder-form-control-placeholder text-form-control-text focus:outline-none focus:ring-2 focus:ring-action-primary-focus focus:border-action-primary-border focus:z-10 sm:text-sm"
                                            placeholder="Email address"
                                            prop:value=email
                                            on:input=move |ev| {
                                                email.set(event_target_value(&ev));
                                            }
                                        />
                                    </div>
                                </div>

                                {move || {
                                    if let Some(err) = error.get() {
                                        view! {
                                            <div class="rounded-md bg-status-error-bg p-4 text-status-error-text">
                                                <div class="flex">
                                                    <div class="ml-3">
                                                        <h3 class="text-sm font-medium text-status-error-text">
                                                            "Error"
                                                        </h3>
                                                        <div class="mt-2 text-sm text-status-error-text">
                                                            <p>{err}</p>
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                            .into_view()
                                    } else {
                                        view! {}.into_view()
                                    }
                                }}

                                <div>
                                    <button
                                        type="submit"
                                        disabled=move || pending.get()
                                        class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-action-primary-text bg-action-primary-bg hover:bg-action-primary-bg_hover focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-action-primary-focus disabled:opacity-50"
                                    >
                                        <span class="absolute left-0 inset-y-0 flex items-center pl-3">
                                            <svg
                                                class="h-5 w-5 text-action-primary-text/70 group-hover:text-action-primary-text"
                                                xmlns="http://www.w3.org/2000/svg"
                                                viewBox="0 0 20 20"
                                                fill="currentColor"
                                                aria-hidden="true"
                                            >
                                                <path
                                                    fill-rule="evenodd"
                                                    d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z"
                                                    clip-rule="evenodd"
                                                ></path>
                                            </svg>
                                        </span>
                                        {move || {
                                            if pending.get() { "Sending..." } else { "Send Reset Link" }
                                        }}

                                    </button>
                                </div>

                                <div class="text-sm text-center">
                                    <A
                                        href="/login"
                                        class="font-medium text-link hover:text-link-hover"
                                    >
                                        "Back to login"
                                    </A>
                                </div>
                            </form>
                        }
                            .into_view()
                    }
                }}

            </div>
        </div>
    }
}
