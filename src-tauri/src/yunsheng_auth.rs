//! 共享云生管理端鉴权：用户粘贴完整 Cookie（须含 token_inner=...）。
//!
//! 手动粘贴 Cookie；预留 AutoLogin 扩展点，调用方只依赖 get_cookies / 请求头辅助。

use std::{fs, path::PathBuf, time::Duration};

use directories::ProjectDirs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::http_client::{self, HttpProxyConfig};

const CONFIG_FILE_NAME: &str = "yunsheng-auth.json";
const WORK_ORIGIN: &str = "https://work.yunsheng.cn";

pub const MISSING_COOKIES_ERROR: &str = "请先配置云生 Cookie（须包含 token_inner=...）";
pub const INVALID_COOKIES_ERROR: &str =
    "云生 Cookie 格式无效，请粘贴完整内容，例如：token_inner=...";
pub const EXPIRED_TOKEN_ERROR: &str = "云生登录已失效，请重新配置 Cookie";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct YunshengAuthConfig {
    /// 完整 Cookie 请求头值，例如 `token_inner=eyJ...` 或 `a=1; token_inner=eyJ...`
    #[serde(default)]
    pub cookies: String,
}

/// 鉴权提供方扩展点：当前仅 ManualCookies；AutoLogin 预留不实现。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AuthProviderKind {
    #[default]
    ManualCookies,
    AutoLogin,
}

pub trait AuthProvider: Send + Sync {
    fn get_cookies(&self) -> Result<String, String>;
}

#[allow(dead_code)]
pub struct ManualCookiesProvider;

impl AuthProvider for ManualCookiesProvider {
    fn get_cookies(&self) -> Result<String, String> {
        let config = load_yunsheng_auth_config()?;
        require_cookies(&config.cookies)
    }
}

pub struct AutoLoginProvider;

impl AuthProvider for AutoLoginProvider {
    fn get_cookies(&self) -> Result<String, String> {
        Err(MISSING_COOKIES_ERROR.to_string())
    }
}

pub fn active_provider() -> AuthProviderKind {
    AuthProviderKind::ManualCookies
}

fn config_path() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from("com", "coter", "CoterEncrypt")
        .ok_or_else(|| "无法解析应用配置目录".to_string())?;
    Ok(dirs.config_dir().join(CONFIG_FILE_NAME))
}

fn ensure_config_dir(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    Ok(())
}

pub fn load_yunsheng_auth_config() -> Result<YunshengAuthConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        let config = YunshengAuthConfig::default();
        save_yunsheng_auth_config_to_disk(&config)?;
        return Ok(config);
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("读取云生鉴权配置失败: {e}"))?;
    let config: YunshengAuthConfig =
        serde_json::from_str(&content).map_err(|e| format!("解析云生鉴权配置失败: {e}"))?;
    Ok(config)
}

pub fn save_yunsheng_auth_config_to_disk(config: &YunshengAuthConfig) -> Result<(), String> {
    let path = config_path()?;
    ensure_config_dir(&path)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化云生鉴权配置失败: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("写入云生鉴权配置失败: {e}"))?;
    Ok(())
}

pub fn save_yunsheng_auth_config(config: YunshengAuthConfig) -> Result<YunshengAuthConfig, String> {
    let mut config = config;
    config.cookies = normalize_cookies_input(&config.cookies)?;
    save_yunsheng_auth_config_to_disk(&config)?;
    Ok(config)
}

/// 规范化并校验用户粘贴的 Cookie：非空，且包含 `token_inner=` 名值对。
pub fn normalize_cookies_input(raw: &str) -> Result<String, String> {
    let cookies = raw.trim().trim_end_matches(';').trim().to_string();
    require_cookies(&cookies)
}

/// 纯逻辑：Cookie 非空且含 token_inner=。
pub fn require_cookies(cookies: &str) -> Result<String, String> {
    let cookies = cookies.trim();
    if cookies.is_empty() {
        return Err(MISSING_COOKIES_ERROR.to_string());
    }
    if !cookie_header_has_token_inner(cookies) {
        return Err(INVALID_COOKIES_ERROR.to_string());
    }
    Ok(cookies.to_string())
}

fn cookie_header_has_token_inner(cookies: &str) -> bool {
    cookies.split(';').any(|part| {
        let part = part.trim();
        part.strip_prefix("token_inner=")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    })
}

