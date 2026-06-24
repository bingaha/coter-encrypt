use std::{fs, net::SocketAddr, path::PathBuf, time::Duration};

use directories::ProjectDirs;
use futures_util::{SinkExt, StreamExt};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri_plugin_opener::OpenerExt;
use tokio::{
 io::{AsyncReadExt, AsyncWriteExt},
 net::{TcpListener, TcpStream},
 time::timeout,
};
use tokio_tungstenite::{accept_hdr_async, tungstenite::handshake::server};
use url::{form_urlencoded, Url};

const CONFIG_DIR_QUALIFIER: &str = "com";
const CONFIG_DIR_ORGANIZATION: &str = "coter";
const CONFIG_DIR_APPLICATION: &str = "CoterEncrypt";
const PLUGIN_CONFIG_FILE_NAME: &str = "browser-bridge.json";
const URL_MAPPINGS_FILE_NAME: &str = "website-url-mappings.json";
const BRIDGE_TIMEOUT_SECONDS: u64 = 30;
const HTTP_REQUEST_LIMIT_BYTES: usize = 8192;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserBridgeConfig {
 #[serde(default)]
 pub extension_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebsiteUrlMapping {
 pub area_id: String,
 pub business_type: String,
 pub url: String,
 #[serde(default, skip_serializing_if = "Option::is_none")]
 pub storage_rules: Option<Vec<StorageRule>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRule {
 pub storage: String,
 pub key: String,
 pub source: StorageSource,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSource {
 #[serde(default, skip_serializing_if = "Option::is_none")]
 pub path: Option<String>,
 #[serde(default, skip_serializing_if = "Option::is_none")]
 pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageItem {
 pub storage: String,
 pub key: String,
 pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWithCookiesRequest {
 pub area_id: String,
 pub business_type: String,
 pub cert_info: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenWithCookiesResponse {
 pub target_url: String,
 pub written: usize,
 pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BridgePayload {
 r#type: &'static str,
 request_id: String,
 target_url: String,
 cookies: Vec<BridgeCookie>,
 #[serde(default, skip_serializing_if = "Vec::is_empty")]
 storage_items: Vec<StorageItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BridgeCookie {
 #[serde(default, skip_serializing_if = "Option::is_none")]
 domain: Option<String>,
 #[serde(default, skip_serializing_if = "Option::is_none")]
 expiration_date: Option<f64>,
 #[serde(default)]
 http_only: bool,
 name: String,
 #[serde(default = "default_cookie_path")]
 path: String,
 #[serde(default)]
 secure: bool,
 #[serde(default, skip_serializing_if = "Option::is_none")]
 same_site: Option<String>,
 value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeResult {
 r#type: Option<String>,
 request_id: Option<String>,
 ok: bool,
 #[serde(default)]
 written: usize,
 #[serde(default)]
 errors: Vec<String>,
}

pub fn load_browser_bridge_config() -> Result<BrowserBridgeConfig, String> {
 let path = browser_bridge_config_path()?;

 if !path.is_file() {
 return Ok(BrowserBridgeConfig {
 extension_id: String::new(),
 });
 }

 let content = fs::read_to_string(&path)
 .map_err(|error| format!("读取浏览器插件配置失败 {}: {error}", path.display()))?;

 serde_json::from_str(&content).map_err(|error| format!("解析浏览器插件配置失败: {error}"))
}

pub fn save_browser_bridge_config(
 config: BrowserBridgeConfig,
) -> Result<BrowserBridgeConfig, String> {
 let normalized = BrowserBridgeConfig {
 extension_id: config.extension_id.trim().to_string(),
 };

 validate_extension_id(&normalized.extension_id)?;
 write_json_file(&browser_bridge_config_path()?, &normalized)?;

 Ok(normalized)
}

pub fn load_website_url_mappings() -> Result<Vec<WebsiteUrlMapping>, String> {
 let path = website_url_mappings_path()?;

 if !path.is_file() {
 let mappings = default_url_mappings();
 write_json_file(&path, &mappings)?;
 return Ok(mappings);
 }

 let content = fs::read_to_string(&path)
 .map_err(|error| format!("读取网站地址映射失败 {}: {error}", path.display()))?;
 let mappings: Vec<WebsiteUrlMapping> =
 serde_json::from_str(&content).map_err(|error| format!("解析网站地址映射失败: {error}"))?;

 validate_url_mappings(&mappings)?;
 Ok(mappings)
}

pub fn save_website_url_mapping(
 mapping: WebsiteUrlMapping,
) -> Result<Vec<WebsiteUrlMapping>, String> {
 let normalized = WebsiteUrlMapping {
 area_id: mapping.area_id.trim().to_string(),
 business_type: mapping.business_type.trim().to_string(),
 url: mapping.url.trim().to_string(),
 storage_rules: mapping.storage_rules,
 };

 validate_url_mappings(std::slice::from_ref(&normalized))?;

 let mut mappings = load_website_url_mappings()?;
 if let Some(existing) = mappings.iter_mut().find(|item| {
 item.area_id.trim() == normalized.area_id
 && item.business_type.trim() == normalized.business_type
 }) {
 *existing = normalized;
 } else {
 mappings.push(normalized);
 }

 validate_url_mappings(&mappings)?;
 write_json_file(&website_url_mappings_path()?, &mappings)?;

 Ok(mappings)
}

pub fn open_app_config_dir(app: tauri::AppHandle) -> Result<(), String> {
 let dir = app_config_dir()?;
 fs::create_dir_all(&dir)
 .map_err(|error| format!("创建应用配置目录失败 {}: {error}", dir.display()))?;
 let dir_display = dir.display().to_string();

 app.opener()
 .open_path(dir_display, None::<&str>)
 .map_err(|error| format!("打开应用配置目录失败: {error}"))
}

pub async fn open_default_browser_with_cookies(
 app: tauri::AppHandle,
 request: OpenWithCookiesRequest,
) -> Result<OpenWithCookiesResponse, String> {
 let config = load_browser_bridge_config()?;
 validate_extension_id(&config.extension_id)?;

 let mapping = find_mapping(&request.area_id, &request.business_type)?;
 let target_url = mapping.url.clone();
 let cookies = extract_cookies(&request.cert_info)?;
 let storage_items = resolve_storage_items(
 &request.cert_info,
 mapping.storage_rules.as_deref(),
 )?;
 let request_id = random_token(18);
 let token = random_token(32);
 let listener = TcpListener::bind("127.0.0.1:0")
 .await
 .map_err(|error| format!("启动本地 WebSocket 失败: {error}"))?;
 let addr = listener
 .local_addr()
 .map_err(|error| format!("读取本地 WebSocket 端口失败: {error}"))?;

 let bridge_url = format!(
 "http://127.0.0.1:{}/bridge?token={}&extensionId={}",
 addr.port(),
 token,
 config.extension_id
 );

 let payload = BridgePayload {
 r#type: "openWithCookies",
 request_id: request_id.clone(),
 target_url: target_url.clone(),
 cookies,
 storage_items,
 };

 app.opener()
 .open_url(bridge_url, None::<&str>)
 .map_err(|error| format!("打开本地浏览器桥接页失败: {error}"))?;

 let result = timeout(
 Duration::from_secs(BRIDGE_TIMEOUT_SECONDS),
 serve_bridge_once(listener, addr, token, request_id, payload),
 )
 .await
 .map_err(|_| "等待浏览器插件响应超时".to_string())??;

 Ok(OpenWithCookiesResponse {
 target_url,
 written: result.written,
 errors: result.errors,
 })
}

async fn serve_bridge_once(
 listener: TcpListener,
 addr: SocketAddr,
 token: String,
 request_id: String,
 payload: BridgePayload,
) -> Result<BridgeResult, String> {
 loop {
 let (stream, _) = listener
 .accept()
 .await
 .map_err(|error| format!("等待浏览器插件连接失败: {error}"))?;

 if is_websocket_connection(&stream).await? {
 return serve_websocket_connection(stream, addr, &token, request_id, payload).await;
 }

 serve_http_connection(stream, &token).await?;
 }
}

async fn serve_websocket_connection(
 stream: TcpStream,
 addr: SocketAddr,
 token: &str,
 request_id: String,
 payload: BridgePayload,
) -> Result<BridgeResult, String> {
 let mut accepted = false;
 let mut rejected_reason = String::new();

 let mut websocket = accept_hdr_async(stream, |request: &server::Request, response| {
 if request.uri().path() == "/ws" && query_contains_token(request.uri().query(), token) {
 accepted = true;
 Ok(response)
 } else {
 rejected_reason = "浏览器插件连接令牌不匹配".to_string();
 Err(server::ErrorResponse::new(Some(rejected_reason.clone())))
 }
 })
 .await
 .map_err(|error| {
 if rejected_reason.is_empty() {
 format!("建立 WebSocket 连接失败 {addr}: {error}")
 } else {
 rejected_reason
 }
 })?;

 if !accepted {
 return Err("浏览器插件连接未通过校验".to_string());
 }

 let payload_text = serde_json::to_string(&payload)
 .map_err(|error| format!("序列化浏览器打开任务失败: {error}"))?;
 websocket
 .send(tokio_tungstenite::tungstenite::Message::Text(
 payload_text.into(),
 ))
 .await
 .map_err(|error| format!("发送浏览器打开任务失败: {error}"))?;

 let Some(message) = websocket.next().await else {
 return Err("浏览器插件未返回执行结果".to_string());
 };
 let message = message.map_err(|error| format!("读取浏览器插件结果失败: {error}"))?;
 let text = message
 .to_text()
 .map_err(|error| format!("浏览器插件结果不是文本消息: {error}"))?;
 let result: BridgeResult =
 serde_json::from_str(text).map_err(|error| format!("解析浏览器插件结果失败: {error}"))?;

 if result.r#type.as_deref() != Some("openWithCookiesResult") {
 return Err("浏览器插件结果类型不正确".to_string());
 }

 if result.request_id.as_deref() != Some(&request_id) {
 return Err("浏览器插件结果 requestId 不匹配".to_string());
 }

 if !result.ok {
 let message = if result.errors.is_empty() {
 "浏览器插件写入 Cookie 失败".to_string()
 } else {
 result.errors.join("; ")
 };
 return Err(message);
 }

 Ok(result)
}

async fn is_websocket_connection(stream: &TcpStream) -> Result<bool, String> {
 let mut buffer = [0_u8; 1024];
 let read = stream
 .peek(&mut buffer)
 .await
 .map_err(|error| format!("读取浏览器桥接请求失败: {error}"))?;

 if read == 0 {
 return Ok(false);
 }

 let request_head = String::from_utf8_lossy(&buffer[..read]).to_ascii_lowercase();
 Ok(request_head.starts_with("get /ws") && request_head.contains("\r\nupgrade: websocket"))
}

async fn serve_http_connection(mut stream: TcpStream, token: &str) -> Result<(), String> {
 let request = read_http_request(&mut stream).await?;
 let response = build_http_response(&request, token);

 stream
 .write_all(response.as_bytes())
 .await
 .map_err(|error| format!("写入本地浏览器桥接页失败: {error}"))?;
 stream
 .shutdown()
 .await
 .map_err(|error| format!("关闭本地浏览器桥接连接失败: {error}"))
}

async fn read_http_request(stream: &mut TcpStream) -> Result<String, String> {
 let mut buffer = Vec::new();
 let mut chunk = [0_u8; 1024];

 loop {
 let read = stream
 .read(&mut chunk)
 .await
 .map_err(|error| format!("读取本地浏览器桥接请求失败: {error}"))?;

 if read == 0 {
 break;
 }

 buffer.extend_from_slice(&chunk[..read]);

 if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
 break;
 }

 if buffer.len() > HTTP_REQUEST_LIMIT_BYTES {
 return Err("本地浏览器桥接请求过大".to_string());
 }
 }

 String::from_utf8(buffer).map_err(|error| format!("本地浏览器桥接请求不是 UTF-8: {error}"))
}

fn build_http_response(request: &str, token: &str) -> String {
 let Some(first_line) = request.lines().next() else {
 return http_text_response("400 Bad Request", "请求不完整");
 };

 let mut parts = first_line.split_whitespace();
 let method = parts.next().unwrap_or_default();
 let target = parts.next().unwrap_or_default();

 if method != "GET" {
 return http_text_response("405 Method Not Allowed", "只支持 GET 请求");
 }

 if target.starts_with("/favicon.ico") {
 return "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".to_string();
 }

 if !target.starts_with("/bridge") {
 return http_text_response("404 Not Found", "桥接页面不存在");
 }

 if !request_target_contains_token(target, token) {
 return http_text_response("403 Forbidden", "桥接令牌不正确");
 }

 http_html_response(bridge_page_html())
}

fn http_html_response(body: &str) -> String {
 format!(
 "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
 body.as_bytes().len(),
 body
 )
}

fn http_text_response(status: &str, message: &str) -> String {
 format!(
 "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
 message.as_bytes().len(),
 message
 )
}

fn bridge_page_html() -> &'static str {
 r#"<!doctype html>
<html lang="zh-CN">
 <head>
 <meta charset="UTF-8">
 <meta name="viewport" content="width=device-width, initial-scale=1.0">
 <title>Coter Cookie Bridge</title>
 <style>
 * { box-sizing: border-box; }
 body {
 min-width: 360px;
 min-height: 240px;
 margin: 0;
 display: grid;
 place-items: center;
 font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Arial, sans-serif;
 color: #1f2328;
 background: #f6f8fa;
 }
 main {
 width: min(420px, calc(100vw - 32px));
 padding: 28px;
 border: 1px solid #d8dee4;
 border-radius: 8px;
 background: #ffffff;
 box-shadow: 0 8px 24px rgba(140, 149, 159, 0.2);
 text-align: center;
 }
 .mark {
 width: 48px;
 height: 48px;
 margin: 0 auto 16px;
 display: grid;
 place-items: center;
 border-radius: 8px;
 color: #ffffff;
 background: #18a058;
 font-size: 22px;
 font-weight: 700;
 }
 h1 {
 margin: 0 0 10px;
 font-size: 20px;
 line-height: 1.25;
 }
 p {
 margin: 0;
 color: #57606a;
 font-size: 14px;
 line-height: 1.6;
 overflow-wrap: anywhere;
 }
 </style>
 </head>
 <body>
 <main>
 <div class="mark">C</div>
 <h1>正在写入登录态</h1>
 <p id="status">正在等待浏览器插件连接...</p>
 </main>
 </body>
</html>"#
}

fn request_target_contains_token(target: &str, expected_token: &str) -> bool {
 let query = target.split_once('?').map(|(_, query)| query);
 query_contains_token(query, expected_token)
}

fn query_contains_token(query: Option<&str>, expected_token: &str) -> bool {
 query
 .map(|query| {
 form_urlencoded::parse(query.as_bytes())
 .any(|(key, value)| key == "token" && value == expected_token)
 })
 .unwrap_or(false)
}

fn find_mapping(area_id: &str, business_type: &str) -> Result<WebsiteUrlMapping, String> {
 let area_id = area_id.trim();
 let business_type = business_type.trim();

 if area_id.is_empty() {
 return Err("area_id 为空，无法匹配网站地址".to_string());
 }

 if business_type.is_empty() {
 return Err("办理类型为空，无法匹配网站地址".to_string());
 }

 let mappings = load_website_url_mappings()?;
 mappings
 .into_iter()
 .find(|mapping| {
 mapping.area_id.trim() == area_id && mapping.business_type.trim() == business_type
 })
 .ok_or_else(|| format!("未配置网站地址映射: area_id={area_id}, 办理类型={business_type}"))
}

fn extract_cookies(cert_info: &str) -> Result<Vec<BridgeCookie>, String> {
 if cert_info.trim().is_empty() {
 return Err("cert_info 为空，无法提取 Cookie".to_string());
 }

 let value: Value = serde_json::from_str(cert_info)
 .map_err(|error| format!("cert_info 不是合法 JSON: {error}"))?;
 let cookies_value = value
 .get("cookies")
 .or_else(|| value.get("cookie"))
 .ok_or_else(|| "cert_info 中未找到 cookies 或 cookie 字段".to_string())?;

 let cookies: Vec<BridgeCookie> = match cookies_value {
 Value::String(text) => serde_json::from_str(text)
 .map_err(|error| format!("cookies 字段不是合法 JSON 数组字符串: {error}"))?,
 Value::Array(_) => serde_json::from_value(cookies_value.clone())
 .map_err(|error| format!("cookies 字段不是合法 JSON 数组: {error}"))?,
 _ => return Err("cookies 字段类型不支持".to_string()),
 };

 let cookies: Vec<BridgeCookie> = cookies
 .into_iter()
 .map(normalize_cookie)
 .collect::<Result<Vec<_>, _>>()?;

 if cookies.is_empty() {
 return Err("cookies 列表为空".to_string());
 }

 Ok(cookies)
}

fn normalize_cookie(mut cookie: BridgeCookie) -> Result<BridgeCookie, String> {
 cookie.name = cookie.name.trim().to_string();
 cookie.path = if cookie.path.trim().is_empty() {
 "/".to_string()
 } else {
 cookie.path.trim().to_string()
 };

 if cookie.name.is_empty() {
 return Err("Cookie name 不能为空".to_string());
 }

 if !cookie.path.starts_with('/') {
 cookie.path = format!("/{}", cookie.path);
 }

 if let Some(domain) = cookie.domain {
 let domain = domain.trim().trim_start_matches('.').to_string();
 cookie.domain = if domain.is_empty() {
 None
 } else {
 Some(domain)
 };
 }

 Ok(cookie)
}

fn validate_extension_id(extension_id: &str) -> Result<(), String> {
 let extension_id = extension_id.trim();

 if extension_id.is_empty() {
 return Err("请先配置浏览器插件 ID".to_string());
 }

 let is_valid =
 extension_id.len() == 32 && extension_id.bytes().all(|byte| matches!(byte, b'a'..=b'p'));

 if !is_valid {
 return Err("浏览器插件 ID 格式不正确".to_string());
 }

 Ok(())
}

fn validate_url_mappings(mappings: &[WebsiteUrlMapping]) -> Result<(), String> {
 for mapping in mappings {
 if mapping.area_id.trim().is_empty() {
 return Err("网站地址映射 areaId 不能为空".to_string());
 }

 if mapping.business_type.trim().is_empty() {
 return Err("网站地址映射 businessType 不能为空".to_string());
 }

 let parsed = Url::parse(&mapping.url)
 .map_err(|error| format!("网站地址映射 URL 不合法 {}: {error}", mapping.url))?;
 if !matches!(parsed.scheme(), "http" | "https") {
 return Err(format!(
 "网站地址映射 URL 只支持 http/https: {}",
 mapping.url
 ));
 }
 }

 Ok(())
}

fn default_url_mappings() -> Vec<WebsiteUrlMapping> {
 vec![WebsiteUrlMapping {
 area_id: "289".to_string(),
 business_type: "社保".to_string(),
 url: "https://sbwx.rst.shanxi.gov.cn:8007/ylwxsb/index.shtml".to_string(),
 storage_rules: None,
 }]
}

fn random_token(length: usize) -> String {
 rand::thread_rng()
 .sample_iter(&Alphanumeric)
 .take(length)
 .map(char::from)
 .collect()
}

fn write_json_file<T: Serialize>(path: &PathBuf, value: &T) -> Result<(), String> {
 let parent = path
 .parent()
 .ok_or_else(|| format!("配置路径不合法: {}", path.display()))?;

 fs::create_dir_all(parent)
 .map_err(|error| format!("创建配置目录失败 {}: {error}", parent.display()))?;
 let content =
 serde_json::to_string_pretty(value).map_err(|error| format!("序列化配置失败: {error}"))?;

 fs::write(path, content).map_err(|error| format!("写入配置失败 {}: {error}", path.display()))
}

fn browser_bridge_config_path() -> Result<PathBuf, String> {
 Ok(app_config_dir()?.join(PLUGIN_CONFIG_FILE_NAME))
}

fn website_url_mappings_path() -> Result<PathBuf, String> {
 Ok(app_config_dir()?.join(URL_MAPPINGS_FILE_NAME))
}

fn app_config_dir() -> Result<PathBuf, String> {
 let dirs = ProjectDirs::from(
 CONFIG_DIR_QUALIFIER,
 CONFIG_DIR_ORGANIZATION,
 CONFIG_DIR_APPLICATION,
 )
 .ok_or_else(|| "无法定位应用配置目录".to_string())?;

 Ok(dirs.config_dir().to_path_buf())
}

fn default_cookie_path() -> String {
 "/".to_string()
}

fn extract_storage_value(cert_info: &str, source: &StorageSource) -> Result<String, String> {
 if let Some(path) = source.path.as_deref() {
 let path = path.trim();
 if path.is_empty() {
 return Err("source.path 不能为空".to_string());
 }

 let value: Value =
 serde_json::from_str(cert_info).map_err(|error| format!("cert_info 不是合法 JSON: {error}"))?;

 let parts: Vec<&str> = path.split('.').collect();
 let mut current = &value;

 for part in &parts {
 current = current
 .get(part)
 .ok_or_else(|| format!("cert_info 中未找到字段: {path}"))?;
 }

 match current {
 Value::String(s) => Ok(s.clone()),
 other => Ok(other.to_string()),
 }
 } else if let Some(value) = source.value.as_deref() {
 Ok(value.to_string())
 } else {
 Err("source 必须指定 path 或 value".to_string())
 }
}

fn resolve_storage_items(
 cert_info: &str,
 rules: Option<&[StorageRule]>,
) -> Result<Vec<StorageItem>, String> {
 let Some(rules) = rules else {
 return Ok(Vec::new());
 };

 if rules.is_empty() {
 return Ok(Vec::new());
 }

 let mut items = Vec::with_capacity(rules.len());

 for rule in rules {
 let storage = rule.storage.trim();
 if storage != "sessionStorage" && storage != "localStorage" {
 return Err(format!(
 "不支持的存储类型: {storage}，仅支持 sessionStorage 或 localStorage"
 ));
 }

 let key = rule.key.trim();
 if key.is_empty() {
 return Err("存储规则的 key 不能为空".to_string());
 }

 let value = extract_storage_value(cert_info, &rule.source)?;

 items.push(StorageItem {
 storage: storage.to_string(),
 key: key.to_string(),
 value,
 });
 }

 Ok(items)
}

#[cfg(test)]
mod tests {
 use super::{extract_cookies, validate_extension_id};

 #[test]
 fn extracts_cookies_string_field() {
 let cert_info = r#"{"cookies":"[{\"name\":\"SESSION\",\"value\":\"abc\",\"path\":\"app\",\"secure\":true}]"}"#;
 let cookies = extract_cookies(cert_info).unwrap();

 assert_eq!(cookies.len(), 1);
 assert_eq!(cookies[0].name, "SESSION");
 assert_eq!(cookies[0].path, "/app");
 assert!(cookies[0].secure);
 }

 #[test]
 fn extracts_cookie_array_field() {
 let cert_info = r#"{"cookie":[{"name":"JSESSIONID","value":"abc"}]}"#;
 let cookies = extract_cookies(cert_info).unwrap();

 assert_eq!(cookies.len(), 1);
 assert_eq!(cookies[0].name, "JSESSIONID");
 assert_eq!(cookies[0].path, "/");
 }

 #[test]
 fn extension_id_must_be_chrome_format() {
 assert!(validate_extension_id("abcdefghijklmnopabcdefghijklmnop").is_ok());
 assert!(validate_extension_id("abc").is_err());
 assert!(validate_extension_id("abcdefghijklmnopabcdefghijklmnox").is_err());
 }

 #[test]
 fn extracts_storage_value_from_top_level_field() {
 let cert_info = r#"{"token":"eyJhbGciOiJIUzI1NiJ9","secretKey":"ABC123"}"#;
 let source = super::StorageSource {
 path: Some("token".to_string()),
 value: None,
 };
 let value = super::extract_storage_value(cert_info, &source).unwrap();
 assert_eq!(value, "eyJhbGciOiJIUzI1NiJ9");
 }

 #[test]
 fn extracts_storage_value_from_nested_field() {
 let cert_info = r#"{"data":{"user":{"token":"nested_value"}}}"#;
 let source = super::StorageSource {
 path: Some("data.user.token".to_string()),
 value: None,
 };
 let value = super::extract_storage_value(cert_info, &source).unwrap();
 assert_eq!(value, "nested_value");
 }

 #[test]
 fn extracts_storage_fixed_value() {
 let cert_info = r#"{"token":"abc"}"#;
 let source = super::StorageSource {
 path: None,
 value: Some("{}".to_string()),
 };
 let value = super::extract_storage_value(cert_info, &source).unwrap();
 assert_eq!(value, "{}");
 }

 #[test]
 fn extracts_storage_value_fails_for_missing_field() {
 let cert_info = r#"{"token":"abc"}"#;
 let source = super::StorageSource {
 path: Some("missing".to_string()),
 value: None,
 };
 assert!(super::extract_storage_value(cert_info, &source).is_err());
 }

 #[test]
 fn resolves_storage_items_with_mixed_rules() {
 let cert_info = r#"{"token":"jwt_value","secret":"key123"}"#;
 let rules = vec![
 super::StorageRule {
 storage: "sessionStorage".to_string(),
 key: "token".to_string(),
 source: super::StorageSource {
 path: Some("token".to_string()),
 value: None,
 },
 },
 super::StorageRule {
 storage: "localStorage".to_string(),
 key: "redux".to_string(),
 source: super::StorageSource {
 path: None,
 value: Some("{}".to_string()),
 },
 },
 ];

 let items = super::resolve_storage_items(cert_info, Some(&rules)).unwrap();
 assert_eq!(items.len(), 2);
 assert_eq!(items[0].storage, "sessionStorage");
 assert_eq!(items[0].key, "token");
 assert_eq!(items[0].value, "jwt_value");
 assert_eq!(items[1].storage, "localStorage");
 assert_eq!(items[1].key, "redux");
 assert_eq!(items[1].value, "{}");
 }
}
