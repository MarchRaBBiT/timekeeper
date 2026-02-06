use crate::state::theme::{use_theme, Theme};
use leptos::*;

#[component]
pub fn ThemeToggle() -> impl IntoView {
    let theme_state = use_theme();
    let current_theme = theme_state.current();

    let on_click = move |_| {
        theme_state.toggle();
    };

    view! {
        <button
            type="button"
            class="relative inline-flex h-6 w-11 items-center rounded-full bg-gray-200 dark:bg-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
            on:click=on_click
            aria-label="Toggle theme"
        >
            <span class="sr-only">"Toggle theme"</span>

            <span
                class=move || {
                    if current_theme.get() == Theme::Dark {
                        "translate-x-6 bg-primary-600"
                    } else {
                        "translate-x-1 bg-white"
                    }
                }
                class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform shadow-theme-switch"
            />

            <span
                class=move || {
                    if current_theme.get() == Theme::Dark {
                        "opacity-100"
                    } else {
                        "opacity-0"
                    }
                }
                class="absolute left-1 top-1/2 -translate-y-1/2 text-xs text-gray-400 transition-opacity"
            >
                <i class="fas fa-moon"></i>
            </span>

            <span
                class=move || {
                    if current_theme.get() == Theme::Light || current_theme.get() == Theme::HighContrast {
                        "opacity-100"
                    } else {
                        "opacity-0"
                    }
                }
                class="absolute right-1 top-1/2 -translate-y-1/2 text-xs text-yellow-500 transition-opacity"
            >
                <i class="fas fa-sun"></i>
            </span>
        </button>
    }
}

#[component]
pub fn ThemeProvider(children: Children) -> impl IntoView {
    let theme_state = crate::state::theme::provide_theme();

    view! {
        <div class=move || theme_state.current().get().as_class()>
            {children()}
        </div>
    }
}
