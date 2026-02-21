use std::cmp::Ordering;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};

use super::{ApiError, list_docker_hub_tags};

fn parse_version_prefix(tag: &str) -> Option<Vec<u32>> {
    let normalized = tag.strip_prefix('v').unwrap_or(tag);
    let mut parts = Vec::new();
    let mut current = String::new();

    for ch in normalized.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
            continue;
        }

        if ch == '.' {
            if current.is_empty() {
                break;
            }

            match current.parse::<u32>() {
                Ok(value) => {
                    parts.push(value);
                    current.clear();
                }
                Err(_) => return None,
            }

            continue;
        }

        break;
    }

    if current.is_empty() {
        if parts.is_empty() {
            None
        } else {
            Some(parts)
        }
    } else {
        match current.parse::<u32>() {
            Ok(value) => {
                parts.push(value);
                Some(parts)
            }
            Err(_) => None,
        }
    }
}

fn compare_version_desc(left_tag: &str, right_tag: &str) -> Ordering {
    match (parse_version_prefix(left_tag), parse_version_prefix(right_tag)) {
        (Some(left), Some(right)) => {
            let max_len = left.len().max(right.len());
            for index in 0..max_len {
                let left_part = left.get(index).copied().unwrap_or(0);
                let right_part = right.get(index).copied().unwrap_or(0);
                match right_part.cmp(&left_part) {
                    Ordering::Equal => continue,
                    other => return other,
                }
            }
            Ordering::Equal
        }
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn is_prerelease(tag: &str) -> bool {
    let lower = tag.to_ascii_lowercase();
    let has_digit_letter = lower
        .as_bytes()
        .windows(2)
        .any(|pair| pair[0].is_ascii_digit() && pair[1].is_ascii_alphabetic());

    lower.contains("-rc")
        || lower.contains("rc")
        || lower.contains("alpha")
        || lower.contains("beta")
        || lower.contains("preview")
        || lower.contains("dev")
        || has_digit_letter
}

fn compare_tag_importance_desc(left: &str, right: &str) -> Ordering {
    let left_lower = left.to_ascii_lowercase();
    let right_lower = right.to_ascii_lowercase();

    let left_is_latest = left_lower == "latest";
    let right_is_latest = right_lower == "latest";
    match (left_is_latest, right_is_latest) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }

    match (is_prerelease(left), is_prerelease(right)) {
        (false, true) => return Ordering::Less,
        (true, false) => return Ordering::Greater,
        _ => {}
    }

    let version_cmp = compare_version_desc(left, right);
    if version_cmp != Ordering::Equal {
        return version_cmp;
    }

    let left_variant = left.contains('-');
    let right_variant = right.contains('-');
    match (left_variant, right_variant) {
        (false, true) => Ordering::Less,
        (true, false) => Ordering::Greater,
        _ => left.cmp(right),
    }
}

fn rank_tags(tags: &[String], query: &str, limit: usize) -> Vec<String> {
    if query.trim().is_empty() {
        let mut ordered: Vec<&str> = tags.iter().map(String::as_str).collect();
        ordered.sort_by(|left, right| compare_tag_importance_desc(left, right));
        return ordered
            .into_iter()
            .take(limit)
            .map(ToString::to_string)
            .collect();
    }

    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);

    let mut matched = pattern.match_list(tags.iter().map(String::as_str), &mut matcher);
    matched.sort_by(|(left_tag, left_score), (right_tag, right_score)| {
        right_score
            .cmp(left_score)
            .then_with(|| compare_tag_importance_desc(left_tag, right_tag))
            .then_with(|| left_tag.cmp(right_tag))
    });

    matched
        .into_iter()
        .take(limit)
        .map(|(tag, _)| tag.to_string())
        .collect()
}

pub fn filter_tags(tags: &[String], query: &str, limit: usize) -> Vec<String> {
    let effective_limit = limit.max(1);
    rank_tags(tags, query, effective_limit)
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
