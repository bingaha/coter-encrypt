#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod browser_bridge;
mod cert_query;
mod crypto;
mod executor;
mod har;
mod http_client;
mod merge_monitor;
mod oss_transfer;
mod pipeline_monitor;
mod project_store;
mod system_notify;

use std::time::Duration;

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
fn execute_batch(request: executor::BatchExecutionRequest) -> Vec<executor::EncryptionResponse> {
    executor::execute_batch(request)
}

#[tauri::command]
fn open_external_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let parsed = Url::parse(&url).map_err(|error| format!("URL 格式错误: {error}"))?;
    let domain = parsed.domain().unwrap_or_default();
    let path = parsed.path();
    let is_allowed_url = parsed.scheme() == "https"
        && ((domain == "console.cloud.tencent.com" && path == "/cls/search")
            || (domain == "account-devops.aliyun.com"
                && matches!(
                    path,
                    "/settings/personalAccessToken" | "/settings/joinedOrganizations"
                )));

    if !is_allowed_url {
        return Err("当前链接不在允许打开的白名单中".to_string());
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

#[tauri::command]
async fn transfer_oss_key(
    oss_key: String,
    direction: String,
    proxy_state: tauri::State<'_, http_client::HttpProxyState>,
) -> Result<oss_transfer::OssTransferResult, String> {
    let proxy = proxy_state.get();
    oss_transfer::transfer_oss_key(&oss_key, &direction, &proxy).await
}

#[tauri::command]
fn load_http_proxy_config(
    proxy_state: tauri::State<'_, http_client::HttpProxyState>,
) -> Result<http_client::HttpProxyConfig, String> {
    Ok(proxy_state.get())
}

#[tauri::command]
fn save_http_proxy_config(
    config: http_client::HttpProxyConfig,
    proxy_state: tauri::State<'_, http_client::HttpProxyState>,
    monitor_state: tauri::State<'_, pipeline_monitor::MonitorState>,
    merge_state: tauri::State<'_, merge_monitor::MergeMonitorState>,
) -> Result<http_client::HttpProxyConfig, String> {
    let config = http_client::normalize_config(config)?;
    http_client::save_http_proxy_config_to_disk(&config)?;
    proxy_state.set(config.clone());
    let client = http_client::build_http_client(Duration::from_secs(20), &config)?;
    monitor_state.replace_http_client(client.clone());
    merge_state.replace_http_client(client);
    Ok(config)
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

#[tauri::command]
async fn load_pipeline_monitor_config(
    state: tauri::State<'_, pipeline_monitor::MonitorState>,
) -> Result<pipeline_monitor::PipelineMonitorConfig, String> {
    pipeline_monitor::load_pipeline_monitor_config(state).await
}

#[tauri::command]
async fn save_pipeline_monitor_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, pipeline_monitor::MonitorState>,
    config: pipeline_monitor::PipelineMonitorConfig,
) -> Result<pipeline_monitor::PipelineMonitorConfig, String> {
    pipeline_monitor::save_pipeline_monitor_config(app, state, config).await
}

#[tauri::command]
async fn start_pipeline_monitor(
    app: tauri::AppHandle,
    state: tauri::State<'_, pipeline_monitor::MonitorState>,
) -> Result<pipeline_monitor::MonitorSnapshot, String> {
    pipeline_monitor::start_pipeline_monitor(app, state).await
}

#[tauri::command]
async fn start_pipeline_monitor_single(
    app: tauri::AppHandle,
    state: tauri::State<'_, pipeline_monitor::MonitorState>,
    request: pipeline_monitor::SingleMonitorRequest,
) -> Result<pipeline_monitor::MonitorSnapshot, String> {
    pipeline_monitor::start_pipeline_monitor_single(app, state, request).await
}

#[tauri::command]
async fn stop_pipeline_monitor(
    app: tauri::AppHandle,
    state: tauri::State<'_, pipeline_monitor::MonitorState>,
) -> Result<pipeline_monitor::MonitorSnapshot, String> {
    pipeline_monitor::stop_pipeline_monitor(app, state).await
}

#[tauri::command]
async fn query_pipeline_latest_run(
    state: tauri::State<'_, pipeline_monitor::MonitorState>,
    pipeline_id: String,
) -> Result<pipeline_monitor::LatestRunInfo, String> {
    pipeline_monitor::query_pipeline_latest_run(state, pipeline_id).await
}

#[tauri::command]
async fn get_pipeline_monitor_snapshot(
    state: tauri::State<'_, pipeline_monitor::MonitorState>,
) -> Result<pipeline_monitor::MonitorSnapshot, String> {
    pipeline_monitor::get_pipeline_monitor_snapshot(state).await
}

#[tauri::command]
async fn respond_pipeline_monitor_action(
    app: tauri::AppHandle,
    state: tauri::State<'_, pipeline_monitor::MonitorState>,
    request: pipeline_monitor::ActionRequest,
) -> Result<pipeline_monitor::MonitorSnapshot, String> {
    pipeline_monitor::respond_pipeline_monitor_action(app, state, request).await
}

#[tauri::command]
async fn clear_pipeline_monitor_logs(
    app: tauri::AppHandle,
    state: tauri::State<'_, pipeline_monitor::MonitorState>,
) -> Result<pipeline_monitor::MonitorSnapshot, String> {
    pipeline_monitor::clear_pipeline_monitor_logs(app, state).await
}

#[tauri::command]
fn open_pipeline_run_page(
    app: tauri::AppHandle,
    pipeline_id: String,
    run_id: String,
) -> Result<(), String> {
    pipeline_monitor::open_pipeline_run_page(app, pipeline_id, run_id)
}

#[tauri::command]
async fn load_merge_monitor_config(
    state: tauri::State<'_, merge_monitor::MergeMonitorState>,
) -> Result<merge_monitor::MergeMonitorConfig, String> {
    merge_monitor::load_merge_monitor_config(state).await
}

#[tauri::command]
async fn save_merge_monitor_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, merge_monitor::MergeMonitorState>,
    config: merge_monitor::MergeMonitorConfig,
) -> Result<merge_monitor::MergeMonitorConfig, String> {
    merge_monitor::save_merge_monitor_config(app, state, config).await
}

