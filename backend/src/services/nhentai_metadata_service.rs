use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::plugins::{PluginOutput, TagProposal};

pub const NHENTAI_METADATA_PLUGIN_ID: &str = "nhentai-metadata";
const NHENTAI_API_URL: &str = "https://nhentai.net/api/v2";
const NHENTAI_WEB_URL: &str = "https://nhentai.net";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NhentaiCandidate {
    pub gallery_id: String,
    pub source_url: String,
    pub title: String,
}

#[derive(Debug)]
pub struct NhentaiCandidateSearch {
    pub candidates: Vec<NhentaiCandidate>,
    pub exact_phrase: bool,
}

#[derive(Debug, Clone)]
pub struct NhentaiConfig {
    pub request_timeout_ms: u64,
    pub max_retries: u32,
    pub fetch_scanlator: bool,
}

impl NhentaiConfig {
    pub fn from_json(config: Option<&Value>) -> Self {
        let get_u64 = |key: &str, default: u64| {
            config
                .and_then(|value| value.get(key))
                .and_then(Value::as_u64)
                .unwrap_or(default)
        };
        Self {
            request_timeout_ms: get_u64("request_timeout_ms", 10_000).clamp(1_000, 60_000),
            max_retries: get_u64("max_retries", 2).min(5) as u32,
            fetch_scanlator: config
                .and_then(|value| value.get("fetch_scanlator"))
                .and_then(Value::as_bool)
                .unwrap_or(true),
        }
    }
}

#[derive(Debug, Deserialize)]
struct NhentaiGallery {
    id: u64,
    #[serde(default)]
    title: NhentaiTitle,
    #[serde(default)]
    tags: Vec<NhentaiTag>,
    #[serde(default)]
    scanlator: String,
}

#[derive(Debug, Default, Deserialize)]
struct NhentaiTitle {
    #[serde(default)]
    pretty: String,
    #[serde(default)]
    english: String,
    #[serde(default)]
    japanese: String,
}

#[derive(Debug, Deserialize)]
struct NhentaiTag {
    #[serde(rename = "type", default)]
    tag_type: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Default, Deserialize)]
struct NhentaiSearchResponse {
    #[serde(default)]
    result: Vec<NhentaiSearchGallery>,
}

#[derive(Debug, Deserialize)]
struct NhentaiSearchGallery {
    id: u64,
    #[serde(default)]
    english_title: String,
    #[serde(default)]
    japanese_title: String,
}

pub fn parse_gallery_reference(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let gallery_id = if trimmed.bytes().all(|byte| byte.is_ascii_digit()) {
        trimmed.to_string()
    } else {
        let normalized_url = if trimmed.starts_with("//") {
            format!("https:{trimmed}")
        } else if trimmed.contains("://") {
            trimmed.to_string()
        } else {
            format!("https://{trimmed}")
        };
        let url = reqwest::Url::parse(&normalized_url).ok()?;
        let host = url.host_str()?.to_ascii_lowercase();
        if host != "nhentai.net" && host != "www.nhentai.net" {
            return None;
        }
        url.path()
            .strip_prefix("/g/")?
            .trim_matches('/')
            .split('/')
            .next()?
            .trim()
            .to_string()
    };
    (!gallery_id.is_empty() && gallery_id.bytes().all(|byte| byte.is_ascii_digit()))
        .then_some(gallery_id)
}

pub fn source_url(gallery_id: &str) -> String {
    format!("{NHENTAI_WEB_URL}/g/{gallery_id}/")
}

pub async fn fetch_metadata(
    gallery_id: &str,
    config: &NhentaiConfig,
) -> Result<PluginOutput, String> {
    let gallery_id = parse_gallery_reference(gallery_id)
        .ok_or_else(|| "无效的 nHentai 画廊编号或链接".to_string())?;
    let client = build_client(config)?;
    let url = format!("{NHENTAI_API_URL}/galleries/{gallery_id}");
    let gallery: NhentaiGallery =
        request_json(&client, &url, config, "获取 nHentai 元数据").await?;
    if gallery.id.to_string() != gallery_id {
        return Err("nHentai 返回的画廊与请求不一致".to_string());
    }
    Ok(metadata_to_output(gallery, config))
}

pub async fn search_candidates(
    title: &str,
    config: &NhentaiConfig,
) -> Result<Vec<NhentaiCandidate>, String> {
    Ok(search_candidates_for_auto_match(title, config)
        .await?
        .candidates)
}

