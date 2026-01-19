use crate::api::ApiClient;
use leptos::*;
use leptos_router::*;

#[component]
pub fn ForgotPasswordPanel() -> impl IntoView {
    let api = expect_context::<ApiClient>();
    let email = create_rw_signal(String::new());
    let error = create_rw_signal(Option::<String>::None);
    let success = create_rw_signal(Option::<String>::None);

    let submit_action = create_action(move |e: &String| {
        let email = e.clone();
        let api = api.clone();
        async move { api.request_password_reset(email).await }
    });

    let pending = submit_action.pending();

    create_effect(move |_| {
        if let Some(result) = submit_action.value().get() {
            match result {
                Ok(resp) => {
                    success.set(Some(resp.message));
                    error.set(None);
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                    success.set(None);
                }
            }
        }
    });

    view! {
        <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
            <div class="max-w-md w-full space-y-8">
                <div>
                    <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
                        "Reset your password"
                    </h2>
                    <p class="mt-2 text-center text-sm text-gray-600">
                        "Enter your email address and we'll send you a link to reset your password."
                    </p>
                </div>

                {move || {
                    if let Some(msg) = success.get() {
                        view! {
                            <div class="rounded-md bg-green-50 p-4">
                                <div class="flex">
                                    <div class="flex-shrink-0">
                                        <svg
                                            class="h-5 w-5 text-green-400"
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
                                        <h3 class="text-sm font-medium text-green-800">{msg}</h3>
                                        <div class="mt-2 text-sm text-green-700">
                                            <p>"Check your email for the reset link."</p>
                                        </div>
                                        <div class="mt-4">
                                            <div class="-mx-2 -my-1.5 flex">
                                                <A
                                                    href="/login"
                                                    class="px-2 py-1.5 rounded-md text-sm font-medium text-green-800 hover:bg-green-100 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-green-50 focus:ring-green-600"
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
                                            class="appearance-none rounded-md relative block w-full px-3 py-2 border border-gray-300 placeholder-gray-500 text-gray-900 focus:outline-none focus:ring-blue-500 focus:border-blue-500 focus:z-10 sm:text-sm"
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
                                            <div class="rounded-md bg-red-50 p-4">
                                                <div class="flex">
                                                    <div class="ml-3">
                                                        <h3 class="text-sm font-medium text-red-800">
                                                            "Error"
                                                        </h3>
                                                        <div class="mt-2 text-sm text-red-700">
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
                                        class="group relative w-full flex justify-center py-2 px-4 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
                                    >
                                        <span class="absolute left-0 inset-y-0 flex items-center pl-3">
                                            <svg
                                                class="h-5 w-5 text-blue-500 group-hover:text-blue-400"
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
                                        class="font-medium text-blue-600 hover:text-blue-500"
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
