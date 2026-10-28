use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE};
use serde::Serialize;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

static CLIENT: OnceLock<RwLock<reqwest::Client>> = OnceLock::new();
static PROXY: OnceLock<RwLock<String>> = OnceLock::new();

fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        ),
    );
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"));
    headers
}

fn build(proxy: &str) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .user_agent(UA)
        .default_headers(default_headers())
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(6))
        // 保留 cookie：必应翻译、Cloudflare 前置的站点都要求先带上会话
        .cookie_store(true)
        .gzip(true)
        .deflate(true)
        .brotli(true);

    let proxy = proxy.trim();
    if !proxy.is_empty() {
        match reqwest::Proxy::all(proxy) {
            Ok(cfg) => builder = builder.proxy(cfg),
            Err(err) => log::warn!("代理地址无效（{proxy}）：{err}"),
        }
    }

    builder
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn client() -> reqwest::Client {
    CLIENT
        .get_or_init(|| RwLock::new(build("")))
        .read()
        .map(|c| c.clone())
        .unwrap_or_else(|_| build(""))
}

/// 配置里改了代理就重建客户端；下次请求立即生效
pub fn set_proxy(proxy: &str) {
    let slot = PROXY.get_or_init(|| RwLock::new(String::new()));
    let changed = slot
        .read()
        .map(|current| current.as_str() != proxy.trim())
        .unwrap_or(true);
    if !changed {
        return;
    }
    if let Ok(mut guard) = slot.write() {
        *guard = proxy.trim().to_string();
    }
    if let Some(lock) = CLIENT.get() {
        if let Ok(mut guard) = lock.write() {
            *guard = build(proxy);
        }
    }
    log::info!("网络代理已更新：{}", if proxy.trim().is_empty() { "直连" } else { proxy.trim() });
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FetchResult {
    pub status: u16,
    pub body: String,
    pub url: String,
}

/// 在 Rust 侧发起请求：不受 WebView 的 CORS / Origin 限制，
/// 也能按需伪造 Referer，这是浏览器扩展之外才具备的自由度。
pub async fn fetch_text(
    url: &str,
    referer: Option<String>,
    extra: Option<HashMap<String, String>>,
    method: Option<String>,
    body: Option<String>,
) -> Result<FetchResult, String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("仅支持 http/https 请求".into());
    }
    let post = method
        .as_deref()
        .map(|m| m.eq_ignore_ascii_case("POST"))
        .unwrap_or(false);
    let mut req = if post {
        client().post(url)
    } else {
        client().get(url)
    };
    if let Some(body) = body {
        req = req.body(body);
    }
    if let Some(referer) = referer {
        if !referer.is_empty() {
            req = req.header(reqwest::header::REFERER, referer);
        }
    }
    if let Some(extra) = extra {
        for (k, v) in extra {
            if k.eq_ignore_ascii_case("host") || k.eq_ignore_ascii_case("content-length") {
                continue;
            }
            req = req.header(k, v);
        }
    }
    let res = req.send().await.map_err(|err| {
        if err.is_timeout() {
            "请求超时".to_string()
        } else if err.is_connect() {
            "无法连接到该站点（网络或代理受限）".to_string()
        } else {
            format!("请求失败：{err}")
        }
    })?;
    let status = res.status().as_u16();
    let final_url = res.url().to_string();
    let body = res.text().await.map_err(|e| e.to_string())?;
    Ok(FetchResult {
        status,
        body,
        url: final_url,
    })
}