pub async fn search_candidates_for_auto_match(
    title: &str,
    config: &NhentaiConfig,
) -> Result<NhentaiCandidateSearch, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("漫画标题为空，无法搜索 nHentai".to_string());
    }
    let client = build_client(config)?;
    let quoted_title = format!("\"{title}\"");
    let mut candidates = search_candidates_once(&client, &quoted_title, config).await?;
    if !candidates.is_empty() {
        return Ok(NhentaiCandidateSearch {
            candidates,
            exact_phrase: true,
        });
    }
    candidates = search_candidates_once(&client, title, config).await?;
    Ok(NhentaiCandidateSearch {
        candidates,
        exact_phrase: false,
    })
}

async fn search_candidates_once(
    client: &Client,
    title: &str,
    config: &NhentaiConfig,
) -> Result<Vec<NhentaiCandidate>, String> {
    let mut url = reqwest::Url::parse(&format!("{NHENTAI_API_URL}/search"))
        .map_err(|err| format!("构建 nHentai 搜索请求失败: {err}"))?;
    url.query_pairs_mut().append_pair("query", title);
    let payload: NhentaiSearchResponse =
        request_json(&client, url.as_str(), config, "搜索 nHentai 候选").await?;
    Ok(payload
        .result
        .into_iter()
        .take(8)
        .map(|gallery| NhentaiCandidate {
            gallery_id: gallery.id.to_string(),
            source_url: source_url(&gallery.id.to_string()),
            title: preferred_search_title(title, &gallery)
                .unwrap_or_else(|| format!("nHentai gallery {}", gallery.id)),
        })
        .collect())
}

fn preferred_search_title(local_title: &str, gallery: &NhentaiSearchGallery) -> Option<String> {
    let local_uses_japanese = local_title
        .chars()
        .any(|character| matches!(character as u32, 0x3040..=0x30ff | 0x31f0..=0x31ff));
    let ordered = if local_uses_japanese {
        [&gallery.japanese_title, &gallery.english_title]
    } else {
        [&gallery.english_title, &gallery.japanese_title]
    };
    ordered
        .into_iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn build_client(config: &NhentaiConfig) -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_millis(config.request_timeout_ms))
        .user_agent("OtamoryX Metadata/1.0")
        .build()
        .map_err(|err| format!("创建 nHentai 请求失败: {err}"))
}

async fn request_json<T: serde::de::DeserializeOwned>(
    client: &Client,
    url: &str,
    config: &NhentaiConfig,
    operation: &str,
) -> Result<T, String> {
    let mut last_error = None;
    for attempt in 0..=config.max_retries {
        let mut retryable = true;
        match client.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                return response
                    .json()
                    .await
                    .map_err(|err| format!("解析 nHentai 返回数据失败: {err}"));
            }
            Ok(response) => {
                let status = response.status();
                retryable =
                    status.as_u16() == 408 || status.as_u16() == 429 || status.is_server_error();
                last_error = Some(format!("{operation}失败（HTTP {status}）"));
            }
            Err(error) => last_error = Some(format!("{operation}网络请求失败: {error}")),
        }
        if retryable && attempt < config.max_retries {
            tokio::time::sleep(Duration::from_millis(600 * (attempt as u64 + 1))).await;
        } else {
            break;
        }
    }
    Err(last_error.unwrap_or_else(|| format!("{operation}失败")))
}

fn metadata_to_output(gallery: NhentaiGallery, config: &NhentaiConfig) -> PluginOutput {
    let gallery_id = gallery.id.to_string();
    let mut tags = gallery
        .tags
        .into_iter()
        .filter_map(|tag| {
            let value = tag.name.trim();
            if value.is_empty() {
                return None;
            }
            let namespace =
                if tag.tag_type.eq_ignore_ascii_case("tag") || tag.tag_type.trim().is_empty() {
                    "general"
                } else {
                    tag.tag_type.trim()
                };
            Some(TagProposal::deterministic(
                namespace,
                value,
                NHENTAI_METADATA_PLUGIN_ID,
            ))
        })
        .collect::<Vec<_>>();
    if config.fetch_scanlator && !gallery.scanlator.trim().is_empty() {
        tags.push(TagProposal::deterministic(
            "scanlator",
            gallery.scanlator.trim(),
            NHENTAI_METADATA_PLUGIN_ID,
        ));
    }
    tags.push(TagProposal::deterministic(
        "source",
        source_url(&gallery_id),
        NHENTAI_METADATA_PLUGIN_ID,
    ));

    let title = gallery_title(&gallery.title);
    PluginOutput {
        tags,
        metadata: Default::default(),
        notes: title
            .map(|title| vec![format!("nHentai 标题：{title}")])
            .unwrap_or_default(),
    }
}

