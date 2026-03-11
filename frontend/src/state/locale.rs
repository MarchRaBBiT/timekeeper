use leptos::*;

use crate::utils::storage::{get_local_storage_item, set_local_storage_item};

const LOCALE_STORAGE_KEY: &str = "timekeeper.locale";
const DEFAULT_LOCALE: &str = "en";
const JAPANESE_LOCALE: &str = "ja";
const SUPPORTED_LOCALES: [&str; 2] = [DEFAULT_LOCALE, JAPANESE_LOCALE];

type LocaleContext = (ReadSignal<LocaleState>, WriteSignal<LocaleState>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocaleState {
    pub current: String,
}

impl Default for LocaleState {
    fn default() -> Self {
        Self {
            current: DEFAULT_LOCALE.to_string(),
        }
    }
}

fn create_locale_context() -> LocaleContext {
    let initial = resolve_initial_locale(
        load_persisted_locale().as_deref(),
        browser_locale().as_deref(),
    );
    rust_i18n::set_locale(&initial);
    create_signal(LocaleState { current: initial })
}

#[component]
pub fn LocaleProvider(children: Children) -> impl IntoView {
    let ctx = create_locale_context();
    let locale = Signal::derive(move || ctx.0.get().current);

    create_effect(move |_| {
        let selected = locale.get();
        rust_i18n::set_locale(&selected);
    });

    provide_context::<LocaleContext>(ctx);
    view! { <>{children()}</> }
}

pub fn use_locale() -> LocaleContext {
    use_context::<LocaleContext>().unwrap_or_else(|| {
        eprintln!("LocaleProvider is not mounted; using fallback locale context");
        create_locale_context()
    })
}

pub fn set_current_locale(set_locale: WriteSignal<LocaleState>, locale: &str) {
    let normalized = normalize_locale(locale);
    // If localStorage is unavailable, still switch the in-memory locale for this page load.
    let _ = persist_locale(normalized);
    set_locale.update(|state| state.current = normalized.to_string());
}

pub fn resolve_initial_locale(persisted: Option<&str>, browser: Option<&str>) -> String {
    persisted
        .map(normalize_locale)
        .or_else(|| browser.map(normalize_locale))
        .unwrap_or(DEFAULT_LOCALE)
        .to_string()
}

pub fn normalize_locale(locale: &str) -> &'static str {
    if locale
        .trim()
        .to_ascii_lowercase()
        .starts_with(JAPANESE_LOCALE)
    {
        JAPANESE_LOCALE
    } else {
        DEFAULT_LOCALE
    }
}

pub fn available_locales() -> Vec<String> {
    let available = rust_i18n::available_locales!()
        .into_iter()
        .map(|locale| locale.into_owned())
        .collect::<Vec<_>>();
    SUPPORTED_LOCALES
        .into_iter()
        .filter(|locale| available.iter().any(|entry| entry == locale))
        .map(str::to_string)
        .collect()
}

fn load_persisted_locale() -> Option<String> {
    get_local_storage_item(LOCALE_STORAGE_KEY).ok().flatten()
}

fn persist_locale(locale: &str) -> Result<(), String> {
    set_local_storage_item(LOCALE_STORAGE_KEY, normalize_locale(locale))
}

#[cfg(target_arch = "wasm32")]
fn browser_locale() -> Option<String> {
    web_sys::window()
        .map(|window| window.navigator().language())
        .filter(|locale| !locale.trim().is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
fn browser_locale() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::helpers::set_test_locale;
    use leptos::create_runtime;

    fn with_runtime<T>(test: impl FnOnce() -> T) -> T {
        let runtime = create_runtime();
        let result = test();
        runtime.dispose();
        result
    }

    #[test]
    fn normalize_locale_maps_to_supported_values() {
        assert_eq!(normalize_locale("ja"), "ja");
        assert_eq!(normalize_locale("ja-JP"), "ja");
        assert_eq!(normalize_locale("en"), "en");
        assert_eq!(normalize_locale("en-US"), "en");
        assert_eq!(normalize_locale("fr"), "en");
    }

    #[test]
    fn resolve_initial_locale_prefers_persisted_value() {
        assert_eq!(resolve_initial_locale(Some("ja-JP"), Some("en-US")), "ja");
        assert_eq!(resolve_initial_locale(Some("en-US"), Some("ja-JP")), "en");
    }

    #[test]
    fn resolve_initial_locale_uses_browser_locale_when_not_persisted() {
        assert_eq!(resolve_initial_locale(None, Some("ja-JP")), "ja");
        assert_eq!(resolve_initial_locale(None, Some("fr-FR")), "en");
    }

    #[test]
    fn create_locale_context_eagerly_syncs_global_locale() {
        let _locale = set_test_locale("ja");
        with_runtime(|| {
            let (read, _write) = create_locale_context();
            assert_eq!(read.get().current, "en");
            assert_eq!(rust_i18n::locale().to_string(), "en");
        });
    }

    #[test]
    fn use_locale_returns_default_without_context() {
        with_runtime(|| {
            let (read, _write) = use_locale();
            assert_eq!(read.get().current, "en");
        });
    }

    #[test]
    fn set_current_locale_normalizes_value() {
        with_runtime(|| {
            let (read, write) = create_signal(LocaleState::default());
            set_current_locale(write, "ja-JP");
            assert_eq!(read.get().current, "ja");
        });
    }

    #[test]
    fn available_locales_contains_en_and_ja() {
        let locales = available_locales();
        assert_eq!(locales, vec!["en".to_string(), "ja".to_string()]);
    }
}
