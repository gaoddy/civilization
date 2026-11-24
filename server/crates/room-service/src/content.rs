//! 客户端展示内容下发：构建期内嵌 `client-content.json`，按内容版本支持条件请求。
//!
//! 内容由 `scripts/gen_client_data.py` 生成，与 `server/data/v0.1/` 下各表同源；
//! 客户端只用于展示，规则判定仍走 `rule-engine`。

use std::sync::OnceLock;

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::Response;

/// 客户端展示内容 bundle：`{version, cards, heroes, skills, taxonomy}`。
pub const CONTENT_JSON: &str = include_str!("../../../data/v0.1/client-content.json");

static VERSION: OnceLock<String> = OnceLock::new();

/// 内容版本：bundle 内 `version` 字段，生成时由四段内容哈希得到。
///
/// 解析失败或字段缺失时返回空串，客户端每次都会重新下载；生成物正确性由单测保证。
pub fn version() -> &'static str {
    VERSION.get_or_init(|| {
        serde_json::from_str::<serde_json::Value>(CONTENT_JSON)
            .ok()
            .and_then(|value| {
                value
                    .get("version")
                    .and_then(|item| item.as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_default()
    })
}

fn etag() -> String {
    format!("\"{}\"", version())
}

/// `GET /content`：返回完整展示内容；`If-None-Match` 与当前版本一致时返回 304。
pub async fn content(headers: HeaderMap) -> Response {
    let etag = etag();
    let matches = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        == Some(etag.as_str());

    let mut response = if matches {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::NOT_MODIFIED;
        response
    } else {
        let mut response = Response::new(Body::from(CONTENT_JSON));
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        response
    };
    if let Ok(value) = HeaderValue::from_str(&etag) {
        response.headers_mut().insert(header::ETAG, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::{CONTENT_JSON, version};

    #[test]
    fn bundle_has_version_and_all_sections() {
        let parsed: serde_json::Value =
            serde_json::from_str(CONTENT_JSON).expect("client-content.json 应可解析");
        assert!(!version().is_empty(), "内容版本不得为空");
        assert_eq!(parsed["version"].as_str(), Some(version()));
        for section in ["cards", "heroes", "skills"] {
            let items = parsed[section].as_array().expect("数据段应为数组");
            assert!(!items.is_empty(), "{section} 不得为空");
        }
        assert!(
            parsed["taxonomy"]["nodes"]
                .as_array()
                .is_some_and(|nodes| !nodes.is_empty()),
            "taxonomy.nodes 不得为空"
        );
    }
}
