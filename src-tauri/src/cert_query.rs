use std::{
 fs,
 path::{Path, PathBuf},
 time::Duration,
};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use sqlx::{mysql::MySqlPoolOptions, MySqlPool, Row};

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
 password: String,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotTaskFeedbackQueryRequest {
 pub task_id: String,
 #[serde(default)]
 pub schema_name: Option<String>,
 #[serde(default)]
 pub status_condition: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotTaskUserFeedbackRow {
 pub id: String,
 pub status: Option<String>,
 pub msg: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotTaskUserInsFeedbackRow {
 pub id: String,
 pub task_user_id: Option<String>,
 pub status: Option<String>,
 pub msg: Option<String>,
 pub feedback_ed: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobotTaskFeedbackQueryResponse {
 pub task_users: Vec<RobotTaskUserFeedbackRow>,
 pub task_user_ins: Vec<RobotTaskUserInsFeedbackRow>,
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
 .map(|p| p.password.clone())
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
 password,
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

pub async fn query_robot_task_feedback_data(
 request: RobotTaskFeedbackQueryRequest,
) -> Result<RobotTaskFeedbackQueryResponse, String> {
 let task_id = parse_task_id(&request.task_id)?;
 let schema_name = normalize_schema_name(request.schema_name.as_deref())?;
 let stored = resolve_config(None)?;
 let pool = connect_pool(&stored).await?;

 let status_condition = parse_optional_task_id(request.status_condition.as_deref())?;
 let task_users =
 query_robot_task_users(&pool, task_id, status_condition, schema_name.as_deref()).await?;
 let task_user_ins =
 query_robot_task_user_ins(&pool, task_id, status_condition, schema_name.as_deref())
 .await?;

 pool.close().await;
 Ok(RobotTaskFeedbackQueryResponse {
 task_users,
 task_user_ins,
 })
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
 JOIN robot_account ra ON rwa.account_id = ra.id
 JOIN robot_website rw ON rwa.website_id = rw.id
 WHERE ra.main_name = ?
 AND rw.blxm = ?
 AND rwa.valid = 1
 AND ra.status = 1
 AND rw.valid = 1
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

async fn query_robot_task_users(
 pool: &MySqlPool,
 task_id: i64,
 status_condition: Option<i64>,
 schema_name: Option<&str>,
) -> Result<Vec<RobotTaskUserFeedbackRow>, String> {
 let table = qualify_table_name(schema_name, "robot_task_user");
 let status_clause = if status_condition.is_some() {
 " AND status = ?"
 } else {
 ""
 };
 let sql = format!(
 r#"
 SELECT CAST(id AS CHAR) AS id,
 CAST(status AS CHAR) AS status,
 msg
 FROM {table}
 WHERE task_id = ?{status_clause}
 ORDER BY id
 "#
 );

 let mut query = sqlx::query(&sql).bind(task_id);
 if let Some(sc) = status_condition {
 query = query.bind(sc);
 }
 let rows = query
 .fetch_all(pool)
 .await
 .map_err(|error| format!("查询 robot_task_user 失败: {error}"))?;

 rows.into_iter()
 .map(|row| {
 let id: Option<String> = row
 .try_get("id")
 .map_err(|error| format!("读取 robot_task_user.id 失败: {error}"))?;
 let id = id
 .map(|value| value.trim().to_string())
 .filter(|value| !value.is_empty())
 .ok_or_else(|| "robot_task_user 记录缺少 id".to_string())?;

 Ok(RobotTaskUserFeedbackRow {
 id,
 status: row.try_get("status").unwrap_or(None),
 msg: row.try_get("msg").unwrap_or(None),
 })
 })
 .collect()
}

async fn query_robot_task_user_ins(
 pool: &MySqlPool,
 task_id: i64,
 status_condition: Option<i64>,
 schema_name: Option<&str>,
) -> Result<Vec<RobotTaskUserInsFeedbackRow>, String> {
 let table = qualify_table_name(schema_name, "robot_task_user_ins");
 let status_clause = if status_condition.is_some() {
 " AND status = ?"
 } else {
 ""
 };
 let sql = format!(
 r#"
 SELECT CAST(id AS CHAR) AS id,
 CAST(task_user_id AS CHAR) AS task_user_id,
 CAST(status AS CHAR) AS status,
 msg,
 CAST(feedback_ed AS CHAR) AS feedback_ed
 FROM {table}
 WHERE task_id = ?{status_clause}
 ORDER BY id
 "#
 );

 let mut query = sqlx::query(&sql).bind(task_id);
 if let Some(sc) = status_condition {
 query = query.bind(sc);
 }
 let rows = query
 .fetch_all(pool)
 .await
 .map_err(|error| format!("查询 robot_task_user_ins 失败: {error}"))?;

 rows.into_iter()
 .map(|row| {
 let id: Option<String> = row
 .try_get("id")
 .map_err(|error| format!("读取 robot_task_user_ins.id 失败: {error}"))?;
 let id = id
 .map(|value| value.trim().to_string())
 .filter(|value| !value.is_empty())
 .ok_or_else(|| "robot_task_user_ins 记录缺少 id".to_string())?;

 Ok(RobotTaskUserInsFeedbackRow {
 id,
 task_user_id: row.try_get("task_user_id").unwrap_or(None),
 status: row.try_get("status").unwrap_or(None),
 msg: row.try_get("msg").unwrap_or(None),
 feedback_ed: row.try_get("feedback_ed").unwrap_or(None),
 })
 })
 .collect()
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

fn parse_task_id(input: &str) -> Result<i64, String> {
 let trimmed = input.trim();

 if trimmed.is_empty() {
 return Err("task_id 不能为空".to_string());
 }

 if !trimmed.chars().all(|char| char.is_ascii_digit()) {
 return Err("task_id 只能包含数字".to_string());
 }

 let normalized = trimmed.trim_start_matches('0');
 let normalized = if normalized.is_empty() {
 "0"
 } else {
 normalized
 };
 let task_id = normalized
 .parse::<i64>()
 .map_err(|_| "task_id 超出支持范围".to_string())?;

 if task_id <= 0 {
 return Err("task_id 必须大于 0".to_string());
 }

 Ok(task_id)
}

fn parse_optional_task_id(value: Option<&str>) -> Result<Option<i64>, String> {
 match value {
 Some(input) if !input.trim().is_empty() => parse_task_id(input).map(Some),
 _ => Ok(None),
 }
}
fn normalize_schema_name(input: Option<&str>) -> Result<Option<String>, String> {
 let Some(input) = input else {
 return Ok(None);
 };

 let trimmed = input.trim();
 if trimmed.is_empty() {
 return Ok(None);
 }

 if !trimmed
 .chars()
 .all(|char| char.is_ascii_alphanumeric() || char == '_')
 {
 return Err("库名只能包含字母、数字和下划线".to_string());
 }

 Ok(Some(trimmed.to_string()))
}

fn qualify_table_name(schema_name: Option<&str>, table_name: &str) -> String {
 match schema_name {
 Some(schema_name) => format!("`{schema_name}`.`{table_name}`"),
 None => format!("`{table_name}`"),
 }
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
 Ok(ResolvedMysqlDatasourceConfig {
 host: stored.host,
 port: stored.port,
 database: stored.database,
 username: stored.username,
 password: stored.password,
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
 has_password: !config.password.is_empty(),
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

#[allow(dead_code)]
fn config_file_exists_in(path: &Path) -> bool {
 path.join(CONFIG_FILE_NAME).is_file()
}

#[cfg(test)]
mod tests {
 use super::{
 normalize_schema_name, parse_task_id, percent_encode, qualify_table_name,
 MysqlDatasourceConfig,
 };

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
 fn robot_task_query_helpers_validate_input() {
 assert_eq!(parse_task_id("0209425").unwrap(), 209425);
 assert!(parse_task_id("abc").is_err());
 assert!(parse_task_id("0").is_err());
 assert_eq!(
 normalize_schema_name(Some("platform_crawler")).unwrap(),
 Some("platform_crawler".to_string())
 );
 assert!(normalize_schema_name(Some("platform-crawler")).is_err());
 assert_eq!(
 qualify_table_name(Some("platform_crawler"), "robot_task_user"),
 "`platform_crawler`.`robot_task_user`"
 );
 }
}
