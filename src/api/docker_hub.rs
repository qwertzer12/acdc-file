use std::collections::BTreeMap;

use serde::de::IgnoredAny;
use serde::Deserialize;

use super::{ApiError, http_client};

#[derive(Debug, Deserialize)]
struct TokenResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct TagsListResponse {
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ManifestEnvelope {
    config: Option<ManifestConfig>,
    manifests: Option<Vec<ManifestDescriptor>>,
}

#[derive(Debug, Deserialize)]
struct ManifestConfig {
    digest: String,
}

#[derive(Debug, Deserialize)]
struct ManifestDescriptor {
    digest: String,
    platform: Option<ManifestPlatform>,
}

#[derive(Debug, Deserialize)]
struct ManifestPlatform {
    architecture: Option<String>,
    os: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConfigBlob {
    config: Option<ImageConfig>,
}

#[derive(Debug, Deserialize)]
struct ImageConfig {
    #[serde(rename = "ExposedPorts")]
    exposed_ports: Option<BTreeMap<String, IgnoredAny>>,
}

async fn get_registry_token(image: &str) -> Result<String, ApiError> {
    let auth_url = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{image}:pull"
    );
    let token_resp: TokenResponse = http_client()
        .get(auth_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(token_resp.token)
}

fn parse_port_number(exposed_port_key: &str) -> Option<u16> {
    let prefix = exposed_port_key.split('/').next()?;
    prefix.parse::<u16>().ok()
}

fn preferred_index_manifest(manifests: &[ManifestDescriptor]) -> Option<&ManifestDescriptor> {
    manifests
        .iter()
        .find(|descriptor| {
            let platform = descriptor.platform.as_ref();
            matches!(
                (
                    platform.and_then(|p| p.os.as_deref()),
                    platform.and_then(|p| p.architecture.as_deref())
                ),
                (Some("linux"), Some("amd64"))
            )
        })
        .or_else(|| manifests.first())
}

async fn fetch_manifest(image: &str, reference: &str, token: &str) -> Result<ManifestEnvelope, ApiError> {
    let manifest_url = format!("https://registry-1.docker.io/v2/{image}/manifests/{reference}");
    let manifest = http_client()
        .get(manifest_url)
        .bearer_auth(token)
        .header(
            reqwest::header::ACCEPT,
            "application/vnd.oci.image.index.v1+json,application/vnd.docker.distribution.manifest.list.v2+json,application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.v2+json",
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(manifest)
}

pub async fn list_docker_hub_tags(namespace: &str, repo: &str) -> Result<Vec<String>, ApiError> {
    let image = format!("{namespace}/{repo}");
    let token = get_registry_token(&image).await?;

    let tags_url = format!("https://registry-1.docker.io/v2/{image}/tags/list");
    let tags_resp: TagsListResponse = http_client()
        .get(tags_url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(tags_resp.tags.unwrap_or_default())
}

pub async fn list_docker_hub_exposed_ports(
    namespace: &str,
    repo: &str,
    tag: &str,
) -> Result<Vec<u16>, ApiError> {
    let image = format!("{namespace}/{repo}");
    let token = get_registry_token(&image).await?;

    let mut manifest = fetch_manifest(&image, tag, &token).await?;
    if manifest.config.is_none() {
        if let Some(manifests) = manifest.manifests.as_ref() {
            if let Some(chosen) = preferred_index_manifest(manifests) {
                manifest = fetch_manifest(&image, &chosen.digest, &token).await?;
            }
        }
    }

    let config_digest = match manifest.config {
        Some(config) => config.digest,
        None => return Ok(Vec::new()),
    };

    let blob_url = format!("https://registry-1.docker.io/v2/{image}/blobs/{config_digest}");
    let blob: ConfigBlob = http_client()
        .get(blob_url)
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut ports: Vec<u16> = blob
        .config
        .and_then(|config| config.exposed_ports)
        .map(|ports| {
            ports
                .keys()
                .filter_map(|port| parse_port_number(port))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    ports.sort_unstable();
    ports.dedup();
    Ok(ports)
}