/// 统一取 Cookie 头；缺失或格式无效返回明确中文错误。
#[allow(dead_code)]
pub fn get_cookies() -> Result<String, String> {
    match active_provider() {
        AuthProviderKind::ManualCookies => ManualCookiesProvider.get_cookies(),
        AuthProviderKind::AutoLogin => AutoLoginProvider.get_cookies(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthRequestHeaders {
    pub cookie: String,
    pub accept: String,
    pub content_type: String,
    pub origin: String,
    pub referer: String,
}

/// 构造管理端通用请求头（Cookie 原样放入 + accept/content-type/origin/referer）。
pub fn build_auth_request_headers(cookies: &str) -> AuthRequestHeaders {
    AuthRequestHeaders {
        cookie: cookies.trim().to_string(),
        accept: "application/json, text/plain, */*".to_string(),
        content_type: "application/json".to_string(),
        origin: WORK_ORIGIN.to_string(),
        referer: WORK_ORIGIN.to_string(),
    }
}

#[allow(dead_code)]
pub fn apply_auth_headers(
    builder: reqwest::RequestBuilder,
    cookies: &str,
) -> reqwest::RequestBuilder {
    let headers = build_auth_request_headers(cookies);
    builder
        .header(reqwest::header::COOKIE, headers.cookie)
        .header(reqwest::header::ACCEPT, headers.accept)
        .header(reqwest::header::CONTENT_TYPE, headers.content_type)
        .header(reqwest::header::ORIGIN, headers.origin)
        .header(reqwest::header::REFERER, headers.referer)
}

pub fn build_yunsheng_http_client(proxy: &HttpProxyConfig) -> Result<Client, String> {
    http_client::build_http_client(Duration::from_secs(30), proxy)
}

pub fn map_auth_error(http_status: u16, body: Option<&Value>) -> Option<String> {
    if http_status == 401 {
        return Some(EXPIRED_TOKEN_ERROR.to_string());
    }

    if let Some(value) = body {
        if looks_like_auth_failure(value) {
            return Some(EXPIRED_TOKEN_ERROR.to_string());
        }
    }

    None
}

fn looks_like_auth_failure(value: &Value) -> bool {
    let status = value
        .get("status")
        .and_then(|v| v.as_u64())
        .or_else(|| value.get("code").and_then(|v| v.as_u64()));
    if status == Some(401) {
        return true;
    }

    let mut texts: Vec<String> = Vec::new();
    for key in ["msg", "message", "error", "errorMessage", "errorMsg"] {
        if let Some(s) = value.get(key).and_then(|v| v.as_str()) {
            texts.push(s.to_string());
        }
    }
    if let Some(s) = value.as_str() {
        texts.push(s.to_string());
    }

    texts.iter().any(|t| message_indicates_auth_failure(t))
}

fn message_indicates_auth_failure(message: &str) -> bool {
    let lower = message.to_lowercase();
    const KEYWORDS: &[&str] = &[
        "未登录",
        "未授权",
        "登录失效",
        "登录过期",
        "登录已失效",
        "登录已过期",
        "token无效",
        "token 无效",
        "token失效",
        "token过期",
        "token已失效",
        "token已过期",
        "unauthorized",
        "unauthenticated",
        "not login",
        "not logged in",
        "login expired",
        "token expired",
        "invalid token",
    ];
    KEYWORDS.iter().any(|kw| {
        if kw.chars().all(|c| c.is_ascii()) {
            lower.contains(kw)
        } else {
            message.contains(kw)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn require_cookies_rejects_empty() {
        let err = require_cookies("").expect_err("empty should fail");
        assert_eq!(err, MISSING_COOKIES_ERROR);

        let err = require_cookies("   ").expect_err("whitespace should fail");
        assert_eq!(err, MISSING_COOKIES_ERROR);
    }

    #[test]
    fn require_cookies_rejects_bare_token_without_name() {
        let err = require_cookies("eyJhbGciOiJSUzI1NiJ9.abc").expect_err("bare jwt");
        assert_eq!(err, INVALID_COOKIES_ERROR);
    }

    #[test]
    fn require_cookies_accepts_token_inner_pair() {
        let cookies = require_cookies("token_inner=eyJhbGciOiJSUzI1NiJ9.abc").unwrap();
        assert_eq!(cookies, "token_inner=eyJhbGciOiJSUzI1NiJ9.abc");
    }

    #[test]
    fn require_cookies_accepts_multi_cookie_header() {
        let cookies =
            require_cookies("foo=1; token_inner=eyJhbGciOiJSUzI1NiJ9.abc; bar=2").unwrap();
        assert!(cookies.contains("token_inner=eyJhbGciOiJSUzI1NiJ9.abc"));
    }

    #[test]
    fn normalize_trims_and_strips_trailing_semicolon() {
        let cookies = normalize_cookies_input("  token_inner=abc;  ").unwrap();
        assert_eq!(cookies, "token_inner=abc");
    }

    #[test]
    fn map_auth_error_on_http_401() {
        let mapped = map_auth_error(401, None).expect("401 should map");
        assert_eq!(mapped, EXPIRED_TOKEN_ERROR);
    }

    #[test]
    fn map_auth_error_on_api_unauthorized_message() {
        let body = json!({"code": 401, "msg": "未登录或登录已失效"});
        let mapped = map_auth_error(200, Some(&body)).expect("api auth fail should map");
        assert_eq!(mapped, EXPIRED_TOKEN_ERROR);
    }

    #[test]
    fn map_auth_error_on_unauthorized_keyword() {
        let body = json!({"message": "Unauthorized"});
        let mapped = map_auth_error(200, Some(&body)).expect("unauthorized should map");
        assert_eq!(mapped, EXPIRED_TOKEN_ERROR);
    }

    #[test]
    fn map_auth_error_ignores_ordinary_errors() {
        let body = json!({"code": 500, "msg": "服务器繁忙"});
        assert!(map_auth_error(500, Some(&body)).is_none());
        assert!(map_auth_error(200, Some(&body)).is_none());
    }

    #[test]
    fn build_auth_request_headers_uses_cookies_as_is() {
        let headers = build_auth_request_headers("token_inner=tok-123");
        assert_eq!(headers.cookie, "token_inner=tok-123");
        assert_eq!(headers.origin, "https://work.yunsheng.cn");
        assert_eq!(headers.referer, "https://work.yunsheng.cn");
        assert!(headers.accept.contains("application/json"));
        assert_eq!(headers.content_type, "application/json");
    }

    #[test]
    fn manual_provider_is_default_active() {
        assert_eq!(active_provider(), AuthProviderKind::ManualCookies);
    }

    #[test]
    fn auto_login_provider_asks_for_cookies() {
        let err = AutoLoginProvider
            .get_cookies()
            .expect_err("auto login reserved");
        assert_eq!(err, MISSING_COOKIES_ERROR);
    }

    #[test]
    fn build_client_respects_proxy_config() {
        let proxy = HttpProxyConfig {
            mode: http_client::HttpProxyMode::Direct,
            url: String::new(),
        };
        assert!(build_yunsheng_http_client(&proxy).is_ok());
    }
}