fn gallery_title(title: &NhentaiTitle) -> Option<String> {
    [&title.pretty, &title.english, &title.japanese]
        .into_iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        metadata_to_output, parse_gallery_reference, preferred_search_title, NhentaiConfig,
        NhentaiGallery, NhentaiSearchGallery, NHENTAI_API_URL,
    };

    #[test]
    fn uses_v2_api_endpoints() {
        assert_eq!(NHENTAI_API_URL, "https://nhentai.net/api/v2");
        assert_eq!(
            format!("{NHENTAI_API_URL}/galleries/52249"),
            "https://nhentai.net/api/v2/galleries/52249"
        );
        assert_eq!(
            format!("{NHENTAI_API_URL}/search"),
            "https://nhentai.net/api/v2/search"
        );
    }

    #[test]
    fn prefers_candidate_title_matching_local_script() {
        let gallery = NhentaiSearchGallery {
            id: 1,
            english_title: "English title".to_string(),
            japanese_title: "日本語タイトル".to_string(),
        };

        assert_eq!(
            preferred_search_title("日本語のタイトル", &gallery).as_deref(),
            Some("日本語タイトル")
        );
        assert_eq!(
            preferred_search_title("English", &gallery).as_deref(),
            Some("English title")
        );
    }

    #[test]
    fn parses_v2_search_response() {
        let payload: super::NhentaiSearchResponse = serde_json::from_value(json!({
            "result": [{
                "id": 52249,
                "english_title": "[Masamune Shirow] Pieces 1",
                "japanese_title": ""
            }],
            "num_pages": 1,
            "per_page": 25,
            "total": 1
        }))
        .expect("valid v2 search response");

        assert_eq!(payload.result.len(), 1);
        assert_eq!(payload.result[0].id, 52249);
        assert_eq!(
            payload.result[0].english_title,
            "[Masamune Shirow] Pieces 1"
        );
    }

    #[test]
    fn parses_gallery_urls_and_plain_ids() {
        assert_eq!(
            parse_gallery_reference("https://nhentai.net/g/123456/"),
            Some("123456".to_string())
        );
        assert_eq!(
            parse_gallery_reference(" 123456 "),
            Some("123456".to_string())
        );
        assert_eq!(
            parse_gallery_reference("nhentai.net/g/123456"),
            Some("123456".to_string())
        );
        assert_eq!(
            parse_gallery_reference("https://e-hentai.org/g/123456/token/"),
            None
        );
        assert_eq!(parse_gallery_reference("https://nhentai.net/g/nope/"), None);
        assert_eq!(parse_gallery_reference("not-a-gallery"), None);
    }

    #[test]
    fn maps_gallery_tags_and_scanlator() {
        let gallery: NhentaiGallery = serde_json::from_value(json!({
            "id": 123456,
            "title": { "pretty": "Example title" },
            "tags": [
                { "type": "artist", "name": "Example Artist" },
                { "type": "tag", "name": "full color" }
            ],
            "scanlator": "Example Group"
        }))
        .expect("valid gallery response");

        let output = metadata_to_output(gallery, &NhentaiConfig::from_json(None));
        let tags = output
            .tags
            .iter()
            .map(|tag| (tag.namespace.as_str(), tag.value.as_str()))
            .collect::<Vec<_>>();

        assert!(tags.contains(&("artist", "Example Artist")));
        assert!(tags.contains(&("general", "full color")));
        assert!(tags.contains(&("scanlator", "Example Group")));
        assert!(tags.contains(&("source", "https://nhentai.net/g/123456/")));
        assert_eq!(output.notes, vec!["nHentai 标题：Example title"]);
    }

    #[test]
    fn can_disable_scanlator_mapping() {
        let gallery: NhentaiGallery = serde_json::from_value(json!({
            "id": 123456,
            "scanlator": "Example Group"
        }))
        .expect("valid gallery response");
        let config = NhentaiConfig::from_json(Some(&json!({ "fetch_scanlator": false })));

        let output = metadata_to_output(gallery, &config);

        assert!(output.tags.iter().all(|tag| tag.namespace != "scanlator"));
    }
}
