use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::plugins::{PluginOutput, TagProposal};

pub const EHENTAI_METADATA_PLUGIN_ID: &str = "ehentai-metadata";
const EHENTAI_API_URL: &str = "https://api.e-hentai.org/api.php";
const EHENTAI_WEB_URL: &str = "https://e-hentai.org";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EhentaiCandidate {
    pub gallery_id: String,
    pub token: String,
    pub source_url: String,
    pub title: String,
}

#[derive(Debug)]
pub struct EhentaiCandidateSearch {
    pub candidates: Vec<EhentaiCandidate>,
    pub exact_phrase: bool,
}

#[derive(Debug, Clone)]
pub struct EhentaiConfig {
    pub request_timeout_ms: u64,
    pub max_retries: u32,
    pub prefer_japanese_title: bool,
}

impl EhentaiConfig {
    pub fn from_json(config: Option<&Value>) -> Self {
        let get_u64 = |key: &str, default: u64| {
            config
                .and_then(|value| value.get(key))
                .and_then(Value::as_u64)
                .unwrap_or(default)
        };
        Self {
            request_timeout_ms: get_u64("request_timeout_ms", 12_000).clamp(1_000, 60_000),
            max_retries: get_u64("max_retries", 2).min(5) as u32,
            prefer_japanese_title: config
                .and_then(|value| value.get("prefer_japanese_title"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GDataResponse {
    #[serde(default)]
    gmetadata: Vec<GDataGallery>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GDataGallery {
    gid: String,
    token: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    title_jpn: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tags: Vec<String>,
}

pub fn parse_gallery_reference(value: &str) -> Option<(String, String)> {
    let trimmed = value.trim();
    let remainder = if trimmed.contains("://") || trimmed.starts_with("//") {
        let normalized_url = if trimmed.starts_with("//") {
            format!("https:{trimmed}")
        } else {
            trimmed.to_string()
        };
        let url = reqwest::Url::parse(&normalized_url).ok()?;
        let host = url.host_str()?.to_ascii_lowercase();
        if host != "e-hentai.org" && host != "www.e-hentai.org" {
            return None;
        }
        url.path().strip_prefix("/g/")?.to_string()
    } else {
        trimmed.to_string()
    };
    let mut parts = remainder.trim_matches('/').split('/');
    let gallery_id = parts.next()?.trim();
    let token = parts.next()?.trim();
    if gallery_id.is_empty()
        || token.is_empty()
        || !gallery_id.bytes().all(|byte| byte.is_ascii_digit())
        || !token.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return None;
    }
    Some((gallery_id.to_string(), token.to_string()))
}

pub fn source_url(gallery_id: &str, token: &str) -> String {
    format!("{EHENTAI_WEB_URL}/g/{gallery_id}/{token}/")
}

pub async fn fetch_metadata(
    gallery_id: &str,
    token: &str,
    config: &EhentaiConfig,
) -> Result<PluginOutput, String> {
    let client = Client::builder()
        .timeout(Duration::from_millis(config.request_timeout_ms))
        .user_agent("OtamoryX Metadata/1.0")
        .build()
        .map_err(|err| format!("创建 E-Hentai 请求失败: {err}"))?;
    let request = json!({
        "method": "gdata",
        "gidlist": [[gallery_id, token]],
        "namespace": 1,
    });

    let mut last_error = None;
    for attempt in 0..=config.max_retries {
        match client.post(EHENTAI_API_URL).json(&request).send().await {
            Ok(response) if response.status().is_success() => {
                let payload: GDataResponse = response
                    .json()
                    .await
                    .map_err(|err| format!("解析 E-Hentai 返回数据失败: {err}"))?;
                if let Some(error) = payload.error {
                    return Err(format!("E-Hentai 返回错误: {error}"));
                }
                let gallery = payload
                    .gmetadata
                    .into_iter()
                    .next()
                    .ok_or_else(|| "未找到对应的 E-Hentai 画廊，链接可能已失效".to_string())?;
                if gallery.gid != gallery_id || !gallery.token.eq_ignore_ascii_case(token) {
                    return Err("E-Hentai 返回的画廊与请求不一致".to_string());
                }
                return Ok(metadata_to_output(gallery, config));
            }
            Ok(response) => {
                last_error = Some(format!("E-Hentai 请求失败（HTTP {}）", response.status()));
            }
            Err(error) => last_error = Some(format!("E-Hentai 网络请求失败: {error}")),
        }
        if attempt < config.max_retries {
            tokio::time::sleep(Duration::from_millis(600 * (attempt as u64 + 1))).await;
        }
    }
    Err(last_error.unwrap_or_else(|| "E-Hentai 请求失败".to_string()))
}

pub async fn search_candidates(
    title: &str,
    config: &EhentaiConfig,
) -> Result<Vec<EhentaiCandidate>, String> {
    Ok(search_candidates_for_auto_match(title, config)
        .await?
        .candidates)
}

pub async fn search_candidates_for_auto_match(
    title: &str,
    config: &EhentaiConfig,
) -> Result<EhentaiCandidateSearch, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("漫画标题为空，无法搜索 E-Hentai".to_string());
    }
    let client = Client::builder()
        .timeout(Duration::from_millis(config.request_timeout_ms))
        .user_agent("OtamoryX Metadata/1.0")
        .build()
        .map_err(|err| format!("创建 E-Hentai 请求失败: {err}"))?;
    let quoted_title = format!("\"{title}\"");
    let mut candidates = search_candidates_once(&client, &quoted_title, config).await?;
    if !candidates.is_empty() {
        return Ok(EhentaiCandidateSearch {
            candidates,
            exact_phrase: true,
        });
    }
    candidates = search_candidates_once(&client, title, config).await?;
    Ok(EhentaiCandidateSearch {
        candidates,
        exact_phrase: false,
    })
}

async fn search_candidates_once(
    client: &Client,
    query_text: &str,
    config: &EhentaiConfig,
) -> Result<Vec<EhentaiCandidate>, String> {
    let query = urlencoding::encode(query_text);
    let search_url =
        format!("{EHENTAI_WEB_URL}/?advsearch=1&f_sfu=on&f_sft=on&f_sfl=on&f_search={query}");
    let mut last_error = None;
    for attempt in 0..=config.max_retries {
        let mut retryable = true;
        match client.get(&search_url).send().await {
            Ok(response) if response.status().is_success() => {
                let html = response
                    .text()
                    .await
                    .map_err(|err| format!("读取 E-Hentai 搜索结果失败: {err}"))?;
                if html.contains("Your IP address has been")
                    || html.contains("You are opening pages too fast")
                {
                    last_error = Some("E-Hentai 暂时限制了当前 IP，请稍后再试".to_string());
                } else {
                    return Ok(parse_search_candidates(&html));
                }
            }
            Ok(response) => {
                let status = response.status();
                retryable =
                    status.as_u16() == 408 || status.as_u16() == 429 || status.is_server_error();
                last_error = Some(format!("E-Hentai 搜索失败（HTTP {status}）"));
            }
            Err(error) => last_error = Some(format!("E-Hentai 搜索请求失败: {error}")),
        }
        if retryable && attempt < config.max_retries {
            tokio::time::sleep(Duration::from_millis(600 * (attempt as u64 + 1))).await;
        } else {
            break;
        }
    }
    Err(last_error.unwrap_or_else(|| "E-Hentai 搜索失败".to_string()))
}

fn metadata_to_output(gallery: GDataGallery, config: &EhentaiConfig) -> PluginOutput {
    let remote_title = if config.prefer_japanese_title && !gallery.title_jpn.trim().is_empty() {
        gallery.title_jpn
    } else {
        gallery.title
    };
    let mut tags = gallery
        .tags
        .into_iter()
        .filter_map(|raw| {
            raw.split_once(':')
                .map(|(namespace, value)| (namespace.trim().to_string(), value.trim().to_string()))
        })
        .filter(|(namespace, value)| !namespace.is_empty() && !value.is_empty())
        .map(|(namespace, value)| {
            TagProposal::deterministic(namespace, value, EHENTAI_METADATA_PLUGIN_ID)
        })
        .collect::<Vec<_>>();
    if !gallery.category.trim().is_empty() {
        tags.push(TagProposal::deterministic(
            "category",
            gallery.category.to_ascii_lowercase(),
            EHENTAI_METADATA_PLUGIN_ID,
        ));
    }
    tags.push(TagProposal::deterministic(
        "source",
        source_url(&gallery.gid, &gallery.token),
        EHENTAI_METADATA_PLUGIN_ID,
    ));
    PluginOutput {
        tags,
        metadata: Default::default(),
        notes: vec![format!(
            "E-Hentai 标题：{}",
            decode_html_entities(&remote_title)
        )],
    }
}

pub fn parse_search_candidates(html: &str) -> Vec<EhentaiCandidate> {
    let mut candidates = Vec::new();
    let mut cursor = html;
    while let Some(link_start) = cursor.find("/g/") {
        let after_link = &cursor[link_start + 3..];
        let Some(link_end) = after_link.find('"') else {
            break;
        };
        let Some((gallery_id, token)) = parse_gallery_reference(&after_link[..link_end]) else {
            cursor = after_link;
            continue;
        };
        let title = after_link[link_end..]
            .find("class=\"glink\"")
            .and_then(|index| {
                let after_class = &after_link[link_end + index..];
                after_class.find('>').and_then(|open| {
                    let content = &after_class[open + 1..];
                    content
                        .find("</")
                        .map(|close| decode_html_entities(&strip_html(&content[..close])))
                })
            })
            .unwrap_or_else(|| format!("E-Hentai gallery {gallery_id}"));
        if !candidates.iter().any(|candidate: &EhentaiCandidate| {
            candidate.gallery_id == gallery_id && candidate.token == token
        }) {
            candidates.push(EhentaiCandidate {
                source_url: source_url(&gallery_id, &token),
                gallery_id,
                token,
                title,
            });
        }
        if candidates.len() == 8 {
            break;
        }
        cursor = after_link;
    }
    candidates
}

fn strip_html(value: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(character),
            _ => {}
        }
    }
    out
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse_gallery_reference, parse_search_candidates};

    #[test]
    fn parses_gallery_urls_and_gid_token_pairs() {
        assert_eq!(
            parse_gallery_reference("https://e-hentai.org/g/123456/abcDeF12/"),
            Some(("123456".to_string(), "abcDeF12".to_string()))
        );
        assert_eq!(
            parse_gallery_reference("https://exhentai.org/g/123456/abcDeF12/"),
            None
        );
        assert_eq!(parse_gallery_reference("not-a-gallery"), None);
    }

    #[test]
    fn keeps_multiple_search_candidates() {
        let html = r#"<a href="https://e-hentai.org/g/1/tokenone/"><div class="glink">First &amp; Story</div></a><a href="https://e-hentai.org/g/2/tokentwo/"><div class="glink">Second Story</div></a>"#;
        let candidates = parse_search_candidates(html);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].title, "First & Story");
    }
}
