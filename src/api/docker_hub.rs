use serde::Deserialize;

use super::ApiError;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct TagsListResponse {
    tags: Option<Vec<String>>,
}

pub async fn list_docker_hub_tags(namespace: &str, repo: &str) -> Result<Vec<String>, ApiError> {
    let image = format!("{namespace}/{repo}");

    let auth_url = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{image}:pull"
    );
    let token_resp: TokenResponse = reqwest::get(auth_url).await?.json().await?;

    let tags_url = format!("https://registry-1.docker.io/v2/{image}/tags/list");
    let client = reqwest::Client::new();
    let tags_resp: TagsListResponse = client
        .get(tags_url)
        .bearer_auth(token_resp.token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(tags_resp.tags.unwrap_or_default())
}
