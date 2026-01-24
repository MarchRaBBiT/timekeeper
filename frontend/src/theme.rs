#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    use web_sys;

    const DARK_CLASS: &str = "dark";

    fn update_html_class(html: &web_sys::Element, is_dark: bool) {
        if let Some(list) = html.class_list().ok() {
            if is_dark {
                let _ = list.add_1(DARK_CLASS);
            } else {
                let _ = list.remove_1(DARK_CLASS);
            }
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
            closure.forget();
        } else {
            set_from_query(false);
        }
    }
}

pub use wasm::init as init_system_theme;

#[cfg(not(target_arch = "wasm32"))]
pub fn init_system_theme() {}
