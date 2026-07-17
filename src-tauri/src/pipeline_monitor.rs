use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use directories::ProjectDirs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

const CONFIG_FILE_NAME: &str = "yunxiao-pipeline.json";
const STATE_EVENT: &str = "pipeline-monitor-state";
const MAX_LOGS: usize = 200;
const OPENAPI_BASE: &str = "https://openapi-rdc.aliyuncs.com";
const PIPELINE_PAGE_BASE: &str = "https://flow.aliyun.com/pipelines";

const TOKEN_HINT: &str = "请先配置云效 Token";
const ORG_HINT: &str = "请先配置组织 ID";
const BRANCH_HINT: &str = "请先配置跟踪分支";
const PIPELINE_LIST_HINT: &str = "请先配置流水线列表";
const ENABLED_HINT: &str = "请至少启用一条流水线";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineConfigItem {
    pub name: String,
    pub pipeline_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineMonitorConfig {
    pub token: String,
    pub org_id: String,
    pub poll_interval_secs: u64,
    pub idle_latest_query_interval_secs: u64,
    pub post_action_refresh_delay_secs: u64,
    pub tracked_source_branch: String,
    pub allowed_trigger_users: Vec<String>,
    pub pipelines: Vec<PipelineConfigItem>,
    #[serde(default)]
    pub auto_mode: bool,
}

impl Default for PipelineMonitorConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            org_id: String::new(),
            poll_interval_secs: 30,
            idle_latest_query_interval_secs: 300,
            post_action_refresh_delay_secs: 5,
            tracked_source_branch: String::new(),
            allowed_trigger_users: Vec::new(),
            pipelines: Vec::new(),
            auto_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCandidateView {
    pub label: String,
    pub job_id: String,
    pub job_name: String,
    pub stage_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentPending {
    pub id: String,
    pub kind: String,
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub run_id: String,
    pub stage_name: String,
    pub job_id: String,
    pub job_name: String,
    pub job_status: String,
    pub stage_status: String,
    pub can_approve: bool,
    pub candidates: Vec<ExecuteCandidateView>,
    pub discovered_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStatusView {
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub enabled: bool,
    pub current_run_id: String,
    pub current_run_status: String,
    pub current_run_status_text: String,
    pub trigger_user: String,
    pub summary: String,
    pub last_seen_run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    pub running: bool,
    /// idle | loop | single
    pub mode: String,
    pub single_pipeline_id: String,
    pub auto_mode: bool,
    pub pending_count: u32,
    pub current_pending: Option<CurrentPending>,
    pub pipelines: Vec<PipelineStatusView>,
    pub logs: Vec<LogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestRunInfo {
    pub pipeline_id: String,
    pub run_id: String,
    pub status: String,
    pub status_text: String,
    pub trigger_text: String,
    pub trigger_user: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingleMonitorRequest {
    pub pipeline_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRequest {
    pub action: String,
    #[serde(default)]
    pub pending_id: String,
    #[serde(default)]
    pub job_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MonitorMode {
    Idle,
    Loop,
    Single,
}

impl MonitorMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Loop => "loop",
            Self::Single => "single",
        }
    }
}

#[derive(Debug, Clone)]
struct OpenApiResult {
    success: bool,
    data: Option<Value>,
    error_message: String,
    #[allow(dead_code)]
    status_code: Option<u16>,
}

#[derive(Debug, Clone)]
struct PipelineRuntimeState {
    pipeline_id: String,
    pipeline_name: String,
    current_run_id: String,
    current_run_status: String,
    last_seen_run_id: String,
    next_latest_query_time: u64,
    checkpoint_ack: HashMap<String, bool>,
    trigger_user: String,
    summary: String,
}

#[derive(Debug)]
struct MonitorRuntime {
    config: PipelineMonitorConfig,
    running: bool,
    mode: MonitorMode,
    single_pipeline_id: String,
    pipeline_states: HashMap<String, PipelineRuntimeState>,
    current_pending: Option<CurrentPending>,
    logs: VecDeque<LogEntry>,
    last_notified_pending_id: String,
}

pub struct MonitorState {
    inner: Mutex<MonitorRuntime>,
    pub loop_started: AtomicBool,
    pub http: Client,
}


fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn now_clock() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

fn config_path() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from("com", "coter", "CoterEncrypt")
        .ok_or_else(|| "Cannot resolve app config dir".to_string())?;
    Ok(dirs.config_dir().join(CONFIG_FILE_NAME))
}

fn ensure_config_dir(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Create config dir failed: {e}"))?;
    }
    Ok(())
}

fn load_config_from_disk() -> Result<PipelineMonitorConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        let config = PipelineMonitorConfig::default();
        save_config_to_disk(&config)?;
        return Ok(config);
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Read config failed: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Parse config failed: {e}"))
}

fn save_config_to_disk(config: &PipelineMonitorConfig) -> Result<(), String> {
    let path = config_path()?;
    ensure_config_dir(&path)?;
    let content =
        serde_json::to_string_pretty(config).map_err(|e| format!("Serialize config failed: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("Write config failed: {e}"))
}

fn append_log(runtime: &mut MonitorRuntime, level: &str, message: impl Into<String>) {
    runtime.logs.push_back(LogEntry {
        timestamp: now_clock(),
        level: level.to_string(),
        message: message.into(),
    });
    while runtime.logs.len() > MAX_LOGS {
        runtime.logs.pop_front();
    }
}

fn value_as_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn format_duration_text(total_seconds: i64) -> String {
    if total_seconds < 0 {
        return String::new();
    }
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}小时{minutes}分{seconds}秒")
    } else if minutes > 0 {
        format!("{minutes}分{seconds}秒")
    } else {
        format!("{seconds}秒")
    }
}

fn resolve_trigger_text(trigger_mode: &Value) -> String {
    let value = value_to_i64(trigger_mode).unwrap_or(0);
    match value {
        1 => "手动".to_string(),
        2 => "定时".to_string(),
        3 => "代码推送".to_string(),
        _ => "其他".to_string(),
    }
}

fn resolve_status_text(run_status: &str) -> String {
    match run_status {
        "RUNNING" => "运行中".to_string(),
        "SUCCESS" => "成功".to_string(),
        "FAIL" => "失败".to_string(),
        "CANCELED" => "已取消".to_string(),
        "WAITING" => "等待中".to_string(),
        "SWITCH_MANUAL" => "等待人工".to_string(),
        other => other.to_string(),
    }
}

fn resolve_run_finished(run_status: &str) -> bool {
    // 与 pipeline-monitor.py.bak 一致：不在运行/等待中即视为结束
    let status = run_status.trim();
    !status.is_empty() && !matches!(status, "RUNNING" | "SWITCH_MANUAL" | "WAITING")
}

/// 云效部分运行顶层 status 会长期停在 RUNNING，但「合并代码」成功即表示业务已结束。
fn resolve_merge_code_completed(response_data: &Value) -> bool {
    for (_stage_info, job) in extract_stage_rows(response_data) {
        let job_name = value_as_str(job.get("name").unwrap_or(&Value::Null));
        let job_status = value_as_str(job.get("status").unwrap_or(&Value::Null));
        if job_name == "合并代码" && job_status == "SUCCESS" {
            return true;
        }
    }
    false
}

fn resolve_effective_run_status(response_data: &Value) -> String {
    let status = {
        let s = value_as_str(response_data.get("status").unwrap_or(&Value::Null));
        if s.is_empty() {
            "UNKNOWN".to_string()
        } else {
            s
        }
    };
    if resolve_run_finished(&status) {
        return status;
    }
    if resolve_merge_code_completed(response_data) {
        return "SUCCESS".to_string();
    }
    status
}

fn build_execute_button_text(stage_name: &str, job_name: &str) -> String {
    format!("{stage_name} / {job_name}")
}

fn resolve_auto_execute_candidate(
    candidates: &[ExecuteCandidateView],
) -> Option<ExecuteCandidateView> {
    for c in candidates {
        if c.job_name == "人工卡点" {
            return Some(c.clone());
        }
    }
    for c in candidates {
        if c.label.contains("人工卡点") {
            return Some(c.clone());
        }
    }
    None
}

fn resolve_prod_publish_started(stage_name: &str, job_name: &str, job_status: &str) -> bool {
    if stage_name != "发布Daily" {
        return false;
    }
    if job_name != "发布Pro" && job_name != "发布Prod" {
        return false;
    }
    !matches!(job_status, "INIT" | "WAITING" | "SWITCH_MANUAL")
}

fn clear_current_run(state: &mut PipelineRuntimeState, next_query_time: u64) {
    state.current_run_id.clear();
    state.current_run_status.clear();
    state.checkpoint_ack.clear();
    state.trigger_user.clear();
    state.summary.clear();
    state.next_latest_query_time = next_query_time;
}

fn reset_monitor_runtime(runtime: &mut MonitorRuntime) {
    runtime.running = false;
    runtime.mode = MonitorMode::Idle;
    runtime.single_pipeline_id.clear();
    runtime.current_pending = None;
    runtime.last_notified_pending_id.clear();
    for state_item in runtime.pipeline_states.values_mut() {
        state_item.current_run_id.clear();
        state_item.current_run_status.clear();
        state_item.summary.clear();
        state_item.trigger_user.clear();
        state_item.checkpoint_ack.clear();
        state_item.next_latest_query_time = 0;
    }
}

fn finish_single_monitor(runtime: &mut MonitorRuntime) {
    if runtime.mode != MonitorMode::Single {
        return;
    }
    reset_monitor_runtime(runtime);
    append_log(runtime, "info", "单次监控已结束");
}

fn show_system_alert(app: &AppHandle, title: &str, message: &str, kind: MessageDialogKind) {
    app.dialog()
        .message(message.to_string())
        .title(title.to_string())
        .kind(kind)
        .show(|_| {});
}

fn show_loop_run_failed_alert(
    app: &AppHandle,
    pipeline_name: &str,
    pipeline_id: &str,
    run_id: &str,
    status: &str,
) {
    let status_text = resolve_status_text(status);
    show_system_alert(
        app,
        "循环监控 · 运行失败",
        &format!(
            "流水线 {pipeline_name}#{pipeline_id}\n运行 #{run_id}\n状态：{status_text}"
        ),
        MessageDialogKind::Error,
    );
}

fn show_single_run_ended_alert(
    app: &AppHandle,
    pipeline_name: &str,
    pipeline_id: &str,
    run_id: &str,
    status: &str,
) {
    let status_text = resolve_status_text(status);
    let failed = matches!(status, "FAIL" | "CANCELED" | "FAILED" | "ERROR");
    let (title, kind) = if failed {
        ("单次监控 · 运行失败", MessageDialogKind::Error)
    } else {
        ("单次监控 · 运行结束", MessageDialogKind::Info)
    };
    show_system_alert(
        app,
        title,
        &format!(
            "流水线 {pipeline_name}#{pipeline_id}\n运行 #{run_id}\n状态：{status_text}\n\n单次监控已自动停止。"
        ),
        kind,
    );
}

fn show_single_aborted_alert(app: &AppHandle, message: &str) {
    show_system_alert(
        app,
        "单次监控 · 已停止",
        &format!("{message}\n\n单次监控已自动停止。"),
        MessageDialogKind::Warning,
    );
}

fn running_conflict_message(mode: MonitorMode) -> String {
    match mode {
        MonitorMode::Loop => "循环监控运行中，请先停止".to_string(),
        MonitorMode::Single => "单次监控运行中，请先停止".to_string(),
        MonitorMode::Idle => "监控运行中，请先停止".to_string(),
    }
}

fn validate_loop_monitor_config(config: &PipelineMonitorConfig) -> Result<(), String> {
    if config.token.trim().is_empty() {
        return Err(TOKEN_HINT.to_string());
    }
    if config.org_id.trim().is_empty() {
        return Err(ORG_HINT.to_string());
    }
    if config.tracked_source_branch.trim().is_empty() {
        return Err(BRANCH_HINT.to_string());
    }
    let has_pipeline = config
        .pipelines
        .iter()
        .any(|item| !item.pipeline_id.trim().is_empty());
    if !has_pipeline {
        return Err(PIPELINE_LIST_HINT.to_string());
    }
    if !config
        .pipelines
        .iter()
        .any(|item| item.enabled && !item.pipeline_id.trim().is_empty())
    {
        return Err(ENABLED_HINT.to_string());
    }
    Ok(())
}

fn parse_global_param_value(response_data: &Value, key: &str) -> String {
    let Some(params) = response_data.get("globalParams").and_then(|v| v.as_array()) else {
        return String::new();
    };
    for param in params {
        if value_as_str(param.get("key").unwrap_or(&Value::Null)) == key {
            return value_as_str(param.get("value").unwrap_or(&Value::Null))
                .trim()
                .to_string();
        }
    }
    String::new()
}

fn parse_source_branch_names(response_data: &Value) -> Vec<String> {
    if let Some(params) = response_data.get("globalParams").and_then(|v| v.as_array()) {
        for param in params {
            if value_as_str(param.get("key").unwrap_or(&Value::Null)) != "CI_SOURCE_BRANCHES" {
                continue;
            }
            let value_text = value_as_str(param.get("value").unwrap_or(&Value::Null))
                .trim()
                .to_string();
            if value_text.is_empty() {
                break;
            }
            if let Ok(source_branches) = serde_json::from_str::<Value>(&value_text) {
                if let Some(items) = source_branches.as_array() {
                    let mut branch_names = Vec::new();
                    for source_branch in items {
                        let branch_name = value_as_str(
                            source_branch
                                .get("CI_COMMIT_REF_NAME")
                                .unwrap_or(&Value::Null),
                        )
                        .trim()
                        .to_string();
                        if !branch_name.is_empty() {
                            branch_names.push(branch_name);
                        }
                    }
                    if !branch_names.is_empty() {
                        return branch_names;
                    }
                }
            }
            break;
        }

        let mut fallback_branch_names = Vec::new();
        for param in params {
            let key_text = value_as_str(param.get("key").unwrap_or(&Value::Null))
                .trim()
                .to_string();
            if !key_text.starts_with("CI_COMMIT_REF_NAME_") {
                continue;
            }
            let value_text = value_as_str(param.get("value").unwrap_or(&Value::Null))
                .trim()
                .to_string();
            if value_text.is_empty() || value_text.starts_with("release/") {
                continue;
            }
            if !fallback_branch_names.contains(&value_text) {
                fallback_branch_names.push(value_text);
            }
        }
        if !fallback_branch_names.is_empty() {
            return fallback_branch_names;
        }
    }

    if let Some(sources) = response_data.get("sources").and_then(|v| v.as_array()) {
        for source in sources {
            let data = source.get("data").cloned().unwrap_or(Value::Null);
            let branch_text = value_as_str(data.get("branch").unwrap_or(&Value::Null))
                .trim()
                .to_string();
            if !branch_text.is_empty() && !branch_text.starts_with("release/") {
                return vec![branch_text];
            }
        }
    }
    Vec::new()
}

fn resolve_tracked_source_branch(response_data: &Value, tracked: &str) -> bool {
    let branches = parse_source_branch_names(response_data);
    branches == vec![tracked.to_string()]
}

fn resolve_validate_permission(job_data: &Value) -> bool {
    let Some(actions) = job_data.get("actions").and_then(|v| v.as_array()) else {
        return false;
    };
    for action in actions {
        let action_type = value_as_str(action.get("type").unwrap_or(&Value::Null));
        let disable = action.get("disable").and_then(|v| v.as_bool()).unwrap_or(true);
        if (action_type == "PassPipelineValidate" || action_type == "RefusePipelineValidate")
            && !disable
        {
            return true;
        }
    }
    false
}

fn resolve_execute_action(job_data: &Value) -> bool {
    let Some(actions) = job_data.get("actions").and_then(|v| v.as_array()) else {
        return false;
    };
    for action in actions {
        let action_type = value_as_str(action.get("type").unwrap_or(&Value::Null));
        let disable = action.get("disable").and_then(|v| v.as_bool()).unwrap_or(false);
        if action_type == "ExecutePipelineJobRun" && !disable {
            return true;
        }
    }
    false
}

fn resolve_validate_action(job_data: &Value) -> bool {
    let Some(actions) = job_data.get("actions").and_then(|v| v.as_array()) else {
        return false;
    };
    for action in actions {
        let action_type = value_as_str(action.get("type").unwrap_or(&Value::Null));
        if action_type == "PassPipelineValidate" || action_type == "RefusePipelineValidate" {
            return true;
        }
    }
    false
}

fn resolve_stage_has_live_manual(response_data: &Value, stage_name: &str) -> bool {
    let Some(stages) = response_data.get("stages").and_then(|v| v.as_array()) else {
        return false;
    };
    for stage in stages {
        let stage_info = stage.get("stageInfo").cloned().unwrap_or(Value::Null);
        let name = value_as_str(stage_info.get("name").unwrap_or(&Value::Null));
        let name = if name.is_empty() {
            "unknown".to_string()
        } else {
            name
        };
        if name != stage_name {
            continue;
        }
        if let Some(jobs) = stage_info.get("jobs").and_then(|v| v.as_array()) {
            for job in jobs {
                let job_name = value_as_str(job.get("name").unwrap_or(&Value::Null));
                let job_status = value_as_str(job.get("status").unwrap_or(&Value::Null));
                let job_status = if job_status.is_empty() {
                    "UNKNOWN".to_string()
                } else {
                    job_status
                };
                if job_name == "人工卡点"
                    && (job_status == "WAITING" || job_status == "SWITCH_MANUAL")
                {
                    return true;
                }
            }
        }
    }
    false
}

fn resolve_stage_branch_already_progressed(
    response_data: &Value,
    stage_name: &str,
    current_job_id: &str,
) -> bool {
    let progressed = ["SUCCESS", "RUNNING", "WAITING", "SWITCH_MANUAL"];
    let Some(stages) = response_data.get("stages").and_then(|v| v.as_array()) else {
        return false;
    };
    for stage in stages {
        let stage_info = stage.get("stageInfo").cloned().unwrap_or(Value::Null);
        let name = value_as_str(stage_info.get("name").unwrap_or(&Value::Null));
        let name = if name.is_empty() {
            "unknown".to_string()
        } else {
            name
        };
        if name != stage_name {
            continue;
        }
        if let Some(jobs) = stage_info.get("jobs").and_then(|v| v.as_array()) {
            for job in jobs {
                let job_id = value_as_str(job.get("id").unwrap_or(&Value::Null));
                let job_id = if job_id.is_empty() {
                    "unknown".to_string()
                } else {
                    job_id
                };
                if job_id == current_job_id {
                    continue;
                }
                let job_status = value_as_str(job.get("status").unwrap_or(&Value::Null));
                let job_status = if job_status.is_empty() {
                    "UNKNOWN".to_string()
                } else {
                    job_status
                };
                if progressed.contains(&job_status.as_str()) {
                    return true;
                }
            }
        }
    }
    false
}

fn extract_stage_rows(response_data: &Value) -> Vec<(Value, Value)> {
    let mut rows = Vec::new();
    let Some(stages) = response_data.get("stages").and_then(|v| v.as_array()) else {
        return rows;
    };
    for stage in stages {
        let stage_info = stage.get("stageInfo").cloned().unwrap_or(Value::Null);
        if let Some(jobs) = stage_info.get("jobs").and_then(|v| v.as_array()) {
            for job in jobs {
                rows.push((stage_info.clone(), job.clone()));
            }
        }
    }
    rows
}

fn build_pipeline_page_url(pipeline_id: &str, run_id: &str) -> String {
    let pipeline_id = pipeline_id.trim();
    let run_id = run_id.trim();
    if run_id.is_empty() {
        format!("{PIPELINE_PAGE_BASE}/{pipeline_id}/current")
    } else {
        format!("{PIPELINE_PAGE_BASE}/{pipeline_id}/builds/{run_id}")
    }
}

fn open_url_fallback(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| format!("fallback open failed: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("fallback open failed: {e}"))?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("fallback open failed: {e}"))?;
        Ok(())
    }
}

fn open_pipeline_page(app: &AppHandle, pipeline_id: &str, run_id: &str) -> Result<(), String> {
    if pipeline_id.trim().is_empty() {
        return Err("流水线 ID 为空".to_string());
    }
    let url = build_pipeline_page_url(pipeline_id, run_id);
    match app.opener().open_url(url.as_str(), None::<&str>) {
        Ok(()) => Ok(()),
        Err(opener_error) => open_url_fallback(&url).map_err(|fallback_error| {
            format!("打开页面失败：{opener_error}；{fallback_error}")
        }),
    }
}

async fn call_open_api(
    http: &Client,
    token: &str,
    path: &str,
    method: &str,
) -> OpenApiResult {
    let url = format!("{OPENAPI_BASE}{path}");
    let builder = match method {
        "POST" => http.post(&url),
        _ => http.get(&url),
    };
    let request = builder
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("x-yunxiao-token", token);

    let response = match request.send().await {
        Ok(resp) => resp,
        Err(error) => {
            if error.is_timeout() {
                return OpenApiResult {
                    success: false,
                    data: None,
                    error_message: "Request timeout".to_string(),
                    status_code: None,
                };
            }
            return OpenApiResult {
                success: false,
                data: None,
                error_message: format!("Network error: {error}"),
                status_code: None,
            };
        }
    };

    let status_code = response.status().as_u16();
    let body_text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return OpenApiResult {
                success: false,
                data: None,
                error_message: format!("Read body failed: {error}"),
                status_code: Some(status_code),
            };
        }
    };

    if !(200..300).contains(&status_code) {
        return OpenApiResult {
            success: false,
            data: None,
            error_message: format!("HTTP {status_code}, body: {body_text}"),
            status_code: Some(status_code),
        };
    }

    match serde_json::from_str::<Value>(&body_text) {
        Ok(data) => OpenApiResult {
            success: true,
            data: Some(data),
            error_message: String::new(),
            status_code: Some(status_code),
        },
        Err(error) => OpenApiResult {
            success: false,
            data: None,
            error_message: format!("Invalid JSON: {error}"),
            status_code: Some(status_code),
        },
    }
}

