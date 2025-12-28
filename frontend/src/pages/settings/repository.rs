use crate::api::ApiClient;

pub async fn change_password(
    client: ApiClient,
    current: String,
    new: String,
) -> Result<(), String> {
    client.change_password(current, new).await
}
