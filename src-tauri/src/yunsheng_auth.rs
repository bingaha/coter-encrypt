//! 共享云生管理端鉴权：手动 Cookie + SM2 全自动登录。
//!
//! 业务请求先用已有 Cookie；鉴权失败且账号密码可用时登录一次并重试原请求一次。

use std::{fs, path::PathBuf, time::Duration};

use directories::ProjectDirs;
use rand::RngCore;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;
use url::Url;

use crate::browser_bridge::{self, OpenUrlWithCookieHeaderRequest};
use crate::http_client::{self, HttpProxyConfig};

const CONFIG_FILE_NAME: &str = "yunsheng-auth.json";
const WORK_ORIGIN: &str = "https://work.yunsheng.cn";
const SSO_ORIGIN: &str = "https://sso.yunsheng.cn";
const PREFLIGHT_URL: &str = "https://gateway.yunsheng.cn/uum/login/preflight";
const LOGIN_URL: &str = "https://gateway.yunsheng.cn/uum/inner/login.json";
const AUTO_LOGIN_URL: &str = "https://gateway.shebaotong.com/uum/inner/autoLogin.json";
pub const SHEBAOROBOT_URL: &str = "https://work.yunsheng.cn/shebaorobot/";

pub const MISSING_COOKIES_ERROR: &str = "请先配置云生 Cookie（须包含 token_inner=...）";
pub const INVALID_COOKIES_ERROR: &str =
    "云生 Cookie 格式无效，请粘贴完整内容，例如：token_inner=...";
pub const EXPIRED_TOKEN_ERROR: &str = "云生登录已失效，请重新配置 Cookie";
pub const MISSING_CREDENTIALS_ERROR: &str = "请先配置云生账号和密码";
pub const AUTH_NEED_CREDENTIALS_ERROR: &str =
    "云生登录已失效，请重新配置 Cookie，或填写账号密码后自动登录";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CookieFileEntry {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct YunshengAuthConfig {
    /// 完整 Cookie 请求头值，例如 `token_inner=eyJ...` 或 `a=1; token_inner=eyJ...`
    #[serde(default)]
    pub cookies: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_cookie_files")]
    pub cookie_files: Vec<CookieFileEntry>,
    #[serde(default)]
    pub open_browser_on_login: bool,
}

impl Default for YunshengAuthConfig {
    fn default() -> Self {
        Self {
            cookies: String::new(),
            account: String::new(),
            password: String::new(),
            cookie_files: default_cookie_files(),
            open_browser_on_login: false,
        }
    }
}

fn default_cookie_files() -> Vec<CookieFileEntry> {
    vec![CookieFileEntry {
        path: default_boss_cookie_path(),
        enabled: false,
    }]
}

fn default_boss_cookie_path() -> String {
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return format!("{home}/.cc-switch/skills/字段清单工具/scripts/.boss_cookie");
        }
    }
    if let Some(base) = directories::BaseDirs::new() {
        return base
            .home_dir()
            .join(".cc-switch/skills/字段清单工具/scripts/.boss_cookie")
            .display()
            .to_string();
    }
    // 最后兜底：本机绝对路径（勿用 ~，前端/写文件都需要可直接使用的路径）
    "/home/bing/.cc-switch/skills/字段清单工具/scripts/.boss_cookie".to_string()
}

pub fn has_credentials(config: &YunshengAuthConfig) -> bool {
    !config.account.trim().is_empty() && !config.password.is_empty()
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
    let mut config: YunshengAuthConfig =
        serde_json::from_str(&content).map_err(|e| format!("解析云生鉴权配置失败: {e}"))?;
    if config.cookie_files.is_empty() {
        config.cookie_files = default_cookie_files();
    }
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
    config.account = config.account.trim().to_string();
    config.cookies = if config.cookies.trim().is_empty() {
        String::new()
    } else {
        normalize_cookies_input(&config.cookies)?
    };
    config.cookie_files = normalize_cookie_files(config.cookie_files);
    if config.cookie_files.is_empty() {
        config.cookie_files = default_cookie_files();
    }
    save_yunsheng_auth_config_to_disk(&config)?;
    // 手动保存：按勾选写文件，不打开浏览器（与登录成功副作用规则一致中的「写文件」部分）
    if !config.cookies.is_empty() {
        let plan = plan_login_side_effects(&config);
        write_cookie_files(&plan.write_paths, &config.cookies)?;
    }
    Ok(config)
}