#[tauri::command]
async fn get_merge_monitor_snapshot(
    state: tauri::State<'_, merge_monitor::MergeMonitorState>,
) -> Result<merge_monitor::MergeSnapshot, String> {
    merge_monitor::get_merge_monitor_snapshot(state).await
}

#[tauri::command]
async fn clear_merge_monitor_logs(
    app: tauri::AppHandle,
    state: tauri::State<'_, merge_monitor::MergeMonitorState>,
) -> Result<merge_monitor::MergeSnapshot, String> {
    merge_monitor::clear_merge_monitor_logs(app, state).await
}

#[tauri::command]
async fn start_merge_monitor(
    app: tauri::AppHandle,
    state: tauri::State<'_, merge_monitor::MergeMonitorState>,
) -> Result<merge_monitor::MergeSnapshot, String> {
    merge_monitor::start_merge_monitor(app, state).await
}

#[tauri::command]
async fn stop_merge_monitor(
    app: tauri::AppHandle,
    state: tauri::State<'_, merge_monitor::MergeMonitorState>,
) -> Result<merge_monitor::MergeSnapshot, String> {
    merge_monitor::stop_merge_monitor(app, state).await
}

#[tauri::command]
fn open_merge_request_page(
    app: tauri::AppHandle,
    detail_url: String,
) -> Result<(), String> {
    merge_monitor::open_merge_request_page(app, detail_url)
}

#[tauri::command]
async fn list_merge_monitor_repositories(
    state: tauri::State<'_, merge_monitor::MergeMonitorState>,
    request: merge_monitor::ListRemoteRepositoriesRequest,
) -> Result<Vec<merge_monitor::RemoteRepositoryItem>, String> {
    merge_monitor::list_merge_monitor_repositories(state, request).await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .manage(http_client::create_state())
        .manage(pipeline_monitor::create_state())
        .manage(merge_monitor::create_state())
        .setup(|app| {
            let version = env!("CARGO_PKG_VERSION");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title(&format!("加解密工具 v{version}"));
            }
            pipeline_monitor::spawn_background(app.handle().clone());
            merge_monitor::spawn_background(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            list_projects,
            get_project_by_id,
            get_project_by_name,
            save_project,
            update_project,
            delete_project,
            rename_project,
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
            clear_oss_transfer_history,
            load_http_proxy_config,
            save_http_proxy_config,
            load_pipeline_monitor_config,
            save_pipeline_monitor_config,
            start_pipeline_monitor,
            start_pipeline_monitor_single,
            stop_pipeline_monitor,
            query_pipeline_latest_run,
            get_pipeline_monitor_snapshot,
            respond_pipeline_monitor_action,
            clear_pipeline_monitor_logs,
            open_pipeline_run_page,
            load_merge_monitor_config,
            save_merge_monitor_config,
            start_merge_monitor,
            stop_merge_monitor,
            get_merge_monitor_snapshot,
            clear_merge_monitor_logs,
            open_merge_request_page,
            list_merge_monitor_repositories
        ])
        .run(tauri::generate_context!())
        .expect("error while running CoterEncrypt");
}