async fn query_latest_run(
    http: &Client,
    token: &str,
    org_id: &str,
    pipeline_id: &str,
) -> OpenApiResult {
    call_open_api(
        http,
        token,
        &format!(
            "/oapi/v1/flow/organizations/{org_id}/pipelines/{pipeline_id}/runs/latestPipelineRun"
        ),
        "GET",
    )
    .await
}

async fn query_pipeline_run(
    http: &Client,
    token: &str,
    org_id: &str,
    pipeline_id: &str,
    run_id: &str,
) -> OpenApiResult {
    call_open_api(
        http,
        token,
        &format!("/oapi/v1/flow/organizations/{org_id}/pipelines/{pipeline_id}/runs/{run_id}"),
        "GET",
    )
    .await
}

async fn query_user_name(http: &Client, token: &str, org_id: &str, user_id: &str) -> String {
    let result = call_open_api(
        http,
        token,
        &format!("/oapi/v1/platform/organizations/{org_id}/users/{user_id}"),
        "GET",
    )
    .await;
    if !result.success {
        return String::new();
    }
    result
        .data
        .as_ref()
        .map(|d| value_as_str(d.get("name").unwrap_or(&Value::Null)))
        .unwrap_or_default()
}

async fn resolve_trigger_user_name(
    http: &Client,
    token: &str,
    org_id: &str,
    response_data: &Value,
    creator_id: &str,
) -> String {
    let build_executor = parse_global_param_value(response_data, "BUILD_EXECUTOR");
    if !build_executor.is_empty() {
        return build_executor;
    }
    let build_message = parse_global_param_value(response_data, "BUILD_MESSAGE");
    if !build_message.is_empty() {
        let name = build_message
            .split_once('-')
            .map(|(left, _)| left.trim())
            .unwrap_or(build_message.trim());
        if !name.is_empty() {
            return name.to_string();
        }
    }
    if let Some(sources) = response_data.get("sources").and_then(|v| v.as_array()) {
        for source in sources {
            let data = source.get("data").cloned().unwrap_or(Value::Null);
            let username = value_as_str(data.get("username").unwrap_or(&Value::Null))
                .trim()
                .to_string();
            if !username.is_empty() {
                return username;
            }
        }
    }
    let creator_name = response_data
        .get("creator")
        .and_then(|c| c.get("name"))
        .map(value_as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if !creator_name.is_empty() {
        return creator_name;
    }
    if !creator_id.is_empty() {
        let queried = query_user_name(http, token, org_id, creator_id).await;
        if !queried.is_empty() {
            return queried;
        }
        return format!("unknown({creator_id})");
    }
    "unknown".to_string()
}

async fn pass_manual_checkpoint(
    http: &Client,
    token: &str,
    org_id: &str,
    pipeline_id: &str,
    run_id: &str,
    job_id: &str,
) -> OpenApiResult {
    call_open_api(
        http,
        token,
        &format!(
            "/oapi/v1/flow/organizations/{org_id}/pipelines/{pipeline_id}/pipelineRuns/{run_id}/jobs/{job_id}/pass"
        ),
        "POST",
    )
    .await
}

async fn refuse_manual_checkpoint(
    http: &Client,
    token: &str,
    org_id: &str,
    pipeline_id: &str,
    run_id: &str,
    job_id: &str,
) -> OpenApiResult {
    call_open_api(
        http,
        token,
        &format!(
            "/oapi/v1/flow/organizations/{org_id}/pipelines/{pipeline_id}/pipelineRuns/{run_id}/jobs/{job_id}/refuse"
        ),
        "POST",
    )
    .await
}

async fn execute_manual_node(
    http: &Client,
    token: &str,
    org_id: &str,
    pipeline_id: &str,
    run_id: &str,
    job_id: &str,
) -> OpenApiResult {
    call_open_api(
        http,
        token,
        &format!(
            "/oapi/v1/flow/organizations/{org_id}/pipelines/{pipeline_id}/pipelineRuns/{run_id}/jobs/{job_id}/start"
        ),
        "POST",
    )
    .await
}

fn build_snapshot(runtime: &MonitorRuntime) -> MonitorSnapshot {
    let mut pipelines = Vec::new();
    for item in &runtime.config.pipelines {
        let state = runtime.pipeline_states.get(&item.pipeline_id);
        pipelines.push(PipelineStatusView {
            pipeline_id: item.pipeline_id.clone(),
            pipeline_name: item.name.clone(),
            enabled: item.enabled,
            current_run_id: state
                .map(|s| s.current_run_id.clone())
                .unwrap_or_default(),
            current_run_status: state
                .map(|s| s.current_run_status.clone())
                .unwrap_or_default(),
            current_run_status_text: state
                .map(|s| resolve_status_text(&s.current_run_status))
                .unwrap_or_default(),
            trigger_user: state.map(|s| s.trigger_user.clone()).unwrap_or_default(),
            summary: state.map(|s| s.summary.clone()).unwrap_or_default(),
            last_seen_run_id: state
                .map(|s| s.last_seen_run_id.clone())
                .unwrap_or_default(),
        });
    }
    let pending_count = if runtime.current_pending.is_some() {
        1
    } else {
        0
    };
    MonitorSnapshot {
        running: runtime.running,
        mode: runtime.mode.as_str().to_string(),
        single_pipeline_id: runtime.single_pipeline_id.clone(),
        auto_mode: runtime.config.auto_mode,
        pending_count,
        current_pending: runtime.current_pending.clone(),
        pipelines,
        logs: runtime.logs.iter().cloned().collect(),
    }
}

fn emit_snapshot(app: &AppHandle, runtime: &MonitorRuntime) {
    let snapshot = build_snapshot(runtime);
    let _ = app.emit(STATE_EVENT, snapshot);
}

fn maybe_notify_pending(app: &AppHandle, runtime: &mut MonitorRuntime) {
    let Some(pending) = runtime.current_pending.clone() else {
        return;
    };
    if pending.id == runtime.last_notified_pending_id {
        return;
    }
    runtime.last_notified_pending_id = pending.id.clone();
    let body = if pending.kind == "validate" {
        format!(
            "validate · {}#{} · {}",
            pending.pipeline_name, pending.run_id, pending.stage_name
        )
    } else {
        format!(
            "branch select · {}#{}",
            pending.pipeline_name, pending.run_id
        )
    };
    let _ = app.emit(
        "pipeline-monitor-notify",
        serde_json::json!({
            "title": "Pipeline pending",
            "body": body,
            "pendingCount": 1,
        }),
    );
}

fn sync_pipeline_states(runtime: &mut MonitorRuntime) {
    let enabled: Vec<_> = runtime
        .config
        .pipelines
        .iter()
        .filter(|p| p.enabled)
        .cloned()
        .collect();
    let enabled_ids: HashSet<String> = enabled.iter().map(|p| p.pipeline_id.clone()).collect();
    runtime
        .pipeline_states
        .retain(|id, _| enabled_ids.contains(id));
    for item in enabled {
        runtime
            .pipeline_states
            .entry(item.pipeline_id.clone())
            .and_modify(|state| {
                state.pipeline_name = item.name.clone();
            })
            .or_insert_with(|| PipelineRuntimeState {
                pipeline_id: item.pipeline_id.clone(),
                pipeline_name: item.name.clone(),
                current_run_id: String::new(),
                current_run_status: String::new(),
                last_seen_run_id: String::new(),
                next_latest_query_time: 0,
                checkpoint_ack: HashMap::new(),
                trigger_user: String::new(),
                summary: String::new(),
            });
    }
}

fn drop_pending_if_stale(runtime: &mut MonitorRuntime) {
    let Some(pending) = runtime.current_pending.clone() else {
        return;
    };
    let Some(state) = runtime.pipeline_states.get(&pending.pipeline_id) else {
        runtime.current_pending = None;
        return;
    };
    if state.current_run_id != pending.run_id {
        runtime.current_pending = None;
    }
}

struct CycleResult {
    refresh: bool,
}

async fn attach_latest_run_if_needed(
    http: &Client,
    runtime: &mut MonitorRuntime,
    pipeline_id: &str,
    now: u64,
) -> bool {
    let config = runtime.config.clone();
    let state = match runtime.pipeline_states.get_mut(pipeline_id) {
        Some(state) => state,
        None => return false,
    };
    if !state.current_run_id.is_empty() {
        return false;
    }
    if now < state.next_latest_query_time {
        return false;
    }

    let prefix = format!("{}#{}", state.pipeline_name, state.pipeline_id);
    let latest = query_latest_run(http, &config.token, &config.org_id, &state.pipeline_id).await;
    if !latest.success || latest.data.is_none() {
        append_log(
            runtime,
            "warn",
            format!(
                "{prefix} 查询最新运行失败：{}",
                latest.error_message
            ),
        );
        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
            state.next_latest_query_time = now + config.idle_latest_query_interval_secs;
        }
        return false;
    }
    let latest_run = latest.data.unwrap();
    let latest_run_id = value_as_str(latest_run.get("pipelineRunId").unwrap_or(&Value::Null))
        .trim()
        .to_string();
    let latest_run_status = {
        let s = value_as_str(latest_run.get("status").unwrap_or(&Value::Null));
        if s.is_empty() {
            "UNKNOWN".to_string()
        } else {
            s
        }
    };

    if latest_run_id.is_empty() {
        append_log(runtime, "info", format!("{prefix} 暂无最新运行"));
        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
            state.next_latest_query_time = now + config.idle_latest_query_interval_secs;
        }
        return false;
    }

    let state = runtime.pipeline_states.get_mut(pipeline_id).unwrap();
    if latest_run_id == state.last_seen_run_id {
        state.next_latest_query_time = now + config.idle_latest_query_interval_secs;
        append_log(
            runtime,
            "info",
            format!("{prefix} 最新运行 #{latest_run_id} 已处理，稍后重试"),
        );
        return false;
    }

    if resolve_run_finished(&latest_run_status) {
        state.last_seen_run_id = latest_run_id.clone();
        state.next_latest_query_time = now + config.idle_latest_query_interval_secs;
        append_log(
            runtime,
            "info",
            format!(
                "{prefix} 最新运行 #{latest_run_id} 已结束（{}），稍后重试",
                resolve_status_text(&latest_run_status)
            ),
        );
        return false;
    }

    let detail =
        query_pipeline_run(http, &config.token, &config.org_id, pipeline_id, &latest_run_id).await;
    if !detail.success || detail.data.is_none() {
        append_log(
            runtime,
            "warn",
            format!(
                "{prefix} 查询运行 #{latest_run_id} 详情失败：{}",
                detail.error_message
            ),
        );
        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
            state.next_latest_query_time = now + config.idle_latest_query_interval_secs;
        }
        return false;
    }
    let detail_data = detail.data.unwrap();

    if !resolve_tracked_source_branch(&detail_data, &config.tracked_source_branch) {
        let branch_names = parse_source_branch_names(&detail_data);
        let branch_text = if branch_names.is_empty() {
            "未知".to_string()
        } else {
            branch_names.join(",")
        };
        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
            state.last_seen_run_id = latest_run_id.clone();
            state.next_latest_query_time = now + config.idle_latest_query_interval_secs;
        }
        append_log(
            runtime,
            "info",
            format!(
                "{prefix} 最新运行 #{latest_run_id} 分支 [{branch_text}] 已跳过"
            ),
        );
        return false;
    }

    let creator_id =
        value_as_str(latest_run.get("creatorAccountId").unwrap_or(&Value::Null));
    let creator_name =
        resolve_trigger_user_name(http, &config.token, &config.org_id, &detail_data, &creator_id)
            .await;
    if !config.allowed_trigger_users.is_empty()
        && !config.allowed_trigger_users.contains(&creator_name)
    {
        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
            state.last_seen_run_id = latest_run_id.clone();
            state.next_latest_query_time = now + config.idle_latest_query_interval_secs;
        }
        append_log(
            runtime,
            "info",
            format!(
                "{prefix} 最新运行 #{latest_run_id} 触发人 [{creator_name}] 不在白名单"
            ),
        );
        return false;
    }

    if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
        state.current_run_id = latest_run_id.clone();
        state.current_run_status = latest_run_status.clone();
        state.last_seen_run_id = latest_run_id.clone();
        state.next_latest_query_time = 0;
        state.checkpoint_ack.clear();
        state.trigger_user = creator_name.clone();
        state.summary.clear();
    }
    let trigger_text = resolve_trigger_text(latest_run.get("triggerMode").unwrap_or(&Value::Null));
    append_log(
        runtime,
        "info",
        format!(
            "{prefix} 开始监控 #{latest_run_id}（{}）触发方式={trigger_text} 触发人={creator_name}",
            resolve_status_text(&latest_run_status)
        ),
    );
    true
}

