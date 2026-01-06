use crate::api::{ApiClient, ApiError};

pub async fn change_password(
    client: ApiClient,
    current: String,
    new: String,
) -> Result<(), ApiError> {
    client.change_password(current, new).await
}
