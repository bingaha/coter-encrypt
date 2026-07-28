//! 共享云盛管理端鉴权（token_inner）。
//!
//! v1：手动粘贴 Token；预留 AutoLogin 扩展点，调用方只依赖 get_token / 请求头辅助。

use std::{fs, path::PathBuf, time::Duration};

use directories::ProjectDirs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::http_client::{self, HttpProxyConfig};

const CONFIG_FILE_NAME: &str = "yunsheng-auth.json";
const WORK_ORIGIN: &str = "https://work.yunsheng.cn";

pub const MISSING_TOKEN_ERROR: &str = "请先配置云盛 Token（token_inner）";
pub const EXPIRED_TOKEN_ERROR: &str = "云盛登录已失效，请重新配置 Token（token_inner）";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct YunshengAuthConfig {
    #[serde(default)]
    pub token_inner: String,
}

/// 鉴权提供方扩展点：v1 仅 ManualToken；AutoLogin 预留不实现。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AuthProviderKind {
    #[default]
    ManualToken,
    /// 预留：自动登录（v1 不实现）
    AutoLogin,
}

pub trait AuthProvider: Send + Sync {
    fn get_token(&self) -> Result<String, String>;
}

/// v1：从独立配置文件读取手动粘贴的 token_inner。
#[allow(dead_code)] // 供后续 manage-api 模块经 get_token / AuthProvider 复用
pub struct ManualTokenProvider;

impl AuthProvider for ManualTokenProvider {
    fn get_token(&self) -> Result<String, String> {
        let config = load_yunsheng_auth_config()?;
        require_token(&config.token_inner)
    }
}

/// 预留自动登录提供方；调用即明确报错，避免误用。
pub struct AutoLoginProvider;

impl AuthProvider for AutoLoginProvider {
    fn get_token(&self) -> Result<String, String> {
        Err("自动登录尚未实现，请使用手动 Token".to_string())
    }
}

pub fn active_provider() -> AuthProviderKind {
    AuthProviderKind::ManualToken
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
    let content = fs::read_to_string(&path).map_err(|e| format!("读取云盛鉴权配置失败: {e}"))?;
    let config: YunshengAuthConfig =
        serde_json::from_str(&content).map_err(|e| format!("解析云盛鉴权配置失败: {e}"))?;
    Ok(config)
}

pub fn save_yunsheng_auth_config_to_disk(config: &YunshengAuthConfig) -> Result<(), String> {
    let path = config_path()?;
    ensure_config_dir(&path)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化云盛鉴权配置失败: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("写入云盛鉴权配置失败: {e}"))?;
    Ok(())
}

pub fn save_yunsheng_auth_config(config: YunshengAuthConfig) -> Result<YunshengAuthConfig, String> {
    let mut config = config;
    config.token_inner = config.token_inner.trim().to_string();
    save_yunsheng_auth_config_to_disk(&config)?;
    Ok(config)
}

/// 纯逻辑：校验 token 非空；供测试与 get_token 复用。
pub fn require_token(token_inner: &str) -> Result<String, String> {
    let token = token_inner.trim();
    if token.is_empty() {
        return Err(MISSING_TOKEN_ERROR.to_string());
    }
    Ok(token.to_string())
}

/// 统一取 token；缺 token 返回明确中文错误。
#[allow(dead_code)] // 供后续 manage-api 模块复用
pub fn get_token() -> Result<String, String> {
    match active_provider() {
        AuthProviderKind::ManualToken => ManualTokenProvider.get_token(),
        AuthProviderKind::AutoLogin => AutoLoginProvider.get_token(),
    }
}

/// Cookie 值：`token_inner=...`
pub fn build_auth_cookie(token: &str) -> String {
    format!("token_inner={token}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthRequestHeaders {
    pub cookie: String,
    pub accept: String,
    pub content_type: String,
    pub origin: String,
    pub referer: String,
}

/// 构造管理端通用请求头（Cookie + accept/content-type/origin/referer）。
pub fn build_auth_request_headers(token: &str) -> AuthRequestHeaders {
    AuthRequestHeaders {
        cookie: build_auth_cookie(token),
        accept: "application/json, text/plain, */*".to_string(),
        content_type: "application/json".to_string(),
        origin: WORK_ORIGIN.to_string(),
        referer: WORK_ORIGIN.to_string(),
    }
}

/// 将鉴权头附加到 reqwest RequestBuilder（供后续 manage-api 模块复用）。
#[allow(dead_code)]
pub fn apply_auth_headers(
    builder: reqwest::RequestBuilder,
    token: &str,
) -> reqwest::RequestBuilder {
    let headers = build_auth_request_headers(token);
    builder
        .header(reqwest::header::COOKIE, headers.cookie)
        .header(reqwest::header::ACCEPT, headers.accept)
        .header(reqwest::header::CONTENT_TYPE, headers.content_type)
        .header(reqwest::header::ORIGIN, headers.origin)
        .header(reqwest::header::REFERER, headers.referer)
}

/// 出站客户端：走全局代理配置。
pub fn build_yunsheng_http_client(proxy: &HttpProxyConfig) -> Result<Client, String> {
    http_client::build_http_client(Duration::from_secs(30), proxy)
}

/// 将 HTTP 401 / 未授权或 API 表示登录失效映射为可展示的中文错误。
/// 若非鉴权类错误则返回 None。
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
    fn require_token_rejects_empty() {
        let err = require_token("").expect_err("empty should fail");
        assert_eq!(err, MISSING_TOKEN_ERROR);

        let err = require_token("   ").expect_err("whitespace should fail");
        assert_eq!(err, MISSING_TOKEN_ERROR);
    }

    #[test]
    fn require_token_accepts_non_empty() {
        assert_eq!(require_token(" abc ").unwrap(), "abc");
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
    fn build_auth_request_headers_shape() {
        let headers = build_auth_request_headers("tok-123");
        assert_eq!(headers.cookie, "token_inner=tok-123");
        assert_eq!(headers.origin, "https://work.yunsheng.cn");
        assert_eq!(headers.referer, "https://work.yunsheng.cn");
        assert!(headers.accept.contains("application/json"));
        assert_eq!(headers.content_type, "application/json");
    }

    #[test]
    fn manual_provider_is_default_active() {
        assert_eq!(active_provider(), AuthProviderKind::ManualToken);
    }

    #[test]
    fn auto_login_provider_is_not_implemented() {
        let err = AutoLoginProvider
            .get_token()
            .expect_err("auto login reserved");
        assert!(err.contains("尚未实现"));
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
