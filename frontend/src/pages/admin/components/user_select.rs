use crate::{api::UserResponse, components::layout::ErrorMessage};
use leptos::{ev, *};

pub type UsersResource = Resource<bool, Result<Vec<UserResponse>, String>>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum UserSelectValue {
    #[default]
    Id,
    Username,
}

#[component]
pub fn AdminUserSelect(
    users: UsersResource,
    selected: RwSignal<String>,
    #[prop(optional_no_strip)] label: Option<String>,
    #[prop(default = "ユーザーを選択".to_string())] placeholder: String,
    #[prop(default = UserSelectValue::Id)] value_kind: UserSelectValue,
) -> impl IntoView {
    let fetch_error = create_rw_signal(None::<String>);

    {
        let fetch_error = fetch_error.clone();
        create_effect(move |_| {
            if let Some(result) = users.get() {
                match result {
                    Ok(_) => fetch_error.set(None),
                    Err(err) => {
                        fetch_error.set(Some(format!("ユーザー一覧の取得に失敗しました: {}", err)))
                    }
                }
            }
        });
    }

    let on_change = {
        let selected = selected.clone();
        move |ev: ev::Event| {
            selected.set(event_target_value(&ev));
        }
    };

    let has_label = label.as_ref().map(|l| !l.is_empty()).unwrap_or(false);
    let label_value = label.unwrap_or_default();

    let options_view = move || match users.get() {
        None => {
            view! { <option value="" disabled>{"ユーザーを読み込み中..."}</option> }.into_view()
        }
        Some(Err(_)) => {
            view! { <option value="" disabled>{"ユーザーの取得に失敗しました"}</option> }
                .into_view()
        }
        Some(Ok(list)) => view! {
            <For
                each=move || list.clone()
                key=|user| user.id.clone()
                children=move |user| {
                    let label = format!("{} ({})", user.full_name, user.username);
                    let value = match value_kind {
                        UserSelectValue::Id => user.id.clone(),
                        UserSelectValue::Username => user.username.clone(),
                    };
                    view! { <option value=value>{label}</option> }
                }
            />
        }
        .into_view(),
    };

    view! {
        <div class="space-y-1">
            <Show when=move || has_label>
                <label class="block text-sm font-medium text-gray-700">
                    {label_value.clone()}
                </label>
            </Show>
            <select
                class="w-full border rounded px-2 py-1 bg-white disabled:opacity-50"
                on:change=on_change
                prop:value=selected
            >
                <option value="">{placeholder.clone()}</option>
                {move || options_view()}
            </select>
            <Show when=move || fetch_error.get().is_some()>
                <ErrorMessage message={fetch_error.get().unwrap_or_default()} />
            </Show>
        </div>
    }
}
