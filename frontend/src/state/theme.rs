use leptos::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    HighContrast,
}

impl Default for Theme {
    fn default() -> Self {
        if web_sys::window()
            .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok())
            .map(|m| m.matches())
            .unwrap_or(false)
        {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

impl Theme {
    pub fn as_class(&self) -> &'static str {
        match self {
            Theme::Light => "",
            Theme::Dark => "dark",
            Theme::HighContrast => "contrast-high",
        }
    }
}

#[derive(Clone)]
pub struct ThemeState {
    pub theme: RwSignal<Theme>,
}

impl ThemeState {
    pub fn new() -> Self {
        let theme = create_rw_signal(Theme::default());
        Self { theme }
    }

    pub fn set_theme(&self, theme: Theme) {
        self.theme.set(theme);
        self.apply_to_dom();
    }

    pub fn toggle(&self) {
        let new_theme = match self.theme.get() {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
            Theme::HighContrast => Theme::Light,
        };
        self.set_theme(new_theme);
    }

    fn apply_to_dom(&self) {
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            let class_list = document.document_element().unwrap().class_list();
            class_list.remove_1("dark");
            class_list.remove_1("contrast-high");
            class_list.add_1(self.theme.get().as_class());
        }
    }

    pub fn current(&self) -> ReadSignal<Theme> {
        self.theme.read_only()
    }
}

pub fn use_theme() -> ThemeState {
    expect_context::<ThemeState>().expect("ThemeState must be provided in app root")
}

pub fn provide_theme() -> ThemeState {
    let state = ThemeState::new();
    provide_context(state.clone());
    state.apply_to_dom();

    if let Some(window) = web_sys::window() {
        if let Ok(media) = window.match_media("(prefers-color-scheme: dark)") {
            let state_clone = state.clone();
            let closure = Closure::wrap(Box::new(move |_: web_sys::MediaQueryListEvent| {
                let system_dark = window
                    .match_media("(prefers-color-scheme: dark)")
                    .map(|m| m.matches())
                    .unwrap_or(false);

                let current = state_clone.theme.get();
            }));

            media
                .add_event_listener_with_callback("change", &closure)
                .unwrap();
            closure.forget();
        }
    }

    state
}
