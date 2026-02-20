use clap::builder::Str;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct TagsListResponse {
    name: String,
    tags: Option<Vec<String>>,
}

pub async fn list_docker_hub_tags(
    namespace: &str,
    repo: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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

#[tokio::main]
pub async fn test(repo: &str) -> Result<(), Box<dyn std::error::Error>> {
    let namespace = "library";
    let tags = list_docker_hub_tags(namespace, repo).await?;
    println!("repo={}/{}", namespace, repo);
    println!("Found {} tags", tags.len());

    for t in tags.iter().take(20) {
        println!("{t}");
    }

    Ok(())
}