async fn inspect_pipeline_run(
    app: &AppHandle,
    http: &Client,
    runtime: &mut MonitorRuntime,
    pipeline_id: &str,
    now: u64,
) -> CycleResult {
    let mut cycle = CycleResult { refresh: false };
    let config = runtime.config.clone();
    let auto_mode = config.auto_mode;

    let (pipeline_name, current_run_id) = {
        let Some(state) = runtime.pipeline_states.get(pipeline_id) else {
            return cycle;
        };
        if state.current_run_id.is_empty() {
            return cycle;
        }
        (state.pipeline_name.clone(), state.current_run_id.clone())
    };
    let prefix = format!("{pipeline_name}#{pipeline_id}");

    let response =
        query_pipeline_run(http, &config.token, &config.org_id, pipeline_id, &current_run_id)
            .await;
    if !response.success || response.data.is_none() {
        append_log(
            runtime,
            "warn",
            format!("{prefix} 请求失败：{}", response.error_message),
        );
        return cycle;
    }
    let response_data = response.data.unwrap();

    let single_mode = runtime.mode == MonitorMode::Single;

    if !single_mode && !resolve_tracked_source_branch(&response_data, &config.tracked_source_branch)
    {
        let branch_names = parse_source_branch_names(&response_data);
        let branch_text = if branch_names.is_empty() {
            "未知".to_string()
        } else {
            branch_names.join(",")
        };
        append_log(
            runtime,
            "info",
            format!(
                "{prefix} 分支 [{branch_text}] 非 {}，停止当前运行",
                config.tracked_source_branch
            ),
        );
        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
            clear_current_run(state, now + config.idle_latest_query_interval_secs);
        }
        if runtime
            .current_pending
            .as_ref()
            .map(|p| p.pipeline_id == pipeline_id)
            .unwrap_or(false)
        {
            runtime.current_pending = None;
        }
        return cycle;
    }

    let pipeline_status = resolve_effective_run_status(&response_data);
    if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
        state.current_run_status = pipeline_status.clone();
    }

    let need_trigger_user = runtime
        .pipeline_states
        .get(pipeline_id)
        .map(|s| s.trigger_user.trim().is_empty())
        .unwrap_or(false);
    if need_trigger_user {
        let creator_id =
            value_as_str(response_data.get("creatorAccountId").unwrap_or(&Value::Null));
        let creator_name = resolve_trigger_user_name(
            http,
            &config.token,
            &config.org_id,
            &response_data,
            &creator_id,
        )
        .await;
        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
            state.trigger_user = if creator_name.is_empty() {
                if creator_id.is_empty() {
                    "未知".to_string()
                } else {
                    format!("未知({creator_id})")
                }
            } else {
                creator_name
            };
        }
    }

    if resolve_run_finished(&pipeline_status) {
        append_log(
            runtime,
            "info",
            format!(
                "{prefix} 运行已结束（{}）",
                resolve_status_text(&pipeline_status)
            ),
        );
        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
            let finished = state.current_run_id.clone();
            clear_current_run(state, now);
            state.last_seen_run_id = finished;
        }
        if runtime
            .current_pending
            .as_ref()
            .map(|p| p.pipeline_id == pipeline_id)
            .unwrap_or(false)
        {
            runtime.current_pending = None;
        }
        if single_mode {
            show_single_run_ended_alert(
                app,
                &pipeline_name,
                pipeline_id,
                &current_run_id,
                &pipeline_status,
            );
            finish_single_monitor(runtime);
        } else if matches!(pipeline_status.as_str(), "FAIL" | "CANCELED" | "FAILED" | "ERROR") {
            show_loop_run_failed_alert(
                app,
                &pipeline_name,
                pipeline_id,
                &current_run_id,
                &pipeline_status,
            );
        }
        return cycle;
    }

    let mut active_summary: Option<String> = None;
    let mut execute_candidates: Vec<ExecuteCandidateView> = Vec::new();
    let mut current_execute_prompt_keys: HashSet<String> = HashSet::new();
    let mut pending_assigned = runtime.current_pending.is_some();
    let mut found_checkpoint = false;
    let mut found_branch_selector = false;
    let mut found_new = false;
    let mut found_active = false;

    for (stage_info, job) in extract_stage_rows(&response_data) {
        let job_id = {
            let s = value_as_str(job.get("id").unwrap_or(&Value::Null));
            if s.is_empty() {
                "未知".to_string()
            } else {
                s
            }
        };
        let job_name = value_as_str(job.get("name").unwrap_or(&Value::Null));
        let job_status = {
            let s = value_as_str(job.get("status").unwrap_or(&Value::Null));
            if s.is_empty() {
                "UNKNOWN".to_string()
            } else {
                s
            }
        };
        let stage_name = {
            let s = value_as_str(stage_info.get("name").unwrap_or(&Value::Null));
            if s.is_empty() {
                "未知".to_string()
            } else {
                s
            }
        };
        let stage_status = {
            let s = value_as_str(stage_info.get("status").unwrap_or(&Value::Null));
            if s.is_empty() {
                "UNKNOWN".to_string()
            } else {
                s
            }
        };
        let job_start_time = job.get("startTime").and_then(value_to_i64);

        if !single_mode && resolve_prod_publish_started(&stage_name, &job_name, &job_status) {
            append_log(
                runtime,
                "info",
                format!("{prefix} 检测到 {stage_name}:{job_name} 已开始，停止当前运行"),
            );
            if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
                let finished = state.current_run_id.clone();
                clear_current_run(state, now);
                state.last_seen_run_id = finished;
            }
            if runtime
                .current_pending
                .as_ref()
                .map(|p| p.pipeline_id == pipeline_id)
                .unwrap_or(false)
            {
                runtime.current_pending = None;
            }
            return cycle;
        }

        // 状态摘要只保留最后一个 RUNNING / WAITING 节点
        if job_status == "RUNNING" {
            let duration_text = job_start_time
                .map(|start_ms| format_duration_text(now as i64 - start_ms / 1000))
                .unwrap_or_default();
            active_summary = Some(if duration_text.is_empty() {
                format!("[{stage_name}:{job_name}:运行中]")
            } else {
                format!("[{stage_name}:{job_name}:运行中 {duration_text}]")
            });
            found_active = true;
        } else if job_status == "WAITING" || job_status == "SWITCH_MANUAL" {
            active_summary = Some(format!("[{stage_name}:{job_name}:等待中]"));
            found_active = true;
        }

        let has_execute_action = resolve_execute_action(&job);
        let has_validate_action = resolve_validate_action(&job);

        if has_execute_action
            && job_status == "INIT"
            && (stage_status == "SWITCH_MANUAL" || stage_status == "WAITING")
        {
            let progressed =
                resolve_stage_branch_already_progressed(&response_data, &stage_name, &job_id);
            found_branch_selector = true;
            found_active = true;
            if !progressed {
                let key = format!("execute:{job_id}");
                current_execute_prompt_keys.insert(key.clone());
                let acked = runtime
                    .pipeline_states
                    .get(pipeline_id)
                    .and_then(|s| s.checkpoint_ack.get(&key).copied())
                    .unwrap_or(false);
                if !acked {
                    execute_candidates.push(ExecuteCandidateView {
                        label: build_execute_button_text(&stage_name, &job_name),
                        job_id: job_id.clone(),
                        job_name: job_name.clone(),
                        stage_name: stage_name.clone(),
                    });
                }
            }
            continue;
        }

        if job_name == "人工卡点" {
            found_checkpoint = true;
            let stage_has_live = resolve_stage_has_live_manual(&response_data, &stage_name);
            let manual_job_live = job_status == "WAITING" || job_status == "SWITCH_MANUAL";
            let manual_job_inferred = !stage_has_live
                && (stage_status == "SWITCH_MANUAL" || stage_status == "WAITING")
                && job_status == "INIT";

            if !manual_job_live && !manual_job_inferred {
                if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
                    state.checkpoint_ack.remove(&format!("validate:{job_id}"));
                }
            }

            if has_validate_action
                && (job_status == "WAITING"
                    || job_status == "SWITCH_MANUAL"
                    || manual_job_live)
            {
                let can_approve = resolve_validate_permission(&job);
                let ack_key = format!("validate:{job_id}");
                let acked = runtime
                    .pipeline_states
                    .get(pipeline_id)
                    .and_then(|s| s.checkpoint_ack.get(&ack_key).copied())
                    .unwrap_or(false);

                if !pending_assigned && !acked {
                    found_new = true;
                    if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
                        state.checkpoint_ack.insert(ack_key.clone(), true);
                    }

                    if auto_mode {
                        if can_approve {
                            let pass_result = pass_manual_checkpoint(
                                http,
                                &config.token,
                                &config.org_id,
                                pipeline_id,
                                &current_run_id,
                                &job_id,
                            )
                            .await;
                            if pass_result.success {
                                append_log(
                                    runtime,
                                    "info",
                                    format!(
                                        "{prefix} 自动通过阶段「{stage_name}」 jobId={job_id}"
                                    ),
                                );
                            } else {
                                append_log(
                                    runtime,
                                    "error",
                                    format!(
                                        "{prefix} 自动通过失败 阶段「{stage_name}」 jobId={job_id}：{}",
                                        pass_result.error_message
                                    ),
                                );
                                if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
                                    clear_current_run(
                                        state,
                                        now + config.idle_latest_query_interval_secs,
                                    );
                                }
                                if single_mode {
                                    show_single_aborted_alert(
                                        app,
                                        &format!(
                                            "{prefix} 自动通过失败：{}",
                                            pass_result.error_message
                                        ),
                                    );
                                    finish_single_monitor(runtime);
                                }
                                return cycle;
                            }
                        } else {
                            append_log(
                                runtime,
                                "info",
                                format!(
                                    "{prefix} 人工卡点无审批权限 阶段「{stage_name}」 jobId={job_id}"
                                ),
                            );
                            if single_mode {
                                if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
                                    clear_current_run(
                                        state,
                                        now + config.idle_latest_query_interval_secs,
                                    );
                                }
                                show_single_aborted_alert(
                                    app,
                                    &format!("{prefix} 人工卡点无审批权限，阶段「{stage_name}」"),
                                );
                                finish_single_monitor(runtime);
                                return cycle;
                            }
                        }
                        cycle.refresh = true;
                    } else {
                        runtime.current_pending = Some(CurrentPending {
                            id: format!("validate:{pipeline_id}:{current_run_id}:{job_id}"),
                            kind: "validate".to_string(),
                            pipeline_id: pipeline_id.to_string(),
                            pipeline_name: pipeline_name.clone(),
                            run_id: current_run_id.clone(),
                            stage_name: stage_name.clone(),
                            job_id: job_id.clone(),
                            job_name: job_name.clone(),
                            job_status: job_status.clone(),
                            stage_status: stage_status.clone(),
                            can_approve,
                            candidates: vec![],
                            discovered_at: now,
                        });
                        pending_assigned = true;
                        cycle.refresh = true;
                        append_log(
                            runtime,
                            "info",
                            format!("{prefix} 待处理人工卡点 阶段「{stage_name}」"),
                        );
                    }
                }

                found_active = true;
            } else if manual_job_inferred {
                found_active = true;
            }
        }
    }

    // cleanup stale execute acks
    if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
        let stale: Vec<String> = state
            .checkpoint_ack
            .keys()
            .filter(|k| k.starts_with("execute:") && !current_execute_prompt_keys.contains(*k))
            .cloned()
            .collect();
        for key in stale {
            state.checkpoint_ack.remove(&key);
        }
    }

    if !pending_assigned && !execute_candidates.is_empty() {
        found_new = true;
        if auto_mode {
            match resolve_auto_execute_candidate(&execute_candidates) {
                None => {
                    append_log(
                        runtime,
                        "info",
                        format!("{prefix} 发现分支选择，但无「人工卡点」分支"),
                    );
                    if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
                        clear_current_run(state, now + config.idle_latest_query_interval_secs);
                    }
                    if single_mode {
                        show_single_aborted_alert(
                            app,
                            &format!("{prefix} 发现分支选择，但无「人工卡点」分支"),
                        );
                        finish_single_monitor(runtime);
                    }
                    return cycle;
                }
                Some(selected) => {
                    let exec_result = execute_manual_node(
                        http,
                        &config.token,
                        &config.org_id,
                        pipeline_id,
                        &current_run_id,
                        &selected.job_id,
                    )
                    .await;
                    if exec_result.success {
                        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
                            state
                                .checkpoint_ack
                                .insert(format!("execute:{}", selected.job_id), true);
                        }
                        append_log(
                            runtime,
                            "info",
                            format!(
                                "{prefix} 自动执行阶段「{}」任务「{}」",
                                selected.stage_name, selected.job_name
                            ),
                        );
                    } else {
                        append_log(
                            runtime,
                            "error",
                            format!(
                                "{prefix} 自动执行失败 阶段「{}」任务「{}」：{}",
                                selected.stage_name, selected.job_name, exec_result.error_message
                            ),
                        );
                        if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
                            clear_current_run(
                                state,
                                now + config.idle_latest_query_interval_secs,
                            );
                        }
                        if single_mode {
                            show_single_aborted_alert(
                                app,
                                &format!(
                                    "{prefix} 自动执行失败：{}",
                                    exec_result.error_message
                                ),
                            );
                            finish_single_monitor(runtime);
                        }
                        return cycle;
                    }
                    cycle.refresh = true;
                }
            }
        } else {
            if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
                for candidate in &execute_candidates {
                    state
                        .checkpoint_ack
                        .insert(format!("execute:{}", candidate.job_id), true);
                }
            }
            let first = &execute_candidates[0];
            runtime.current_pending = Some(CurrentPending {
                id: format!(
                    "execute:{pipeline_id}:{current_run_id}:{}",
                    first.job_id
                ),
                kind: "execute".to_string(),
                pipeline_id: pipeline_id.to_string(),
                pipeline_name: pipeline_name.clone(),
                run_id: current_run_id.clone(),
                stage_name: first.stage_name.clone(),
                job_id: first.job_id.clone(),
                job_name: first.job_name.clone(),
                job_status: "INIT".to_string(),
                stage_status: String::new(),
                can_approve: false,
                candidates: execute_candidates.clone(),
                discovered_at: now,
            });
            pending_assigned = true;
            cycle.refresh = true;
            append_log(
                runtime,
                "info",
                format!(
                    "{prefix} 待处理分支选择：{} 个选项",
                    execute_candidates.len()
                ),
            );
        }
    }

    let summary = active_summary.unwrap_or_default();
    if let Some(state) = runtime.pipeline_states.get_mut(pipeline_id) {
        state.summary = summary.clone();
    }
    if !summary.is_empty() {
        append_log(runtime, "debug", format!("{prefix} {summary}"));
    }

    let _ = (
        found_checkpoint,
        found_branch_selector,
        found_new,
        found_active,
        pending_assigned,
    );
    cycle
}
async fn run_monitor_cycle(app: &AppHandle, http: &Client, state: &MonitorState) -> u64 {
    let mut refresh = false;
    let (running, mode, poll_interval, post_delay, pipeline_ids) = {
        let mut runtime = state.inner.lock().await;
        if !runtime.running {
            return 1;
        }
        sync_pipeline_states(&mut runtime);
        drop_pending_if_stale(&mut runtime);
        let ids: Vec<String> = if runtime.mode == MonitorMode::Single {
            if runtime.single_pipeline_id.is_empty() {
                Vec::new()
            } else {
                vec![runtime.single_pipeline_id.clone()]
            }
        } else {
            runtime
                .config
                .pipelines
                .iter()
                .filter(|p| p.enabled)
                .map(|p| p.pipeline_id.clone())
                .collect()
        };
        (
            runtime.running,
            runtime.mode,
            runtime.config.poll_interval_secs.max(5),
            runtime.config.post_action_refresh_delay_secs.max(1),
            ids,
        )
    };
    if !running {
        return 1;
    }

    let now = now_secs();
    for pipeline_id in pipeline_ids {
        if mode == MonitorMode::Loop {
            let mut runtime = state.inner.lock().await;
            if !runtime.running {
                break;
            }
            let attached =
                attach_latest_run_if_needed(http, &mut runtime, &pipeline_id, now).await;
            if attached {
                refresh = true;
            }
            emit_snapshot(app, &runtime);
        }

        {
            let mut runtime = state.inner.lock().await;
            if !runtime.running {
                break;
            }
            let cycle = inspect_pipeline_run(app, http, &mut runtime, &pipeline_id, now).await;
            if cycle.refresh {
                refresh = true;
            }
            maybe_notify_pending(app, &mut runtime);
            emit_snapshot(app, &runtime);
            if !runtime.running {
                break;
            }
        }

        if mode == MonitorMode::Loop {
            let mut runtime = state.inner.lock().await;
            if !runtime.running {
                break;
            }
            let need_reattach = runtime
                .pipeline_states
                .get(&pipeline_id)
                .map(|s| s.current_run_id.is_empty())
                .unwrap_or(false);
            if need_reattach {
                let attached =
                    attach_latest_run_if_needed(http, &mut runtime, &pipeline_id, now).await;
                if attached {
                    refresh = true;
                }
                emit_snapshot(app, &runtime);
            }
        }
    }

    if refresh {
        post_delay
    } else {
        poll_interval
    }
}

