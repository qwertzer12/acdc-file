mod docker_hub;
mod ranking;
mod repo_resolution;

pub use docker_hub::list_docker_hub_tags;
pub use ranking::{filter_tags, search_docker_hub_tags};
pub use repo_resolution::{
    auto_search_docker_hub_tags,
    resolve_docker_hub_repository,
};

pub type ApiError = Box<dyn std::error::Error + Send + Sync>;

async fn run_test(repo: &str) -> Result<(), ApiError> {
    let namespace = "library";
    let tags = list_docker_hub_tags(namespace, repo).await?;
    let fuzzy_preview = search_docker_hub_tags(namespace, repo, "alp", 10).await?;

    println!("repo={}/{}", namespace, repo);
    println!("Found {} tags", tags.len());
    println!("\nTop matches for query 'alp':");
    for tag in fuzzy_preview {
        println!("  {tag}");
    }

    println!("\nFirst 20 raw tags:");

    for t in tags.iter().take(20) {
        println!("{t}");
    }

    Ok(())
}

pub fn test(repo: &str) -> Result<(), ApiError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(run_test(repo))
}
