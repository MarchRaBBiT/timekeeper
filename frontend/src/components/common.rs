use leptos::*;

// TODO: リファクタリング後に使用可否を判断
// - 使う可能性: あり
// - 想定機能: 共通ボタンのバリアント切替
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Danger,
    Ghost,
}

impl ButtonVariant {
    pub fn classes(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "bg-action-primary-bg hover:bg-action-primary-bg_hover text-action-primary-text shadow-sm focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action-primary-focus",
            ButtonVariant::Secondary => "bg-action-secondary-bg hover:bg-action-secondary-bg_hover text-action-secondary-text shadow-sm focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action-secondary-focus",
            ButtonVariant::Danger => "bg-action-danger-bg hover:bg-action-danger-bg_hover text-action-danger-text shadow-sm focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-action-danger-focus",
            ButtonVariant::Ghost => "bg-transparent hover:bg-action-ghost-bg_hover text-action-ghost-text",
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