pub fn create_state() -> MonitorState {
    let config = load_config_from_disk().unwrap_or_default();
    MonitorState {
        inner: Mutex::new(MonitorRuntime {
            config,
            running: false,
            mode: MonitorMode::Idle,
            single_pipeline_id: String::new(),
            pipeline_states: HashMap::new(),
            current_pending: None,
            logs: VecDeque::new(),
            last_notified_pending_id: String::new(),
        }),
        loop_started: AtomicBool::new(false),
        http: Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_else(|_| Client::new()),
    }
}

pub fn spawn_background(app: AppHandle) {
    let state = app.state::<MonitorState>();
    if state
        .loop_started
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let state = app_handle.state::<MonitorState>();
            let sleep_secs = run_monitor_cycle(&app_handle, &state.http, &state).await;
            tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
        }
    });
}

pub async fn load_pipeline_monitor_config(
    state: State<'_, MonitorState>,
) -> Result<PipelineMonitorConfig, String> {
    let runtime = state.inner.lock().await;
    Ok(runtime.config.clone())
}

pub async fn save_pipeline_monitor_config(
    app: AppHandle,
    state: State<'_, MonitorState>,
    config: PipelineMonitorConfig,
) -> Result<PipelineMonitorConfig, String> {
    save_config_to_disk(&config)?;
    let mut runtime = state.inner.lock().await;
    runtime.config = config.clone();
    sync_pipeline_states(&mut runtime);
    append_log(&mut runtime, "info", "配置已保存");
    emit_snapshot(&app, &runtime);
    Ok(config)
}

