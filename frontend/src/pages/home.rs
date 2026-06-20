use leptos::*;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-surface">
            <div class="max-w-7xl mx-auto py-12 px-4 sm:px-6 lg:px-8">
                <div class="text-center">
                    <h1 class="text-4xl font-extrabold text-fg sm:text-5xl lg:text-6xl">
                        "Timekeeper"
                    </h1>
                    <p class="mt-3 max-w-md mx-auto text-base text-fg-muted sm:text-lg lg:mt-5 lg:text-xl lg:max-w-3xl">
                        {rust_i18n::t!("pages.home.tagline")}
                    </p>
                    <div class="mt-5 max-w-md mx-auto sm:flex sm:justify-center lg:mt-8">
                        <div class="rounded-md shadow">
                            <a href="/login" class="w-full flex items-center justify-center px-8 py-3 border border-transparent text-base font-medium rounded-md text-action-primary-text bg-action-primary-bg hover:bg-action-primary-bg-hover lg:py-4 lg:text-lg lg:px-10">
                                {rust_i18n::t!("pages.home.actions.login")}
                            </a>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::*;
    use crate::test_support::{helpers::set_test_locale, ssr::render_to_string};

    #[test]
    fn home_page_resolves_copy_from_the_active_locale() {
        let _locale = set_test_locale("en");
        let html = render_to_string(|| view! { <HomePage /> });

        assert!(html.contains(rust_i18n::t!("pages.home.tagline").as_ref()));
        assert!(html.contains(rust_i18n::t!("pages.home.actions.login").as_ref()));
        assert!(!html.contains("少人数向けの勤怠管理システム"));
    }
}