fn normalize_cookie_files(entries: Vec<CookieFileEntry>) -> Vec<CookieFileEntry> {
    entries
        .into_iter()
        .map(|mut e| {
            e.path = e.path.trim().to_string();
            e
        })
        .filter(|e| !e.path.is_empty())
        .collect()
}

/// 规范化并校验用户粘贴的 Cookie：非空，且包含 `token_inner=` 名值对。
pub fn normalize_cookies_input(raw: &str) -> Result<String, String> {
    let mut cookies = raw.trim().to_string();
    if cookies.to_ascii_lowercase().starts_with("cookie:") {
        cookies = cookies[7..].trim().to_string();
    }
    cookies = cookies.trim_end_matches(';').trim().to_string();
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
    Ok(normalize_cookie_pairs(cookies))
}

fn normalize_cookie_pairs(cookies: &str) -> String {
    cookies
        .split(';')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() || !part.contains('=') {
                return None;
            }
            Some(part.to_string())
        })
        .collect::<Vec<_>>()
        .join("; ")
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
#[allow(dead_code)] // 供其他业务模块直接取 Cookie；订单订阅走 with_auth_retry
pub fn get_cookies() -> Result<String, String> {
    let config = load_yunsheng_auth_config()?;
    require_cookies(&config.cookies)
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

/// 校验云生管理端业务响应。
pub fn ensure_yunsheng_business_ok(body: &Value) -> Result<(), String> {
    if let Some(err) = map_auth_error(200, Some(body)) {
        return Err(err);
    }

    let msg = body_message(body).unwrap_or_else(|| "业务失败".to_string());

    if body.get("status").and_then(|v| v.as_bool()) == Some(false) {
        return Err(format!("云生接口失败: {msg}"));
    }
    if body.get("success").and_then(|v| v.as_bool()) == Some(false) {
        return Err(format!("云生接口失败: {msg}"));
    }

    if let Some(code) = json_code_as_i64(body.get("code")) {
        if !matches!(code, 0 | 200) {
            return Err(format!("云生接口失败 ({code}): {msg}"));
        }
    }

    Ok(())
}

fn body_message(value: &Value) -> Option<String> {
    for key in ["msg", "message", "error", "errorMessage", "errorMsg"] {
        if let Some(s) = value.get(key).and_then(|v| v.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn json_code_as_i64(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    match value {
        Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn looks_like_auth_failure(value: &Value) -> bool {
    if let Some(code) = json_code_as_i64(value.get("code")).or_else(|| {
        value
            .get("status")
            .and_then(|v| v.as_u64().map(|u| u as i64).or_else(|| v.as_i64()))
    }) {
        if code == 401 || code == 30000 {
            return true;
        }
    }

    let mut texts: Vec<String> = Vec::new();
    if let Some(s) = body_message(value) {
        texts.push(s);
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
        "没有认证",
        "非法访问",
        "认证失败",
        "无权限",
        "权限不足",
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

pub fn is_auth_failure_error(err: &str) -> bool {
    err == EXPIRED_TOKEN_ERROR
        || err == AUTH_NEED_CREDENTIALS_ERROR
        || err.contains("云生登录已失效")
        || message_indicates_auth_failure(err)
}

/// 登录成功副作用计划（纯逻辑，便于单测）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoginSideEffectPlan {
    pub write_paths: Vec<String>,
    pub open_browser: bool,
}

pub fn plan_login_side_effects(config: &YunshengAuthConfig) -> LoginSideEffectPlan {
    LoginSideEffectPlan {
        write_paths: config
            .cookie_files
            .iter()
            .filter(|e| e.enabled && !e.path.trim().is_empty())
            .map(|e| e.path.trim().to_string())
            .collect(),
        open_browser: config.open_browser_on_login,
    }
}

/// 鉴权失败后是否应登录并重试（纯逻辑）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthRetryDecision {
    /// 已有 Cookie，直接请求，不登录
    UseCookies,
    /// 无有效 Cookie 但有账号，先登录再请求
    LoginBeforeRequest,
    /// 请求鉴权失败后登录并重试一次
    LoginAndRetry,
    /// 无账号且无法继续
    FailNoCredentials,
    /// 已登录过仍失败，不再重试
    FailAlreadyRetried,
}

pub fn decide_initial_auth(cookies_ok: bool, has_credentials: bool) -> AuthRetryDecision {
    if cookies_ok {
        AuthRetryDecision::UseCookies
    } else if has_credentials {
        AuthRetryDecision::LoginBeforeRequest
    } else {
        AuthRetryDecision::FailNoCredentials
    }
}

pub fn decide_after_auth_failure(
    already_logged_in: bool,
    has_credentials: bool,
) -> AuthRetryDecision {
    if already_logged_in {
        AuthRetryDecision::FailAlreadyRetried
    } else if has_credentials {
        AuthRetryDecision::LoginAndRetry
    } else {
        AuthRetryDecision::FailNoCredentials
    }
}

pub fn write_cookie_files(paths: &[String], cookies: &str) -> Result<(), String> {
    let cookies = normalize_cookie_pairs(cookies);
    for path in paths {
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建 Cookie 文件目录失败 {}: {e}", parent.display()))?;
        }
        fs::write(&path, &cookies)
            .map_err(|e| format!("写入 Cookie 文件失败 {}: {e}", path.display()))?;
    }
    Ok(())
}

async fn open_shebaorobot_with_cookies(
    app: &AppHandle,
    cookies: &str,
) -> Result<browser_bridge::OpenWithCookiesResponse, String> {
    browser_bridge::open_browser_with_url_cookie_header(
        app.clone(),
        OpenUrlWithCookieHeaderRequest {
            target_url: SHEBAOROBOT_URL.to_string(),
            cookie_header: cookies.to_string(),
        },
    )
    .await
}

async fn apply_login_side_effects(
    app: Option<&AppHandle>,
    config: &YunshengAuthConfig,
    cookies: &str,
    browser_error_is_fatal: bool,
) -> Result<(), String> {
    let plan = plan_login_side_effects(config);
    write_cookie_files(&plan.write_paths, cookies)?;

    if !plan.open_browser {
        return Ok(());
    }

    let Some(app) = app else {
        if browser_error_is_fatal {
            return Err("无法打开浏览器：应用句柄不可用".to_string());
        }
        return Ok(());
    };

    match open_shebaorobot_with_cookies(app, cookies).await {
        Ok(_) => Ok(()),
        Err(err) if browser_error_is_fatal => Err(err),
        Err(_) => Ok(()),
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn encrypt_password_sm2(public_key: &str, ts: i64, password: &str) -> Result<String, String> {
    let mut nonce_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = bytes_to_hex(&nonce_bytes);
    // 字段顺序与 SSO 前端 / sm-crypto 一致：{ts, nonce, password}
    let plain = json!({
        "ts": ts,
        "nonce": nonce,
        "password": password,
    })
    .to_string();

    let cipher = coter_core::crypto::process_sm2(
        Some(public_key),
        None,
        Some("C1C3C2"),
        "text",
        "hex",
        "lower",
        &plain,
        "encrypt",
    )?;
    // sm-crypto doEncrypt 输出不含未压缩点前缀 04；带 04 会被网关判为「密码格式错误」
    Ok(strip_sm2_cipher_uncompressed_prefix(&cipher))
}

/// 对齐 sm-crypto：C1 为 x||y（无 0x04 前缀）。
fn strip_sm2_cipher_uncompressed_prefix(cipher_hex: &str) -> String {
    let hex = cipher_hex.trim();
    if hex.len() >= 2 && hex[..2].eq_ignore_ascii_case("04") {
        hex[2..].to_string()
    } else {
        hex.to_string()
    }
}

fn merge_cookie(base_cookie: &str, set_cookie_headers: &[String]) -> String {
    let mut jar: Vec<(String, String)> = Vec::new();

    let push_pair = |jar: &mut Vec<(String, String)>, key: &str, value: &str| {
        let key = key.trim();
        if key.is_empty() {
            return;
        }
        if let Some((_, existing)) = jar.iter_mut().find(|(k, _)| k == key) {
            *existing = value.trim().to_string();
        } else {
            jar.push((key.to_string(), value.trim().to_string()));
        }
    };

    let base = normalize_cookie_pairs(base_cookie);
    if !base.is_empty() {
        for seg in base.split("; ") {
            if let Some((k, v)) = seg.split_once('=') {
                push_pair(&mut jar, k, v);
            }
        }
    }

    for raw in set_cookie_headers {
        let first = raw.split(';').next().unwrap_or("").trim();
        if let Some((k, v)) = first.split_once('=') {
            push_pair(&mut jar, k, v);
        }
    }

    jar.into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn cookie_value(cookie: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    for seg in normalize_cookie_pairs(cookie).split("; ") {
        if let Some(v) = seg.strip_prefix(&prefix) {
            return Some(v.to_string());
        }
    }
    None
}

fn collect_set_cookie_headers(response: &reqwest::Response) -> Vec<String> {
    response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .collect()
}

/// 执行 SM2 全自动登录，写回配置 Cookie，并按勾选执行副作用。
///
/// `browser_error_is_fatal`: 「立即登录」为 true；任务静默重登为 false（浏览器失败不阻断业务）。
pub async fn login_yunsheng(
    app: Option<&AppHandle>,
    browser_error_is_fatal: bool,
) -> Result<YunshengAuthConfig, String> {
    let mut config = load_yunsheng_auth_config()?;
    if !has_credentials(&config) {
        return Err(MISSING_CREDENTIALS_ERROR.to_string());
    }

    let cookies = perform_sm2_login(&config.account, &config.password).await?;
    require_cookies(&cookies)?;

    config.cookies = cookies.clone();
    save_yunsheng_auth_config_to_disk(&config)?;
    apply_login_side_effects(app, &config, &cookies, browser_error_is_fatal).await?;
    Ok(config)
}

async fn perform_sm2_login(account: &str, password: &str) -> Result<String, String> {
    let proxy = http_client::load_http_proxy_config().unwrap_or_default();
    let client = build_yunsheng_http_client(&proxy)?;

    let preflight_url = format!(
        "{PREFLIGHT_URL}?_={}",
        chrono::Utc::now().timestamp_millis()
    );
    let pre_resp = client
        .get(&preflight_url)
        .header(header::ORIGIN, SSO_ORIGIN)
        .header(header::REFERER, format!("{SSO_ORIGIN}/"))
        .send()
        .await
        .map_err(|e| format!("云生 preflight 请求失败: {e}"))?;
    let pre_status = pre_resp.status().as_u16();
    let pre: Value = pre_resp
        .json()
        .await
        .map_err(|e| format!("解析云生 preflight 响应失败: {e}"))?;
    if !(200..300).contains(&pre_status) || pre.get("status").and_then(|v| v.as_bool()) != Some(true)
    {
        return Err(format!(
            "云生 preflight 失败: {}",
            serde_json::to_string(&pre).unwrap_or_else(|_| pre.to_string())
        ));
    }
    let data = pre.get("data").cloned().unwrap_or(Value::Null);
    let key = data
        .get("key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "云生 preflight 未返回公钥 key".to_string())?;
    let ts = data
        .get("ts")
        .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)))
        .ok_or_else(|| "云生 preflight 未返回 ts".to_string())?;

    let enc_password = encrypt_password_sm2(key, ts, password)?;

    let login_resp = client
        .post(LOGIN_URL)
        .header(header::ORIGIN, SSO_ORIGIN)
        .header(header::REFERER, format!("{SSO_ORIGIN}/"))
        .json(&json!({
            "sysType": "inner",
            "account": account.trim(),
            "password": enc_password,
        }))
        .send()
        .await
        .map_err(|e| format!("云生登录请求失败: {e}"))?;
    let login_status = login_resp.status().as_u16();
    let login_body: Value = login_resp
        .json()
        .await
        .map_err(|e| format!("解析云生登录响应失败: {e}"))?;
    if !(200..300).contains(&login_status)
        || login_body.get("status").and_then(|v| v.as_bool()) != Some(true)
    {
        let msg = body_message(&login_body).unwrap_or_else(|| "登录失败".to_string());
        return Err(format!("云生登录失败: {msg}"));
    }
    let token = login_body
        .pointer("/data/token/token")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "云生登录成功但未返回 token".to_string())?
        .to_string();

    let mut auto_url = Url::parse(AUTO_LOGIN_URL).map_err(|e| format!("autoLogin URL 无效: {e}"))?;
    auto_url.query_pairs_mut().append_pair("token", &token);

    let auto_resp = client
        .post(auto_url)
        .header(header::ORIGIN, SSO_ORIGIN)
        .header(header::REFERER, format!("{SSO_ORIGIN}/"))
        .header(header::CONTENT_LENGTH, "0")
        .send()
        .await
        .map_err(|e| format!("云生 autoLogin 请求失败: {e}"))?;
    let set_cookies = collect_set_cookie_headers(&auto_resp);
    let _auto_body = auto_resp.text().await.unwrap_or_default();

    let mut cookie = format!("token_inner={token}");
    cookie = merge_cookie(&cookie, &set_cookies);
    if cookie_value(&cookie, "token_inner").is_none() {
        cookie = merge_cookie(&cookie, &[format!("token_inner={token}")]);
    }
    require_cookies(&cookie)
}

/// 带一次自动登录重试的业务请求辅助。
///
/// `execute` 接收当前 Cookie，返回 `(http_status, body)`；鉴权失败判定沿用 `map_auth_error` /
/// `ensure_yunsheng_business_ok`。
pub async fn with_auth_retry<F, Fut>(
    app: Option<&AppHandle>,
    mut execute: F,
) -> Result<Value, String>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Result<(u16, Value), String>>,
{
    let config = load_yunsheng_auth_config()?;
    let credentials = has_credentials(&config);
    let cookies_ok = require_cookies(&config.cookies).is_ok();

    let (mut cookies, mut already_logged_in) = match decide_initial_auth(cookies_ok, credentials) {
        AuthRetryDecision::UseCookies => (require_cookies(&config.cookies)?, false),
        AuthRetryDecision::LoginBeforeRequest => {
            let logged = login_yunsheng(app, false).await?;
            (logged.cookies, true)
        }
        AuthRetryDecision::FailNoCredentials => {
            return Err(if config.cookies.trim().is_empty() {
                MISSING_COOKIES_ERROR.to_string()
            } else {
                INVALID_COOKIES_ERROR.to_string()
            });
        }
        other => {
            return Err(format!("云生鉴权状态异常: {other:?}"));
        }
    };

    loop {
        let (status, body) = execute(cookies.clone()).await?;

        if let Some(auth_err) = map_auth_error(status, Some(&body)) {
            match decide_after_auth_failure(already_logged_in, credentials) {
                AuthRetryDecision::LoginAndRetry => {
                    let logged = login_yunsheng(app, false).await?;
                    cookies = logged.cookies;
                    already_logged_in = true;
                    continue;
                }
                AuthRetryDecision::FailNoCredentials => {
                    return Err(AUTH_NEED_CREDENTIALS_ERROR.to_string());
                }
                AuthRetryDecision::FailAlreadyRetried => {
                    return Err(auth_err);
                }
                other => {
                    return Err(format!("云生鉴权重试状态异常: {other:?}"));
                }
            }
        }

        if !(200..300).contains(&status) {
            let msg = body_message(&body).unwrap_or_else(|| "请求失败".to_string());
            return Err(format!("云生接口错误 ({status}): {msg}"));
        }

        // 业务层也可能返回鉴权失败（如 code 30000）
        match ensure_yunsheng_business_ok(&body) {
            Ok(()) => return Ok(body),
            Err(err) if is_auth_failure_error(&err) => {
                match decide_after_auth_failure(already_logged_in, credentials) {
                    AuthRetryDecision::LoginAndRetry => {
                        let logged = login_yunsheng(app, false).await?;
                        cookies = logged.cookies;
                        already_logged_in = true;
                        continue;
                    }
                    AuthRetryDecision::FailNoCredentials => {
                        return Err(AUTH_NEED_CREDENTIALS_ERROR.to_string());
                    }
                    AuthRetryDecision::FailAlreadyRetried => return Err(err),
                    other => {
                        return Err(format!("云生鉴权重试状态异常: {other:?}"));
                    }
                }
            }
            Err(err) => return Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

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
    fn normalize_strips_cookie_prefix() {
        let cookies = normalize_cookies_input("Cookie: token_inner=abc").unwrap();
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
    fn map_auth_error_on_yunsheng_gateway_30000() {
        let body = json!({
            "code": 30000,
            "message": "非法访问,没有认证",
            "status": false
        });
        let mapped = map_auth_error(200, Some(&body)).expect("30000 should map");
        assert_eq!(mapped, EXPIRED_TOKEN_ERROR);
    }

    #[test]
    fn ensure_business_ok_accepts_code_zero() {
        let body = json!({"code": 0, "data": {"records": []}});
        assert!(ensure_yunsheng_business_ok(&body).is_ok());
    }

    #[test]
    fn ensure_business_ok_accepts_yunsheng_code_200() {
        let body = json!({
            "code": 200,
            "message": "ok",
            "status": true,
            "data": { "records": [] }
        });
        assert!(ensure_yunsheng_business_ok(&body).is_ok());
    }

    #[test]
    fn ensure_business_ok_rejects_nonzero_code() {
        let body = json!({"code": 500, "msg": "服务器繁忙", "status": false});
        let err = ensure_yunsheng_business_ok(&body).expect_err("nonzero code");
        assert!(err.contains("500") || err.contains("服务器繁忙"));
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
    fn valid_cookies_do_not_trigger_login_before_request() {
        assert_eq!(
            decide_initial_auth(true, true),
            AuthRetryDecision::UseCookies
        );
        assert_eq!(
            decide_initial_auth(true, false),
            AuthRetryDecision::UseCookies
        );
    }

    #[test]
    fn auth_failure_with_credentials_logs_in_and_retries_once() {
        assert_eq!(
            decide_after_auth_failure(false, true),
            AuthRetryDecision::LoginAndRetry
        );
        assert_eq!(
            decide_after_auth_failure(true, true),
            AuthRetryDecision::FailAlreadyRetried
        );
    }

    #[test]
    fn auth_failure_without_credentials_fails_clearly() {
        assert_eq!(
            decide_after_auth_failure(false, false),
            AuthRetryDecision::FailNoCredentials
        );
        assert_eq!(
            decide_initial_auth(false, false),
            AuthRetryDecision::FailNoCredentials
        );
    }

    #[test]
    fn login_side_effects_follow_checkboxes() {
        let dir = tempdir().unwrap();
        let enabled = dir.path().join("enabled.cookie");
        let disabled = dir.path().join("disabled.cookie");
        let config = YunshengAuthConfig {
            cookies: "token_inner=abc".into(),
            account: "u".into(),
            password: "p".into(),
            cookie_files: vec![
                CookieFileEntry {
                    path: enabled.to_string_lossy().to_string(),
                    enabled: true,
                },
                CookieFileEntry {
                    path: disabled.to_string_lossy().to_string(),
                    enabled: false,
                },
            ],
            open_browser_on_login: true,
        };

        let plan = plan_login_side_effects(&config);
        assert_eq!(plan.write_paths.len(), 1);
        assert!(plan.write_paths[0].ends_with("enabled.cookie"));
        assert!(plan.open_browser);

        write_cookie_files(&plan.write_paths, "token_inner=abc; SESSION=s1").unwrap();
        let written = fs::read_to_string(&enabled).unwrap();
        assert_eq!(written, "token_inner=abc; SESSION=s1");
        assert!(!disabled.exists());
    }

    #[test]
    fn login_side_effects_skip_browser_when_unchecked() {
        let plan = plan_login_side_effects(&YunshengAuthConfig {
            open_browser_on_login: false,
            ..YunshengAuthConfig::default()
        });
        assert!(!plan.open_browser);
    }

    #[test]
    fn merge_cookie_prefers_later_values() {
        let merged = merge_cookie(
            "token_inner=old; SESSION=a",
            &["SESSION=b; Path=/".into(), "foo=1; Path=/".into()],
        );
        assert!(merged.contains("token_inner=old"));
        assert!(merged.contains("SESSION=b"));
        assert!(merged.contains("foo=1"));
        assert!(!merged.contains("SESSION=a"));
    }

    #[test]
    fn old_config_json_only_cookies_deserializes() {
        let config: YunshengAuthConfig =
            serde_json::from_str(r#"{"cookies":"token_inner=x"}"#).unwrap();
        assert_eq!(config.cookies, "token_inner=x");
        assert!(config.account.is_empty());
        assert!(config.password.is_empty());
        assert!(!config.open_browser_on_login);
        // default cookie_files applied by serde default
        assert!(!config.cookie_files.is_empty());
        assert!(!config.cookie_files[0].enabled);
    }

    #[test]
    fn has_credentials_requires_both() {
        assert!(!has_credentials(&YunshengAuthConfig {
            account: "a".into(),
            password: "".into(),
            ..YunshengAuthConfig::default()
        }));
        assert!(has_credentials(&YunshengAuthConfig {
            account: "a".into(),
            password: "p".into(),
            ..YunshengAuthConfig::default()
        }));
    }

    #[test]
    fn encrypt_password_sm2_produces_hex_ciphertext() {
        // 使用 coter-core 测试向量公钥；只断言密文为非空 hex（随机 nonce）
        const PUBLIC_Q: &str = "049031694836FCCD075D20CC284278901F37AF7D1EF8DEA025393C4643CE577C9DB64DF3E331ECC5B105E40E6C65949B6B5F6E8F1D99D28B6E01539DAE723588F0";
        let cipher = encrypt_password_sm2(PUBLIC_Q, 1_700_000_000_000, "secret").unwrap();
        assert!(cipher.len() > 64);
        assert!(cipher.chars().all(|c| c.is_ascii_hexdigit()));
        // 必须对齐 sm-crypto：不得带未压缩点前缀 04
        assert!(
            !cipher.to_ascii_lowercase().starts_with("04"),
            "cipher must not start with 04 (got {})",
            &cipher[..4.min(cipher.len())]
        );
    }

    #[test]
    fn strip_sm2_prefix_removes_leading_04() {
        assert_eq!(
            strip_sm2_cipher_uncompressed_prefix("04aabb"),
            "aabb"
        );
        assert_eq!(strip_sm2_cipher_uncompressed_prefix("aabb"), "aabb");
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