pub async fn start_pipeline_monitor(
    app: AppHandle,
    state: State<'_, MonitorState>,
) -> Result<MonitorSnapshot, String> {
    let mut runtime = state.inner.lock().await;
    if runtime.running {
        return Err(running_conflict_message(runtime.mode));
    }
    validate_loop_monitor_config(&runtime.config)?;
    sync_pipeline_states(&mut runtime);
    runtime.mode = MonitorMode::Loop;
    runtime.single_pipeline_id.clear();
    runtime.running = true;
    append_log(&mut runtime, "info", "循环监控已启动");
    let snapshot = build_snapshot(&runtime);
    emit_snapshot(&app, &runtime);
    Ok(snapshot)
}

pub async fn start_pipeline_monitor_single(
    app: AppHandle,
    state: State<'_, MonitorState>,
    request: SingleMonitorRequest,
) -> Result<MonitorSnapshot, String> {
    let pipeline_id = request.pipeline_id.trim().to_string();
    let run_id = request.run_id.trim().to_string();
    if pipeline_id.is_empty() {
        return Err("流水线 ID 不能为空".to_string());
    }
    if run_id.is_empty() {
        return Err("运行 ID 不能为空".to_string());
    }

    let (token, org_id, pipeline_name) = {
        let runtime = state.inner.lock().await;
        if runtime.running {
            return Err(running_conflict_message(runtime.mode));
        }
        if runtime.config.token.trim().is_empty() {
            return Err(TOKEN_HINT.to_string());
        }
        if runtime.config.org_id.trim().is_empty() {
            return Err(ORG_HINT.to_string());
        }
        if !runtime
            .config
            .pipelines
            .iter()
            .any(|item| item.pipeline_id == pipeline_id)
        {
            return Err("流水线不在配置列表中".to_string());
        }
        let pipeline_name = runtime
            .config
            .pipelines
            .iter()
            .find(|item| item.pipeline_id == pipeline_id)
            .map(|item| item.name.clone())
            .unwrap_or_else(|| pipeline_id.clone());
        (
            runtime.config.token.clone(),
            runtime.config.org_id.clone(),
            pipeline_name,
        )
    };

    let detail = query_pipeline_run(&state.http, &token, &org_id, &pipeline_id, &run_id).await;
    let mut run_status = String::new();
    let mut trigger_user = String::new();
    let mut creator_id = String::new();

    if detail.success {
        if let Some(detail_data) = detail.data.as_ref() {
            run_status = resolve_effective_run_status(detail_data);
            creator_id =
                value_as_str(detail_data.get("creatorAccountId").unwrap_or(&Value::Null));
        }
    }

    // 运行详情里不一定带 creatorAccountId / status，再从最新运行补一次
    if creator_id.is_empty() || run_status.is_empty() {
        let latest = query_latest_run(&state.http, &token, &org_id, &pipeline_id).await;
        if latest.success {
            if let Some(latest_run) = latest.data.as_ref() {
                let latest_run_id =
                    value_as_str(latest_run.get("pipelineRunId").unwrap_or(&Value::Null))
                        .trim()
                        .to_string();
                if latest_run_id == run_id {
                    if creator_id.is_empty() {
                        creator_id = value_as_str(
                            latest_run.get("creatorAccountId").unwrap_or(&Value::Null),
                        );
                    }
                    if run_status.is_empty() {
                        run_status = resolve_effective_run_status(latest_run);
                    }
                }
            }
        }
    }

    if detail.success {
        if let Some(detail_data) = detail.data.as_ref() {
            trigger_user =
                resolve_trigger_user_name(&state.http, &token, &org_id, detail_data, &creator_id)
                    .await;
        }
    }
    if trigger_user.is_empty() {
        trigger_user = if creator_id.is_empty() {
            "未知".to_string()
        } else {
            format!("未知({creator_id})")
        };
    }

    // 启动前已结束：直接弹窗提示，不进入监控
    if !run_status.is_empty() && resolve_run_finished(&run_status) {
        show_single_run_ended_alert(&app, &pipeline_name, &pipeline_id, &run_id, &run_status);
        let mut runtime = state.inner.lock().await;
        append_log(
            &mut runtime,
            "info",
            format!(
                "{pipeline_name}#{pipeline_id} 单次监控未启动：运行 #{run_id} 已结束（{}）",
                resolve_status_text(&run_status)
            ),
        );
        let snapshot = build_snapshot(&runtime);
        emit_snapshot(&app, &runtime);
        return Ok(snapshot);
    }

    // 查不到状态时先按运行中启动，交给首次轮询确认真实状态
    if run_status.is_empty() {
        run_status = "RUNNING".to_string();
    }

    let mut runtime = state.inner.lock().await;
    if runtime.running {
        return Err(running_conflict_message(runtime.mode));
    }

    sync_pipeline_states(&mut runtime);
    for state_item in runtime.pipeline_states.values_mut() {
        state_item.current_run_id.clear();
        state_item.current_run_status.clear();
        state_item.summary.clear();
        state_item.checkpoint_ack.clear();
        state_item.trigger_user.clear();
        state_item.next_latest_query_time = 0;
    }
    runtime.current_pending = None;
    runtime.last_notified_pending_id.clear();

    if let Some(state_item) = runtime.pipeline_states.get_mut(&pipeline_id) {
        state_item.current_run_id = run_id.clone();
        state_item.current_run_status = run_status.clone();
        state_item.last_seen_run_id = run_id.clone();
        state_item.next_latest_query_time = 0;
        state_item.checkpoint_ack.clear();
        state_item.summary.clear();
        state_item.trigger_user = trigger_user.clone();
    } else {
        runtime.pipeline_states.insert(
            pipeline_id.clone(),
            PipelineRuntimeState {
                pipeline_id: pipeline_id.clone(),
                pipeline_name: pipeline_name.clone(),
                current_run_id: run_id.clone(),
                current_run_status: run_status.clone(),
                last_seen_run_id: run_id.clone(),
                next_latest_query_time: 0,
                checkpoint_ack: HashMap::new(),
                trigger_user: trigger_user.clone(),
                summary: String::new(),
            },
        );
    }

    runtime.mode = MonitorMode::Single;
    runtime.single_pipeline_id = pipeline_id.clone();
    runtime.running = true;
    let trigger_log = if trigger_user.is_empty() {
        "未知".to_string()
    } else {
        trigger_user
    };
    append_log(
        &mut runtime,
        "info",
        format!(
            "{pipeline_name}#{pipeline_id} 单次监控已启动，运行 #{run_id}（{}）触发人={trigger_log}",
            resolve_status_text(&run_status)
        ),
    );
    let snapshot = build_snapshot(&runtime);
    emit_snapshot(&app, &runtime);
    Ok(snapshot)
}

