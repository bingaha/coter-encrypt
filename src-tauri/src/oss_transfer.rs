use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::http_client::{self, HttpProxyConfig};

const PROD_QUERY_URL: &str =
 "https://gateway.shebaotong.com/platform-oss/oss/queryFile?ossKey=";
const TEST_QUERY_URL: &str =
 "https://gwdaily.shebaotong.com/platform-oss/oss/queryFile?ossKey=";
const PROD_UPLOAD_URL: &str =
 "https://gateway.shebaotong.com/platform-oss/oss/uploadFile";
const TEST_UPLOAD_URL: &str =
 "https://gwdaily.shebaotong.com/platform-oss/oss/uploadFile";

const HISTORY_FILE_NAME: &str = "oss-transfer-history.json";

// ---------------------------------------------------------------------------
// Transfer history persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRecord {
 pub id: String,
 pub timestamp: String,
 pub oss_key: String,
 pub direction: String,
 pub new_oss_key: String,
 pub file_name: String,
 pub content_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TransferHistory {
 records: Vec<TransferRecord>,
}

fn history_path() -> Result<PathBuf, String> {
 let dirs = ProjectDirs::from("com", "coter", "CoterEncrypt")
 .ok_or_else(|| "无法确定应用配置目录".to_string())?;
 Ok(dirs.config_dir().join(HISTORY_FILE_NAME))
}

pub fn load_transfer_history() -> Result<Vec<TransferRecord>, String> {
 let path = history_path()?;
 if !path.is_file() {
 return Ok(Vec::new());
 }
 let content =
 fs::read_to_string(&path).map_err(|e| format!("读取转换历史失败: {e}"))?;
 let history: TransferHistory =
 serde_json::from_str(&content).map_err(|e| format!("解析转换历史失败: {e}"))?;
 Ok(history.records)
}

fn save_transfer_history(records: &[TransferRecord]) -> Result<(), String> {
 let path = history_path()?;
 let parent = path
 .parent()
 .ok_or_else(|| format!("配置路径不合法: {}", path.display()))?;
 fs::create_dir_all(parent)
 .map_err(|e| format!("创建配置目录失败 {}: {e}", parent.display()))?;
 let content = serde_json::to_string_pretty(&TransferHistory {
 records: records.to_vec(),
 })
 .map_err(|e| format!("序列化转换历史失败: {e}"))?;
 fs::write(&path, content)
 .map_err(|e| format!("写入转换历史失败 {}: {e}", path.display()))
}

pub fn delete_transfer_record(id: &str) -> Result<Vec<TransferRecord>, String> {
 let mut records = load_transfer_history()?;
 records.retain(|r| r.id != id);
 save_transfer_history(&records)?;
 Ok(records)
}

pub fn clear_transfer_history() -> Result<(), String> {
 save_transfer_history(&[])
}

fn append_transfer_record(record: TransferRecord) -> Result<(), String> {
 let mut records = load_transfer_history()?;
 records.push(record);
 save_transfer_history(&records)
}

// ---------------------------------------------------------------------------
// OSS transfer logic
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct QueryApiResponse {
 code: u32,
 data: Option<QueryFileData>,
 message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueryFileData {
 #[serde(rename = "outerUrl")]
 outer_url: String,
 #[serde(rename = "contentType")]
 content_type: String,
 #[serde(rename = "fileName")]
 file_name: String,
}

#[derive(Debug, Deserialize)]
struct UploadApiResponse {
 code: u32,
 data: Option<UploadFileData>,
 message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadFileData {
 #[serde(rename = "ossKey")]
 oss_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OssTransferResult {
 pub new_oss_key: String,
 pub file_name: String,
 pub content_type: String,
}

/// Transfer an OSS file between production and test environments.
///
/// `direction` must be `"prod_to_test"` or `"test_to_prod"`.
pub async fn transfer_oss_key(
 oss_key: &str,
 direction: &str,
 proxy_config: &HttpProxyConfig,
) -> Result<OssTransferResult, String> {
 let (query_url, upload_url) = match direction {
 "prod_to_test" => (PROD_QUERY_URL, TEST_UPLOAD_URL),
 "test_to_prod" => (TEST_QUERY_URL, PROD_UPLOAD_URL),
 _ => return Err("无效的转换方向".to_string()),
 };

 let client = http_client::build_http_client(std::time::Duration::from_secs(60), proxy_config)?;

 // 1. Query source environment
 let query_full_url = format!("{query_url}{oss_key}");
 let query_resp = client
 .get(&query_full_url)
 .send()
 .await
 .map_err(|e| format!("查询源环境失败: {e}"))?;

 let query_body: QueryApiResponse = query_resp
 .json()
 .await
 .map_err(|e| format!("解析查询响应失败: {e}"))?;

 if query_body.code != 200 {
 return Err(format!(
 "查询失败: {}",
 query_body.message.unwrap_or_default()
 ));
 }

 let file_data = query_body
 .data
 .ok_or_else(|| "查询响应缺少文件信息".to_string())?;

 // 2. Download file from outerUrl
 let file_resp = client
 .get(&file_data.outer_url)
 .send()
 .await
 .map_err(|e| format!("下载文件失败: {e}"))?;

 if !file_resp.status().is_success() {
 return Err(format!(
 "下载文件失败，HTTP 状态码: {}",
 file_resp.status()
 ));
 }

 let file_bytes = file_resp
 .bytes()
 .await
 .map_err(|e| format!("读取文件内容失败: {e}"))?;

 // 3. Upload file to target environment
 let file_part = reqwest::multipart::Part::bytes(file_bytes.to_vec())
 .file_name(file_data.file_name.clone())
 .mime_str(&file_data.content_type)
 .map_err(|e| format!("构建文件分段失败: {e}"))?;

 let form = reqwest::multipart::Form::new()
 .text("fileType", "15")
 .part("file", file_part);

 let upload_resp = client
 .post(upload_url)
 .multipart(form)
 .send()
 .await
 .map_err(|e| format!("上传文件失败: {e}"))?;

 let upload_body: UploadApiResponse = upload_resp
 .json()
 .await
 .map_err(|e| format!("解析上传响应失败: {e}"))?;

 if upload_body.code != 200 {
 return Err(format!(
 "上传失败: {}",
 upload_body.message.unwrap_or_default()
 ));
 }

 let upload_data = upload_body
 .data
 .ok_or_else(|| "上传响应缺少数据".to_string())?;

 let result = OssTransferResult {
 new_oss_key: upload_data.oss_key,
 file_name: file_data.file_name,
 content_type: file_data.content_type,
 };

 // 4. Save transfer record
 let record = TransferRecord {
 id: uuid_v4(),
 timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
 oss_key: oss_key.to_string(),
 direction: direction.to_string(),
 new_oss_key: result.new_oss_key.clone(),
 file_name: result.file_name.clone(),
 content_type: result.content_type.clone(),
 };
 let _ = append_transfer_record(record);

 Ok(result)
}

fn uuid_v4() -> String {
 use rand::Rng;
 let mut rng = rand::thread_rng();
 let bytes: [u8; 16] = rng.gen();
 let mut uuid = bytes;
 uuid[6] = (uuid[6] & 0x0f) | 0x40;
 uuid[8] = (uuid[8] & 0x3f) | 0x80;
 format!(
 "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
 uuid[0], uuid[1], uuid[2], uuid[3],
 uuid[4], uuid[5],
 uuid[6], uuid[7],
 uuid[8], uuid[9],
 uuid[10], uuid[11], uuid[12], uuid[13], uuid[14], uuid[15]
 )
}
