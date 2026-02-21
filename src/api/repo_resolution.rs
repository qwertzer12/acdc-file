use serde::Deserialize;

use super::{ApiError, http_client, search_docker_hub_tags};

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

    let encoded_term = input.replace(' ', "%20");
    let search_url = format!(
        "https://hub.docker.com/v2/search/repositories/?query={encoded_term}&page_size=25"
    );
    let response: RepoSearchResponse = http_client()
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
        parse_repo_name(&result.repo_name, result.is_official)
            .map(|(namespace, repo)| ResolvedRepository { namespace, repo })
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