pub async fn stop_pipeline_monitor(
    app: AppHandle,
    state: State<'_, MonitorState>,
) -> Result<MonitorSnapshot, String> {
    let mut runtime = state.inner.lock().await;
    let stop_label = match runtime.mode {
        MonitorMode::Single => "单次监控已停止",
        MonitorMode::Loop => "循环监控已停止",
        MonitorMode::Idle => "监控已停止",
    };
    reset_monitor_runtime(&mut runtime);
    append_log(&mut runtime, "info", stop_label);
    let snapshot = build_snapshot(&runtime);
    emit_snapshot(&app, &runtime);
    Ok(snapshot)
}

pub async fn query_pipeline_latest_run(
    state: State<'_, MonitorState>,
    pipeline_id: String,
) -> Result<LatestRunInfo, String> {
    let pipeline_id = pipeline_id.trim().to_string();
    if pipeline_id.is_empty() {
        return Err("流水线 ID 不能为空".to_string());
    }
    let (token, org_id) = {
        let runtime = state.inner.lock().await;
        if runtime.config.token.trim().is_empty() {
            return Err(TOKEN_HINT.to_string());
        }
        (
            runtime.config.token.clone(),
            runtime.config.org_id.clone(),
        )
    };

    let latest = query_latest_run(&state.http, &token, &org_id, &pipeline_id).await;
    if !latest.success || latest.data.is_none() {
        return Err(if latest.error_message.is_empty() {
            "查询最新运行失败".to_string()
        } else {
            latest.error_message
        });
    }
    let latest_run = latest.data.unwrap();
    let run_id = value_as_str(latest_run.get("pipelineRunId").unwrap_or(&Value::Null))
        .trim()
        .to_string();
    if run_id.is_empty() {
        return Err("未找到运行记录".to_string());
    }
    let status = {
        let s = value_as_str(latest_run.get("status").unwrap_or(&Value::Null));
        if s.is_empty() {
            "UNKNOWN".to_string()
        } else {
            s
        }
    };
    let trigger_text = resolve_trigger_text(latest_run.get("triggerMode").unwrap_or(&Value::Null));
    let creator_id = value_as_str(latest_run.get("creatorAccountId").unwrap_or(&Value::Null));
    let detail = query_pipeline_run(&state.http, &token, &org_id, &pipeline_id, &run_id).await;
    let trigger_user = if detail.success {
        if let Some(detail_data) = detail.data.as_ref() {
            resolve_trigger_user_name(&state.http, &token, &org_id, detail_data, &creator_id).await
        } else {
            String::new()
        }
    } else {
        String::new()
    };
    let trigger_user = if trigger_user.is_empty() {
        if creator_id.is_empty() {
            "未知".to_string()
        } else {
            format!("未知({creator_id})")
        }
    } else {
        trigger_user
    };

    Ok(LatestRunInfo {
        pipeline_id,
        run_id,
        status_text: resolve_status_text(&status),
        status,
        trigger_text,
        trigger_user,
    })
}

