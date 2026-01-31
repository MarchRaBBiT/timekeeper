use leptos::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
}

impl ButtonVariant {
    pub fn classes(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "bg-action-primary-bg hover:bg-action-primary-bg-hover text-action-primary-text shadow-sm focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action-primary-focus",
        }
    }
}

#[component]
pub fn Button(
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional, into)] class: String,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional, into)] loading: MaybeSignal<bool>,
    #[prop(attrs)] attributes: Vec<(&'static str, Attribute)>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            class=move || {
                format!(
                    "inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-semibold transition-colors duration-200 disabled:opacity-50 disabled:cursor-not-allowed {} {}",
                    variant.classes(),
                    class
                )
            }
            disabled=move || disabled.get() || loading.get()
            {..attributes}
        >
            <Show when=move || loading.get()>
                <span class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
            </Show>
            {children()}
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_variant_includes_primary_class() {
        let classes = ButtonVariant::Primary.classes();
        assert!(classes.contains("bg-action-primary-bg"));
    }
}
