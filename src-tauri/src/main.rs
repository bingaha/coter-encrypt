#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod browser_bridge;
mod cert_query;
mod crypto;
mod executor;
mod har;
mod oss_transfer;
mod project_store;

use coter_core::removed_module::{Request, Response};
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;
use url::Url;

#[tauri::command]
fn ping() -> &'static str {
 "pong"
}

#[tauri::command]
fn list_projects() -> Result<Vec<project_store::Project>, String> {
 project_store::list_projects()
}

#[tauri::command]
fn get_project_by_id(id: u64) -> Result<Option<project_store::Project>, String> {
 project_store::get_project_by_id(id)
}

#[tauri::command]
fn get_project_by_name(name: String) -> Result<Option<project_store::Project>, String> {
 project_store::get_project_by_name(&name)
}

#[tauri::command]
fn save_project(project: project_store::Project) -> Result<project_store::Project, String> {
 project_store::save_project(project)
}

#[tauri::command]
fn update_project(
 project: project_store::Project,
) -> Result<Option<project_store::Project>, String> {
 project_store::update_project(project)
}

#[tauri::command]
fn delete_project(id: u64) -> Result<bool, String> {
 project_store::delete_project(id)
}

#[tauri::command]
fn rename_project(id: u64, new_name: String) -> Result<Option<project_store::Project>, String> {
 project_store::rename_project(id, new_name)
}

#[tauri::command]
fn removed_command(
 payload: Request,
) -> Result<Response, String> {
 coter_core::removed_module::removed_command(payload)
}

#[tauri::command]
fn execute_batch(request: executor::BatchExecutionRequest) -> Vec<executor::EncryptionResponse> {
 executor::execute_batch(request)
}

#[tauri::command]
fn open_external_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
 let parsed = Url::parse(&url).map_err(|error| format!("URL 格式错误: {error}"))?;
 let is_allowed_url = parsed.scheme() == "https"
 && parsed.domain() == Some("console.cloud.tencent.com")
 && parsed.path() == "/cls/search";

 if !is_allowed_url {
 return Err("只允许打开腾讯云 CLS 查询链接".to_string());
 }

 app.opener()
 .open_url(url, None::<&str>)
 .map_err(|error| format!("打开浏览器失败: {error}"))
}

#[tauri::command]
async fn load_mysql_datasource_config(
) -> Result<Option<cert_query::MysqlDatasourceConfigView>, String> {
 cert_query::load_mysql_datasource_config().await
}

#[tauri::command]
async fn save_mysql_datasource_config(
 config: cert_query::MysqlDatasourceConfig,
) -> Result<cert_query::MysqlDatasourceConfigView, String> {
 cert_query::save_mysql_datasource_config(config).await
}

#[tauri::command]
async fn test_mysql_datasource(
 config: Option<cert_query::MysqlDatasourceConfig>,
) -> Result<(), String> {
 cert_query::test_mysql_datasource(config).await
}

#[tauri::command]
async fn query_cert_info(
 request: cert_query::CertQueryRequest,
) -> Result<cert_query::CertQueryResponse, String> {
 cert_query::query_cert_info(request).await
}

#[tauri::command]
async fn query_robot_task_feedback_data(
 request: cert_query::RobotTaskFeedbackQueryRequest,
) -> Result<cert_query::RobotTaskFeedbackQueryResponse, String> {
 cert_query::query_robot_task_feedback_data(request).await
}

#[tauri::command]
fn load_browser_bridge_config() -> Result<browser_bridge::BrowserBridgeConfig, String> {
 browser_bridge::load_browser_bridge_config()
}

#[tauri::command]
fn save_browser_bridge_config(
 config: browser_bridge::BrowserBridgeConfig,
) -> Result<browser_bridge::BrowserBridgeConfig, String> {
 browser_bridge::save_browser_bridge_config(config)
}