pub async fn get_pipeline_monitor_snapshot(
    state: State<'_, MonitorState>,
) -> Result<MonitorSnapshot, String> {
    let runtime = state.inner.lock().await;
    Ok(build_snapshot(&runtime))
}

pub async fn respond_pipeline_monitor_action(
    app: AppHandle,
    state: State<'_, MonitorState>,
    request: ActionRequest,
) -> Result<MonitorSnapshot, String> {
    let (action, pending, config) = {
        let runtime = state.inner.lock().await;
        let pending = runtime
            .current_pending
            .clone()
            .ok_or_else(|| "当前没有待办".to_string())?;
        if !request.pending_id.is_empty() && request.pending_id != pending.id {
            return Err("待办已变更，请刷新".to_string());
        }
        (request.action.clone(), pending, runtime.config.clone())
    };

    match action.as_str() {
        "later" => {
            let mut runtime = state.inner.lock().await;
            runtime.current_pending = None;
            append_log(&mut runtime, "info", "已标记为稍后处理");
            let snapshot = build_snapshot(&runtime);
            emit_snapshot(&app, &runtime);
            Ok(snapshot)
        }
        "open" => {
            open_pipeline_page(&app, &pending.pipeline_id, &pending.run_id)?;
            let mut runtime = state.inner.lock().await;
            runtime.current_pending = None;
            append_log(&mut runtime, "info", "已打开云效页面");
            let snapshot = build_snapshot(&runtime);
            emit_snapshot(&app, &runtime);
            Ok(snapshot)
        }
        "pass" => {
            if pending.kind != "validate" {
                return Err("当前待办不是人工卡点审批".to_string());
            }
            let result = pass_manual_checkpoint(
                &state.http,
                &config.token,
                &config.org_id,
                &pending.pipeline_id,
                &pending.run_id,
                &pending.job_id,
            )
            .await;
            let mut runtime = state.inner.lock().await;
            if result.success {
                runtime.current_pending = None;
                append_log(
                    &mut runtime,
                    "info",
                    format!("已通过人工卡点 jobId={}", pending.job_id),
                );
            } else {
                append_log(
                    &mut runtime,
                    "error",
                    format!(
                        "通过失败 jobId={}：{}",
                        pending.job_id, result.error_message
                    ),
                );
                let _ = open_pipeline_page(&app, &pending.pipeline_id, &pending.run_id);
            }
            let snapshot = build_snapshot(&runtime);
            emit_snapshot(&app, &runtime);
            Ok(snapshot)
        }
        "refuse" => {
            if pending.kind != "validate" {
                return Err("当前待办不是人工卡点审批".to_string());
            }
            let result = refuse_manual_checkpoint(
                &state.http,
                &config.token,
                &config.org_id,
                &pending.pipeline_id,
                &pending.run_id,
                &pending.job_id,
            )
            .await;
            let mut runtime = state.inner.lock().await;
            if result.success {
                runtime.current_pending = None;
                append_log(
                    &mut runtime,
                    "info",
                    format!("已拒绝人工卡点 jobId={}", pending.job_id),
                );
            } else {
                append_log(
                    &mut runtime,
                    "error",
                    format!(
                        "拒绝失败 jobId={}：{}",
                        pending.job_id, result.error_message
                    ),
                );
                let _ = open_pipeline_page(&app, &pending.pipeline_id, &pending.run_id);
            }
            let snapshot = build_snapshot(&runtime);
            emit_snapshot(&app, &runtime);
            Ok(snapshot)
        }
        "execute" => {
            if pending.kind != "execute" {
                return Err("当前待办不是分支选择".to_string());
            }
            let job_id = if request.job_id.is_empty() {
                pending.job_id.clone()
            } else {
                request.job_id.clone()
            };
            let result = execute_manual_node(
                &state.http,
                &config.token,
                &config.org_id,
                &pending.pipeline_id,
                &pending.run_id,
                &job_id,
            )
            .await;
            let mut runtime = state.inner.lock().await;
            if result.success {
                if let Some(state_item) = runtime.pipeline_states.get_mut(&pending.pipeline_id) {
                    state_item
                        .checkpoint_ack
                        .insert(format!("execute:{job_id}"), true);
                }
                runtime.current_pending = None;
                append_log(
                    &mut runtime,
                    "info",
                    format!("已执行分支选择 jobId={job_id}"),
                );
            } else {
                if let Some(state_item) = runtime.pipeline_states.get_mut(&pending.pipeline_id) {
                    state_item.checkpoint_ack.remove(&format!("execute:{job_id}"));
                }
                append_log(
                    &mut runtime,
                    "error",
                    format!("执行失败 jobId={job_id}：{}", result.error_message),
                );
            }
            let snapshot = build_snapshot(&runtime);
            emit_snapshot(&app, &runtime);
            Ok(snapshot)
        }
        other => Err(format!("未知操作：{other}")),
    }
}

pub async fn clear_pipeline_monitor_logs(
    app: AppHandle,
    state: State<'_, MonitorState>,
) -> Result<MonitorSnapshot, String> {
    let mut runtime = state.inner.lock().await;
    runtime.logs.clear();
    append_log(&mut runtime, "info", "日志已清空");
    let snapshot = build_snapshot(&runtime);
    emit_snapshot(&app, &runtime);
    Ok(snapshot)
}

pub fn open_pipeline_run_page(
    app: AppHandle,
    pipeline_id: String,
    run_id: String,
) -> Result<(), String> {
    open_pipeline_page(&app, &pipeline_id, &run_id)
}
