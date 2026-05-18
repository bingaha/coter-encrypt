use std::{
 fs,
 path::{Path, PathBuf},
 time::Duration,
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use sqlx::{mysql::MySqlPoolOptions, MySqlPool, Row};
use windows_sys::Win32::{
 Foundation::LocalFree,
 Security::Cryptography::{
 CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
 },
};

const CONFIG_FILE_NAME: &str = "mysql-datasource.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlDatasourceConfig {
 pub host: String,
 pub port: u16,
 pub database: String,
 pub username: String,
 #[serde(default, skip_serializing_if = "Option::is_none")]
 pub password: Option<String>,
 #[serde(default = "default_connect_timeout_seconds")]
 pub connect_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlDatasourceConfigView {
 pub host: String,
 pub port: u16,
 pub database: String,
 pub username: String,
 pub has_password: bool,
 pub connect_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredMysqlDatasourceConfig {
 host: String,
 port: u16,
 database: String,
 username: String,
 #[serde(default, skip_serializing_if = "String::is_empty")]
 encrypted_password: String,
 #[serde(default = "default_connect_timeout_seconds")]
 connect_timeout_seconds: u64,
}

#[derive(Debug, Clone)]
struct ResolvedMysqlDatasourceConfig {
 host: String,
 port: u16,
 database: String,
 username: String,
 password: String,
 connect_timeout_seconds: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertQueryRequest {
 pub main_name: String,
 pub business_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertQueryAccount {
 pub id: Option<String>,
 pub company_name: Option<String>,
 pub login_key: String,
 pub business_type: Option<String>,
 pub dwbh: Option<String>,
 pub area_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertQueryCert {
 pub cert_info: String,
 pub update_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertQueryItem {
 pub account: CertQueryAccount,
 pub cert: Option<CertQueryCert>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CertQueryResponse {
 pub items: Vec<CertQueryItem>,
}

pub async fn load_mysql_datasource_config() -> Result<Option<MysqlDatasourceConfigView>, String> {
 let Some(stored) = read_stored_config()? else {
 return Ok(None);
 };

 Ok(Some(to_config_view(&stored)))
}

pub async fn save_mysql_datasource_config(
 config: MysqlDatasourceConfig,
) -> Result<MysqlDatasourceConfigView, String> {
 validate_config(&config)?;

 let previous = read_stored_config()?;
 let password = match config.password {
 Some(password) if !password.is_empty() => password,
 _ => previous
 .as_ref()
 .map(decrypt_stored_password)
 .transpose()?
 .unwrap_or_default(),
 };

 if password.is_empty() {
 return Err("数据库密码不能为空".to_string());
 }

 let stored = StoredMysqlDatasourceConfig {
 host: config.host.trim().to_string(),
 port: config.port,
 database: config.database.trim().to_string(),
 username: config.username.trim().to_string(),
 encrypted_password: encrypt_password(&password)?,
 connect_timeout_seconds: config.connect_timeout_seconds,
 };

 write_stored_config(&stored)?;
 Ok(to_config_view(&stored))
}

pub async fn test_mysql_datasource(config: Option<MysqlDatasourceConfig>) -> Result<(), String> {
 let stored = resolve_config(config)?;
 let pool = connect_pool(&stored).await?;
 sqlx::query("SELECT 1")
 .execute(&pool)
 .await
 .map_err(|error| format!("测试 MySQL 连接失败: {error}"))?;
 pool.close().await;

 Ok(())
}

pub async fn query_cert_info(request: CertQueryRequest) -> Result<CertQueryResponse, String> {
 let main_name = request.main_name.trim();
 let business_type = request.business_type.trim();

 if main_name.is_empty() {
 return Err("主体名不能为空".to_string());
 }

 if business_type.is_empty() {
 return Err("办理类型不能为空".to_string());
 }

 let stored = resolve_config(None)?;
 let pool = connect_pool(&stored).await?;
 let accounts = query_accounts(&pool, main_name, business_type).await?;
 let mut items = Vec::with_capacity(accounts.len());

 for account in accounts {
 let cert = query_latest_cert(&pool, &account.login_key).await?;
 items.push(CertQueryItem { account, cert });
 }

 pool.close().await;
 Ok(CertQueryResponse { items })
}

async fn query_accounts(
 pool: &MySqlPool,
 main_name: &str,
 business_type: &str,
) -> Result<Vec<CertQueryAccount>, String> {
 let rows = sqlx::query(
 r#"
 SELECT CAST(rwa.id AS CHAR) AS id,
 ra.main_name AS company_name,
 rwa.login_key,
 rw.blxm AS business_type,
 rwa.dwbh,
 CAST(ra.area_id AS CHAR) AS area_id
 FROM robot_website_account rwa
 LEFT JOIN robot_account ra ON rwa.account_id = ra.id
 LEFT JOIN robot_website rw ON rwa.website_id = rw.id
 WHERE ra.main_name = ?
 AND rw.blxm = ?
 "#,
 )
 .bind(main_name)
 .bind(business_type)
 .fetch_all(pool)
 .await
 .map_err(|error| format!("查询账号失败: {error}"))?;

 rows.into_iter()
 .map(|row| {
 let login_key: Option<String> = row
 .try_get("login_key")
 .map_err(|error| format!("读取 login_key 失败: {error}"))?;
 let login_key = login_key
 .map(|value| value.trim().to_string())
 .filter(|value| !value.is_empty())
 .ok_or_else(|| "账号记录缺少 login_key".to_string())?;

 Ok(CertQueryAccount {
 id: row.try_get("id").unwrap_or(None),
 company_name: row.try_get("company_name").unwrap_or(None),
 login_key,
 business_type: row.try_get("business_type").unwrap_or(None),
 dwbh: row.try_get("dwbh").unwrap_or(None),
 area_id: row.try_get("area_id").unwrap_or(None),
 })
 })
 .collect()
}

async fn query_latest_cert(
 pool: &MySqlPool,
 account_code: &str,
) -> Result<Option<CertQueryCert>, String> {
 let row = sqlx::query(
 r#"
 SELECT cert_info,
 CAST(update_time AS CHAR) AS update_time
 FROM assistant_cert
 WHERE account_code = ?
 AND valid = 1
 ORDER BY update_time DESC
 LIMIT 1
 "#,
 )
 .bind(account_code)
 .fetch_optional(pool)
 .await
 .map_err(|error| format!("查询 cert_info 失败: {error}"))?;

 row.map(|row| {
 let cert_info: Option<String> = row
 .try_get("cert_info")
 .map_err(|error| format!("读取 cert_info 失败: {error}"))?;
 Ok(CertQueryCert {
 cert_info: cert_info.unwrap_or_default(),
 update_time: row.try_get("update_time").unwrap_or(None),
 })
 })
 .transpose()
}

async fn connect_pool(config: &ResolvedMysqlDatasourceConfig) -> Result<MySqlPool, String> {
 let url = format!(
 "mysql://{}:{}@{}:{}/{}",
 percent_encode(&config.username),
 percent_encode(&config.password),
 config.host.trim(),
 config.port,
 percent_encode(&config.database)
 );

 MySqlPoolOptions::new()
 .max_connections(2)
 .acquire_timeout(Duration::from_secs(config.connect_timeout_seconds))
 .connect(&url)
 .await
 .map_err(|error| format!("连接 MySQL 失败: {error}"))
}

fn resolve_config(
 incoming: Option<MysqlDatasourceConfig>,
) -> Result<ResolvedMysqlDatasourceConfig, String> {
 match incoming {
 Some(config) => {
 validate_config(&config)?;
 Ok(ResolvedMysqlDatasourceConfig {
 host: config.host.trim().to_string(),
 port: config.port,
 database: config.database.trim().to_string(),
 username: config.username.trim().to_string(),
 password: config.password.unwrap_or_default(),
 connect_timeout_seconds: config.connect_timeout_seconds,
 })
 }
 None => {
 let stored =
 read_stored_config()?.ok_or_else(|| "请先配置 MySQL 数据源".to_string())?;
 let password = decrypt_stored_password(&stored)?;
 Ok(ResolvedMysqlDatasourceConfig {
 host: stored.host,
 port: stored.port,
 database: stored.database,
 username: stored.username,
 password,
 connect_timeout_seconds: stored.connect_timeout_seconds,
 })
 }
 }
}

fn validate_config(config: &MysqlDatasourceConfig) -> Result<(), String> {
 if config.host.trim().is_empty() {
 return Err("数据库地址不能为空".to_string());
 }

 if config.port == 0 {
 return Err("数据库端口不合法".to_string());
 }

 if config.database.trim().is_empty() {
 return Err("数据库名不能为空".to_string());
 }

 if config.username.trim().is_empty() {
 return Err("数据库账号不能为空".to_string());
 }

 if config.connect_timeout_seconds == 0 {
 return Err("连接超时时间必须大于 0".to_string());
 }

 Ok(())
}

fn read_stored_config() -> Result<Option<StoredMysqlDatasourceConfig>, String> {
 let path = config_path()?;

 if !path.is_file() {
 return Ok(None);
 }

 let content = fs::read_to_string(&path)
 .map_err(|error| format!("读取数据源配置失败 {}: {error}", path.display()))?;
 let stored: StoredMysqlDatasourceConfig =
 serde_json::from_str(&content).map_err(|error| format!("解析数据源配置失败: {error}"))?;

 Ok(Some(stored))
}

fn write_stored_config(config: &StoredMysqlDatasourceConfig) -> Result<(), String> {
 let path = config_path()?;
 let parent = path
 .parent()
 .ok_or_else(|| format!("数据源配置路径不合法: {}", path.display()))?;

 fs::create_dir_all(parent)
 .map_err(|error| format!("创建数据源配置目录失败 {}: {error}", parent.display()))?;

 let content = serde_json::to_string_pretty(config)
 .map_err(|error| format!("序列化数据源配置失败: {error}"))?;
 fs::write(&path, content)
 .map_err(|error| format!("保存数据源配置失败 {}: {error}", path.display()))
}

fn to_config_view(config: &StoredMysqlDatasourceConfig) -> MysqlDatasourceConfigView {
 MysqlDatasourceConfigView {
 host: config.host.clone(),
 port: config.port,
 database: config.database.clone(),
 username: config.username.clone(),
 has_password: !config.encrypted_password.is_empty(),
 connect_timeout_seconds: config.connect_timeout_seconds,
 }
}

fn config_path() -> Result<PathBuf, String> {
 let dirs = ProjectDirs::from("com", "coter", "CoterEncrypt")
 .ok_or_else(|| "无法定位应用配置目录".to_string())?;

 Ok(dirs.config_dir().join(CONFIG_FILE_NAME))
}

fn default_connect_timeout_seconds() -> u64 {
 8
}

fn percent_encode(input: &str) -> String {
 input
 .bytes()
 .flat_map(|byte| match byte {
 b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
 vec![byte as char]
 }
 _ => format!("%{byte:02X}").chars().collect(),
 })
 .collect()
}

fn encrypt_password(password: &str) -> Result<String, String> {
 let encrypted = crypt_protect(password.as_bytes())?;
 Ok(hex_encode(&encrypted))
}

fn decrypt_stored_password(config: &StoredMysqlDatasourceConfig) -> Result<String, String> {
 if config.encrypted_password.trim().is_empty() {
 return Ok(String::new());
 }

 let encrypted = hex_decode(&config.encrypted_password)?;
 let decrypted = crypt_unprotect(&encrypted)?;

 String::from_utf8(decrypted).map_err(|error| format!("解密数据库密码失败: {error}"))
}

fn crypt_protect(input: &[u8]) -> Result<Vec<u8>, String> {
 let mut input_blob = CRYPT_INTEGER_BLOB {
 cbData: input
 .len()
 .try_into()
 .map_err(|_| "数据库密码过长".to_string())?,
 pbData: input.as_ptr() as *mut u8,
 };
 let mut output_blob = CRYPT_INTEGER_BLOB::default();

 let ok = unsafe {
 CryptProtectData(
 &mut input_blob,
 std::ptr::null(),
 std::ptr::null(),
 std::ptr::null(),
 std::ptr::null(),
 CRYPTPROTECT_UI_FORBIDDEN,
 &mut output_blob,
 )
 };

 if ok == 0 {
 return Err("加密数据库密码失败".to_string());
 }

 blob_to_vec_and_free(output_blob)
}

fn crypt_unprotect(input: &[u8]) -> Result<Vec<u8>, String> {
 let mut input_blob = CRYPT_INTEGER_BLOB {
 cbData: input
 .len()
 .try_into()
 .map_err(|_| "已保存的数据库密码过长".to_string())?,
 pbData: input.as_ptr() as *mut u8,
 };
 let mut output_blob = CRYPT_INTEGER_BLOB::default();

 let ok = unsafe {
 CryptUnprotectData(
 &mut input_blob,
 std::ptr::null_mut(),
 std::ptr::null(),
 std::ptr::null(),
 std::ptr::null(),
 CRYPTPROTECT_UI_FORBIDDEN,
 &mut output_blob,
 )
 };

 if ok == 0 {
 return Err("解密数据库密码失败".to_string());
 }

 blob_to_vec_and_free(output_blob)
}

fn blob_to_vec_and_free(blob: CRYPT_INTEGER_BLOB) -> Result<Vec<u8>, String> {
 if blob.pbData.is_null() {
 return Ok(Vec::new());
 }

 let bytes = unsafe { std::slice::from_raw_parts(blob.pbData, blob.cbData as usize) }.to_vec();
 unsafe {
 LocalFree(blob.pbData.cast());
 }

 Ok(bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
 const HEX: &[u8; 16] = b"0123456789abcdef";
 let mut output = String::with_capacity(bytes.len() * 2);

 for byte in bytes {
 output.push(HEX[(byte >> 4) as usize] as char);
 output.push(HEX[(byte & 0x0f) as usize] as char);
 }

 output
}

fn hex_decode(input: &str) -> Result<Vec<u8>, String> {
 let trimmed = input.trim();
 if trimmed.len() % 2 != 0 {
 return Err("已保存的数据库密码格式不合法".to_string());
 }

 let mut bytes = Vec::with_capacity(trimmed.len() / 2);
 let chars = trimmed.as_bytes();

 for index in (0..chars.len()).step_by(2) {
 let high = hex_value(chars[index])?;
 let low = hex_value(chars[index + 1])?;
 bytes.push((high << 4) | low);
 }

 Ok(bytes)
}

fn hex_value(byte: u8) -> Result<u8, String> {
 match byte {
 b'0'..=b'9' => Ok(byte - b'0'),
 b'a'..=b'f' => Ok(byte - b'a' + 10),
 b'A'..=b'F' => Ok(byte - b'A' + 10),
 _ => Err("已保存的数据库密码格式不合法".to_string()),
 }
}

#[allow(dead_code)]
fn config_file_exists_in(path: &Path) -> bool {
 path.join(CONFIG_FILE_NAME).is_file()
}

#[cfg(test)]
mod tests {
 use super::{hex_decode, hex_encode, percent_encode, MysqlDatasourceConfig};

 #[test]
 fn percent_encode_encodes_url_reserved_characters() {
 assert_eq!(percent_encode("user:name@db"), "user%3Aname%40db");
 assert_eq!(percent_encode("abc-_.~123"), "abc-_.~123");
 }

 #[test]
 fn config_default_timeout_is_deserialized() {
 let config: MysqlDatasourceConfig = serde_json::from_str(
 r#"{
 "host": "127.0.0.1",
 "port": 3306,
 "database": "robot",
 "username": "root",
 "password": "secret"
 }"#,
 )
 .unwrap();

 assert_eq!(config.connect_timeout_seconds, 8);
 }

 #[test]
 fn hex_round_trips_bytes() {
 let bytes = [0, 1, 15, 16, 127, 255];
 let encoded = hex_encode(&bytes);
 assert_eq!(encoded, "00010f107fff");
 assert_eq!(hex_decode(&encoded).unwrap(), bytes);
 }
}
