use super::repository::AdminExportRepository;
use crate::api::{ApiClient, ApiError};
use crate::utils::trigger_csv_download;
use chrono::NaiveDate;
use leptos::*;
use std::rc::Rc;

#[derive(Clone, Default)]
pub struct ExportFilters {
    pub username: String,
    pub from_date: String,
    pub to_date: String,
}

impl ExportFilters {
    fn normalized_str(value: &str) -> Option<&str> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    pub fn username_param(&self) -> Option<&str> {
        Self::normalized_str(&self.username)
    }

    pub fn start_date_param(&self) -> Option<&str> {
        Self::normalized_str(&self.from_date)
    }

    pub fn end_date_param(&self) -> Option<&str> {
        Self::normalized_str(&self.to_date)
    }

    pub fn validate(&self) -> Result<(), ApiError> {
        let from_str = self.from_date.trim();
        let to_str = self.to_date.trim();

        let parse_date = |value: &str| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| ApiError::validation("日付は YYYY-MM-DD 形式で入力してください。"))
        };

        let from = if from_str.is_empty() {
            None
        } else {
            Some(parse_date(from_str)?)
        };
        let to = if to_str.is_empty() {
            None
        } else {
            Some(parse_date(to_str)?)
        };

        if let (Some(f), Some(t)) = (from, to) {
            if f > t {
                return Err(ApiError::validation(
                    "From は To 以前の日付を指定してください。",
                ));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct AdminExportViewModel {
    pub error: RwSignal<Option<ApiError>>,
    pub preview: RwSignal<Option<String>>,
    pub filename: RwSignal<String>,
    pub username: RwSignal<String>,
    pub from_date: RwSignal<String>,
    pub to_date: RwSignal<String>,
    pub use_specific_user: RwSignal<bool>,
    pub users_resource: Resource<bool, Result<Vec<crate::api::UserResponse>, ApiError>>,
    pub export_action: Action<ExportFilters, Result<serde_json::Value, ApiError>>,
}

pub fn use_admin_export_view_model() -> AdminExportViewModel {
    let api = use_context::<ApiClient>().unwrap_or_else(ApiClient::new);
    let repo = AdminExportRepository::new_with_client(Rc::new(api));

    let error = create_rw_signal(None::<ApiError>);
    let preview = create_rw_signal(None::<String>);
    let filename = create_rw_signal(String::new());
    let username = create_rw_signal(String::new());
    let from_date = create_rw_signal(String::new());
    let to_date = create_rw_signal(String::new());
    let use_specific_user = create_rw_signal(false);

    let repo_users = repo.clone();
    let users_resource = create_resource(
        || true,
        move |_| {
            let repo = repo_users.clone();
            async move { repo.fetch_users().await }
        },
    );

    let repo_export = repo.clone();
    let export_action = create_action(move |filters: &ExportFilters| {
        let repo = repo_export.clone();
        let filters = filters.clone();
        async move {
            repo.export_data_filtered(
                filters.username_param(),
                filters.start_date_param(),
                filters.end_date_param(),
            )
            .await
        }
    });

    create_effect(move |_| {
        if let Some(result) = export_action.value().get() {
            match result {
                Ok(payload) => {
                    let fname = payload
                        .get("filename")
                        .and_then(|s| s.as_str())
                        .unwrap_or("export.csv");
                    let csv = payload
                        .get("csv_data")
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    filename.set(fname.to_string());
                    preview.set(Some(csv.chars().take(2000).collect()));
                    let _ = trigger_csv_download(fname, csv);
                    error.set(None);
                }
                Err(message) => {
                    error.set(Some(message));
                    preview.set(None);
                }
            }
        }
    });

    AdminExportViewModel {
        error,
        preview,
        filename,
        username,
        from_date,
        to_date,
        use_specific_user,
        users_resource,
        export_action,
    }
}

pub fn needs_specific_user_selection(use_specific_user: bool, username: &str) -> bool {
    use_specific_user && username.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::needs_specific_user_selection;

    #[test]
    fn specific_user_requires_selection() {
        assert!(needs_specific_user_selection(true, ""));
        assert!(needs_specific_user_selection(true, "   "));
        assert!(!needs_specific_user_selection(true, "admin"));
        assert!(!needs_specific_user_selection(false, ""));
    }
}
