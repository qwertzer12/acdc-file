use nucleo_matcher::{Config, Matcher};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use serde::Deserialize;

type ApiError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct TagsListResponse {
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RepoSearchResponse {
    results: Vec<RepoSearchResult>,
}

#[derive(Debug, Deserialize)]
struct RepoSearchResult {
    repo_name: String,
    #[serde(default)]
    pull_count: u64,
    #[serde(default)]
    star_count: u64,
    #[serde(default)]
    is_official: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedRepository {
    pub namespace: String,
    pub repo: String,
}

pub async fn list_docker_hub_tags(
    namespace: &str,
    repo: &str,
) -> Result<Vec<String>, ApiError> {
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

fn rank_tags(tags: &[String], query: &str, limit: usize) -> Vec<String> {
    if query.trim().is_empty() {
        return tags.iter().take(limit).cloned().collect();
    }

    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);

    pattern
        .match_list(tags.iter().map(String::as_str), &mut matcher)
        .into_iter()
        .take(limit)
        .map(|(tag, _)| tag.to_string())
        .collect()
}

pub async fn search_docker_hub_tags(
    namespace: &str,
    repo: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<String>, ApiError> {
    let tags = list_docker_hub_tags(namespace, repo).await?;
    let effective_limit = limit.max(1);
    Ok(rank_tags(&tags, query, effective_limit))
}

fn parse_repo_name(repo_name: &str, is_official: bool) -> Option<(String, String)> {
    if let Some((namespace, repo)) = repo_name.split_once('/') {
        if !namespace.trim().is_empty() && !repo.trim().is_empty() {
            return Some((namespace.trim().to_string(), repo.trim().to_string()));
        }
    }

    if is_official && !repo_name.trim().is_empty() {
        return Some(("library".to_string(), repo_name.trim().to_string()));
    }

    None
}

fn score_repo_candidate(result: &RepoSearchResult, term: &str) -> i64 {
    let term_lower = term.to_ascii_lowercase();
    let (_, repo) = match parse_repo_name(&result.repo_name, result.is_official) {
        Some(value) => value,
        None => return i64::MIN / 2,
    };
    let repo_lower = repo.to_ascii_lowercase();

    let mut score = 0i64;

    if repo_lower == term_lower {
        score += 2_000;
    } else if repo_lower.starts_with(&term_lower) {
        score += 1_200;
    } else if repo_lower.contains(&term_lower) {
        score += 600;
    }

    if result.is_official || result.repo_name == repo {
        score += 1_500;
    }

    score += ((result.star_count as f64).sqrt() * 6.0) as i64;
    score += ((result.pull_count as f64).ln_1p() * 10.0) as i64;

    score
}

pub async fn resolve_docker_hub_repository(term: &str) -> Result<Option<ResolvedRepository>, ApiError> {
    let input = term.trim();
    if input.is_empty() {
        return Ok(None);
    }

    if let Some((namespace, repo)) = input.split_once('/') {
        if !namespace.trim().is_empty() && !repo.trim().is_empty() {
            return Ok(Some(ResolvedRepository {
                namespace: namespace.trim().to_string(),
                repo: repo.trim().to_string(),
            }));
        }
    }

    let client = reqwest::Client::new();
    let encoded_term = input.replace(' ', "%20");
    let search_url = format!(
        "https://hub.docker.com/v2/search/repositories/?query={encoded_term}&page_size=25"
    );
    let response: RepoSearchResponse = client
        .get(search_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let best = response
        .results
        .iter()
        .max_by_key(|result| score_repo_candidate(result, input));

    Ok(best.and_then(|result| {
        parse_repo_name(&result.repo_name, result.is_official).map(|(namespace, repo)| {
            ResolvedRepository { namespace, repo }
        })
    }))
}

pub async fn auto_search_docker_hub_tags(
    image_term: &str,
    tag_query: &str,
    limit: usize,
) -> Result<Option<(ResolvedRepository, Vec<String>)>, ApiError> {
    let resolved = match resolve_docker_hub_repository(image_term).await? {
        Some(value) => value,
        None => return Ok(None),
    };

    let tags = search_docker_hub_tags(&resolved.namespace, &resolved.repo, tag_query, limit).await?;
    Ok(Some((resolved, tags)))
}

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