#[tauri::command]
fn load_website_url_mappings() -> Result<Vec<browser_bridge::WebsiteUrlMapping>, String> {
 browser_bridge::load_website_url_mappings()
}

#[tauri::command]
fn save_website_url_mapping(
 mapping: browser_bridge::WebsiteUrlMapping,
) -> Result<Vec<browser_bridge::WebsiteUrlMapping>, String> {
 browser_bridge::save_website_url_mapping(mapping)
}

#[tauri::command]
fn open_app_config_dir(app: tauri::AppHandle) -> Result<(), String> {
 browser_bridge::open_app_config_dir(app)
}

#[tauri::command]
async fn open_default_browser_with_cookies(
 app: tauri::AppHandle,
 request: browser_bridge::OpenWithCookiesRequest,
) -> Result<browser_bridge::OpenWithCookiesResponse, String> {
 browser_bridge::open_default_browser_with_cookies(app, request).await
}

#[cfg(test)]
mod tests {
 use serde_json::json;

 use super::{removed_command, Request};

 #[test]
 fn removed_command_command_returns_frontend_shape() {
 let response = removed_command(Request {
 project_name: "项目A".to_string(),
 config: json!({
 "inputMappings": [
 {
 "id": "input-1",
 "name": "原文",
 "inputRef": "plain",
 "defaultValue": "abc"
 }
 ],
 "components": [
 {
 "id": "component-1",
 "type": "BASE64",
 "outputRef": "out",
 "config": {
 "inputSourceType": "inputMapping",
 "inputMappingRef": "plain",
 "operation": "encode"
 }
 }
 ],
 "outputMappings": [
 {
 "id": "output-1",
 "name": "result",
 "componentRef": "out"
 }
 ]
 }),
 })
 .unwrap();

 assert_eq!(response.project_name, "项目A");
 assert!(response
 .code
 .contains("String result = CryptoUtil.processBase64(plain, \"encode\");"));
 }
}

#[tauri::command]
async fn transfer_oss_key(
 oss_key: String,
 direction: String,
) -> Result<oss_transfer::OssTransferResult, String> {
 oss_transfer::transfer_oss_key(&oss_key, &direction).await
}

#[tauri::command]
fn load_oss_transfer_history() -> Result<Vec<oss_transfer::TransferRecord>, String> {
 oss_transfer::load_transfer_history()
}

#[tauri::command]
fn delete_oss_transfer_record(id: String) -> Result<Vec<oss_transfer::TransferRecord>, String> {
 oss_transfer::delete_transfer_record(&id)
}

#[tauri::command]
fn clear_oss_transfer_history() -> Result<(), String> {
 oss_transfer::clear_transfer_history()
}

#[tauri::command]
fn process_har(request: har::HarProcessRequest) -> Result<har::HarProcessResponse, String> {
 har::process_har(request)
}

fn main() {
 tauri::Builder::default()
 .plugin(tauri_plugin_dialog::init())
 .plugin(tauri_plugin_opener::init())
 .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
 if let Some(window) = app.get_webview_window("main") {
 let _ = window.set_focus();
 }
 }))
 .invoke_handler(tauri::generate_handler![
 ping,
 list_projects,
 get_project_by_id,
 get_project_by_name,
 save_project,
 update_project,
 delete_project,
 rename_project,
 removed_command,
 execute_batch,
 process_har,
 open_external_url,
 load_mysql_datasource_config,
 save_mysql_datasource_config,
 test_mysql_datasource,
 query_cert_info,
 query_robot_task_feedback_data,
 load_browser_bridge_config,
 save_browser_bridge_config,
 load_website_url_mappings,
 save_website_url_mapping,
 open_app_config_dir,
 open_default_browser_with_cookies,
 transfer_oss_key,
 load_oss_transfer_history,
 delete_oss_transfer_record,
 clear_oss_transfer_history
 ])
 .run(tauri::generate_context!())
 .expect("error while running CoterEncrypt");
}
