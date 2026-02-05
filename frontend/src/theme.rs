#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::cell::RefCell;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys;

    const DARK_CLASS: &str = "dark";

    struct ThemeWatcher {
        _media_query: web_sys::MediaQueryList,
        _listener: Closure<dyn FnMut(web_sys::MediaQueryListEvent)>,
    }

    thread_local! {
        static THEME_WATCHER: RefCell<Option<ThemeWatcher>> = RefCell::new(None);
    }

    fn update_html_class(html: &web_sys::Element, is_dark: bool) {
        let list = html.class_list();
        if is_dark {
            let _ = list.add_1(DARK_CLASS);
        } else {
            let _ = list.remove_1(DARK_CLASS);
        }
    }

    pub fn init() {
        let window = match web_sys::window() {
            Some(win) => win,
            None => return,
        };

        let document = match window.document() {
            Some(doc) => doc,
            None => return,
        };

        let html = match document.document_element() {
            Some(node) => node,
            None => return,
        };

        let media_query = window
            .match_media("(prefers-color-scheme: dark)")
            .ok()
            .flatten();

        let set_from_query = |matches: bool| {
            update_html_class(&html, matches);
        };

        if let Some(list) = media_query {
            set_from_query(list.matches());
            let html_clone = html.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::MediaQueryListEvent| {
                update_html_class(&html_clone, event.matches());
            }) as Box<dyn FnMut(_)>);
            if list
                .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
                .is_err()
            {
                // ignore
            }
            THEME_WATCHER.with(|slot| {
                *slot.borrow_mut() = Some(ThemeWatcher {
                    _media_query: list,
                    _listener: closure,
                });
            });
        } else {
            set_from_query(false);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use wasm_bindgen_test::*;

        wasm_bindgen_test_configure!(run_in_browser);

        #[wasm_bindgen_test]
        fn update_html_class_toggles_dark_mode() {
            let document = web_sys::window().unwrap().document().unwrap();
            let element = document.create_element("div").unwrap();

            update_html_class(&element, true);
            assert!(element.class_list().contains(DARK_CLASS));

            update_html_class(&element, false);
            assert!(!element.class_list().contains(DARK_CLASS));
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::init as init_system_theme;

#[cfg(not(target_arch = "wasm32"))]
pub fn init_system_theme() {}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod host_tests {
    use super::init_system_theme;

    #[test]
    fn init_system_theme_is_noop_on_host() {
        init_system_theme();
    }
}
