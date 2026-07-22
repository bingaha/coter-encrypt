use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use directories::ProjectDirs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::Mutex;

const CONFIG_FILE_NAME: &str = "yunxiao-merge.json";
const STATE_EVENT: &str = "merge-monitor-state";
const MAX_LOGS: usize = 200;
const OPENAPI_BASE: &str = "https://openapi-rdc.aliyuncs.com";

const TOKEN_HINT: &str = "请先配置云效 Token";
const ORG_HINT: &str = "请先配置组织 ID";
const REPO_LIST_HINT: &str = "请先配置仓库列表";
const ENABLED_HINT: &str = "请至少启用一个仓库";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfigItem {
    pub name: String,
    pub repository_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeMonitorConfig {
    pub token: String,
    pub org_id: String,
    pub list_poll_interval_secs: u64,
    pub ai_poll_interval_secs: u64,
    pub allowed_authors: Vec<String>,
    pub repositories: Vec<RepoConfigItem>,
}

impl Default for MergeMonitorConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            org_id: String::new(),
            list_poll_interval_secs: 30,
            ai_poll_interval_secs: 10,
            allowed_authors: Vec::new(),
            repositories: Vec::new(),
        }
    }
}

fn clamp_config(config: &mut MergeMonitorConfig) {
    config.list_poll_interval_secs = config.list_poll_interval_secs.max(5);
    config.ai_poll_interval_secs = config.ai_poll_interval_secs.max(3);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeTodo {
    pub project_id: i64,
    pub local_id: i64,
    pub author_name: String,
    pub title: String,
    pub repo_name: String,
    pub detail_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedMerge {
    pub project_id: i64,
    pub local_id: i64,
    pub author_name: String,
    pub title: String,
    pub repo_name: String,
    pub detail_url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoStatusView {
    pub repository_id: String,
    pub name: String,
    pub enabled: bool,
    pub summary: String,
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
pub struct MergeSnapshot {
    pub running: bool,
    pub todo_count: usize,
    pub current: Option<TrackedMerge>,
    pub todos: Vec<MergeTodo>,
    pub repositories: Vec<RepoStatusView>,
    pub logs: Vec<LogEntry>,
}

#[derive(Debug)]
struct MergeRuntime {
    config: MergeMonitorConfig,
    running: bool,
    current: Option<TrackedMerge>,
    processed: HashSet<(i64, i64)>,
    todos: Vec<MergeTodo>,
    logs: VecDeque<LogEntry>,
    repo_summaries: HashMap<String, String>,
}

pub struct MergeMonitorState {
    inner: Mutex<MergeRuntime>,
    pub loop_started: AtomicBool,
    http: std::sync::Mutex<Client>,
}

impl MergeMonitorState {
    pub fn http_client(&self) -> Client {
        self.http
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn replace_http_client(&self, client: Client) {
        *self.http.lock().unwrap_or_else(|e| e.into_inner()) = client;
    }
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

fn load_config_from_disk() -> Result<MergeMonitorConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        let mut config = MergeMonitorConfig::default();
        clamp_config(&mut config);
        save_config_to_disk(&config)?;
        return Ok(config);
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Read config failed: {e}"))?;
    let mut config: MergeMonitorConfig =
        serde_json::from_str(&content).map_err(|e| format!("Parse config failed: {e}"))?;
    clamp_config(&mut config);
    Ok(config)
}

fn save_config_to_disk(config: &MergeMonitorConfig) -> Result<(), String> {
    let path = config_path()?;
    ensure_config_dir(&path)?;
    let content =
        serde_json::to_string_pretty(config).map_err(|e| format!("Serialize config failed: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("Write config failed: {e}"))
}

fn append_log(runtime: &mut MergeRuntime, level: &str, message: impl Into<String>) {
    runtime.logs.push_back(LogEntry {
        timestamp: now_clock(),
        level: level.to_string(),
        message: message.into(),
    });
    while runtime.logs.len() > MAX_LOGS {
        runtime.logs.pop_front();
    }
}

fn build_snapshot(runtime: &MergeRuntime) -> MergeSnapshot {
    let repositories = runtime
        .config
        .repositories
        .iter()
        .map(|repo| RepoStatusView {
            repository_id: repo.repository_id.clone(),
            name: repo.name.clone(),
            enabled: repo.enabled,
            summary: runtime
                .repo_summaries
                .get(&repo.repository_id)
                .cloned()
                .unwrap_or_default(),
        })
        .collect();

    MergeSnapshot {
        running: runtime.running,
        todo_count: runtime.todos.len(),
        current: runtime.current.clone(),
        todos: runtime.todos.clone(),
        repositories,
        logs: runtime.logs.iter().cloned().collect(),
    }
}

fn emit_snapshot(app: &AppHandle, runtime: &MergeRuntime) {
    let snapshot = build_snapshot(runtime);
    let _ = app.emit(STATE_EVENT, snapshot);
}

pub fn create_state() -> MergeMonitorState {
    let config = load_config_from_disk().unwrap_or_else(|_| {
        let mut config = MergeMonitorConfig::default();
        clamp_config(&mut config);
        config
    });
    let proxy = crate::http_client::load_http_proxy_config().unwrap_or_default();
    let http = crate::http_client::build_http_client(Duration::from_secs(20), &proxy)
        .unwrap_or_else(|_| Client::new());
    MergeMonitorState {
        inner: Mutex::new(MergeRuntime {
            config,
            running: false,
            current: None,
            processed: HashSet::new(),
            todos: Vec::new(),
            logs: VecDeque::new(),
            repo_summaries: HashMap::new(),
        }),
        loop_started: AtomicBool::new(false),
        http: std::sync::Mutex::new(http),
    }
}

fn validate_merge_monitor_config(config: &MergeMonitorConfig) -> Result<(), String> {
    if config.token.trim().is_empty() {
        return Err(TOKEN_HINT.to_string());
    }
    if config.org_id.trim().is_empty() {
        return Err(ORG_HINT.to_string());
    }
    let has_repo = config
        .repositories
        .iter()
        .any(|item| !item.repository_id.trim().is_empty());
    if !has_repo {
        return Err(REPO_LIST_HINT.to_string());
    }
    if !config
        .repositories
        .iter()
        .any(|item| item.enabled && !item.repository_id.trim().is_empty())
    {
        return Err(ENABLED_HINT.to_string());
    }
    Ok(())
}

pub async fn load_merge_monitor_config(
    state: State<'_, MergeMonitorState>,
) -> Result<MergeMonitorConfig, String> {
    let runtime = state.inner.lock().await;
    Ok(runtime.config.clone())
}

pub async fn save_merge_monitor_config(
    app: AppHandle,
    state: State<'_, MergeMonitorState>,
    mut config: MergeMonitorConfig,
) -> Result<MergeMonitorConfig, String> {
    clamp_config(&mut config);
    save_config_to_disk(&config)?;
    let mut runtime = state.inner.lock().await;
    runtime.config = config.clone();
    append_log(&mut runtime, "info", "配置已保存");
    emit_snapshot(&app, &runtime);
    Ok(config)
}

pub async fn get_merge_monitor_snapshot(
    state: State<'_, MergeMonitorState>,
) -> Result<MergeSnapshot, String> {
    let runtime = state.inner.lock().await;
    Ok(build_snapshot(&runtime))
}

pub async fn clear_merge_monitor_logs(
    app: AppHandle,
    state: State<'_, MergeMonitorState>,
) -> Result<MergeSnapshot, String> {
    let mut runtime = state.inner.lock().await;
    runtime.logs.clear();
    append_log(&mut runtime, "info", "日志已清空");
    let snapshot = build_snapshot(&runtime);
    emit_snapshot(&app, &runtime);
    Ok(snapshot)
}

/// Internal MR candidate used by list filtering / earliest pick.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    project_id: i64,
    local_id: i64,
    author_name: String,
    title: String,
    repo_name: String,
    detail_url: String,
    created_at: String,
}

fn is_ai_review_complete(comments: &[Value]) -> bool {
    comments.iter().any(|c| {
        let name = c
            .pointer("/author/name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = c.get("content").and_then(|v| v.as_str()).unwrap_or("");
        name == "云效AI助手" && (content.contains("代码评审报告") || content.contains('🔎'))
    })
}

fn filter_whitelist_candidates(
    items: &[Candidate],
    allowed_authors: &[String],
    processed: &HashSet<(i64, i64)>,
) -> Vec<Candidate> {
    if allowed_authors.is_empty() {
        return Vec::new();
    }
    let allowed: HashSet<&str> = allowed_authors
        .iter()
        .map(|a| a.trim())
        .filter(|a| !a.is_empty())
        .collect();
    items
        .iter()
        .filter(|c| {
            allowed.contains(c.author_name.trim())
                && !processed.contains(&(c.project_id, c.local_id))
        })
        .cloned()
        .collect()
}

fn parse_created_at(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

fn pick_earliest_by_created_at(candidates: &[Candidate]) -> Option<Candidate> {
    candidates
        .iter()
        .enumerate()
        .min_by(|(i, a), (j, b)| {
            match (parse_created_at(&a.created_at), parse_created_at(&b.created_at)) {
                (Some(ta), Some(tb)) => ta.cmp(&tb).then_with(|| i.cmp(j)),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => i.cmp(j),
            }
        })
        .map(|(_, c)| c.clone())
}

fn cleanup_stale(runtime: &mut MergeRuntime, opened_keys: &HashSet<(i64, i64)>) {
    runtime.processed.retain(|k| opened_keys.contains(k));
    runtime
        .todos
        .retain(|t| opened_keys.contains(&(t.project_id, t.local_id)));
}

/// Skip cleanup when any repo list fetch failed — aggregate `opened_keys` is incomplete
/// and would falsely drop still-opened processed/todos.
fn should_cleanup_stale(any_repo_list_failed: bool) -> bool {
    !any_repo_list_failed
}

fn json_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok()))
        .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()))
}

fn values_as_array(data: &Value) -> Vec<Value> {
    if let Some(arr) = data.as_array() {
        return arr.clone();
    }
    for key in ["result", "data", "list", "items"] {
        if let Some(arr) = data.get(key).and_then(|v| v.as_array()) {
            return arr.clone();
        }
    }
    Vec::new()
}

fn candidate_from_change_request(
    item: &Value,
    fallback_project_id: i64,
    repo_name: &str,
) -> Option<Candidate> {
    let project_id = item
        .get("projectId")
        .and_then(json_i64)
        .unwrap_or(fallback_project_id);
    let local_id = item.get("localId").and_then(json_i64)?;
    let author_name = item
        .pointer("/author/name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let title = item
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let detail_url = item
        .get("detailUrl")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let created_at = item
        .get("createdAt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Some(Candidate {
        project_id,
        local_id,
        author_name,
        title,
        repo_name: repo_name.to_string(),
        detail_url,
        created_at,
    })
}

fn tracked_from_candidate(c: &Candidate) -> TrackedMerge {
    TrackedMerge {
        project_id: c.project_id,
        local_id: c.local_id,
        author_name: c.author_name.clone(),
        title: c.title.clone(),
        repo_name: c.repo_name.clone(),
        detail_url: c.detail_url.clone(),
        created_at: c.created_at.clone(),
    }
}

fn todo_from_tracked(t: &TrackedMerge) -> MergeTodo {
    MergeTodo {
        project_id: t.project_id,
        local_id: t.local_id,
        author_name: t.author_name.clone(),
        title: t.title.clone(),
        repo_name: t.repo_name.clone(),
        detail_url: t.detail_url.clone(),
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

async fn call_open_api(
    http: &Client,
    token: &str,
    path: &str,
    method: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    let url = format!("{OPENAPI_BASE}{path}");
    let builder = match method {
        "POST" => http.post(&url),
        _ => http.get(&url),
    };
    let mut request = builder
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("x-yunxiao-token", token);
    if let Some(payload) = body {
        request = request.json(&payload);
    }

    let response = request
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                "Request timeout".to_string()
            } else {
                format!("Network error: {error}")
            }
        })?;

    let status_code = response.status().as_u16();
    let body_text = response
        .text()
        .await
        .map_err(|error| format!("Read body failed: {error}"))?;

    if !(200..300).contains(&status_code) {
        return Err(format!("HTTP {status_code}, body: {body_text}"));
    }

    serde_json::from_str::<Value>(&body_text).map_err(|error| format!("Invalid JSON: {error}"))
}

async fn list_opened_change_requests(
    http: &Client,
    token: &str,
    org: &str,
    repo_id: &str,
) -> Result<Vec<Value>, String> {
    let path = format!(
        "/oapi/v1/codeup/organizations/{org}/changeRequests?projectIds={repo_id}&state=opened"
    );
    let data = call_open_api(http, token, &path, "GET", None).await?;
    Ok(values_as_array(&data))
}

async fn list_change_request_comments(
    http: &Client,
    token: &str,
    org: &str,
    project_id: i64,
    local_id: i64,
) -> Result<Vec<Value>, String> {
    let path = format!(
        "/oapi/v1/codeup/organizations/{org}/repositories/{project_id}/changeRequests/{local_id}/comments/list"
    );
    let data = call_open_api(http, token, &path, "POST", Some(serde_json::json!({}))).await?;
    Ok(values_as_array(&data))
}

async fn run_monitor_cycle(app: &AppHandle, http: &Client, state: &MergeMonitorState) -> u64 {
    let (running, token, org_id, list_poll, ai_poll, allowed_authors, enabled_repos, current) = {
        let runtime = state.inner.lock().await;
        if !runtime.running {
            return 1;
        }
        let enabled_repos: Vec<(String, String)> = runtime
            .config
            .repositories
            .iter()
            .filter(|r| r.enabled && !r.repository_id.trim().is_empty())
            .map(|r| (r.repository_id.clone(), r.name.clone()))
            .collect();
        (
            runtime.running,
            runtime.config.token.clone(),
            runtime.config.org_id.clone(),
            runtime.config.list_poll_interval_secs.max(5),
            runtime.config.ai_poll_interval_secs.max(3),
            runtime.config.allowed_authors.clone(),
            enabled_repos,
            runtime.current.clone(),
        )
    };

    if !running {
        return 1;
    }

    if token.trim().is_empty() || org_id.trim().is_empty() {
        let mut runtime = state.inner.lock().await;
        if runtime.running {
            append_log(
                &mut runtime,
                "error",
                "Token 或组织 ID 为空，请检查配置后重新启动",
            );
            emit_snapshot(app, &runtime);
        }
        return list_poll;
    }

    // Track AI for current MR.
    if let Some(tracked) = current {
        // Mid-track disappear: if this MR left opened, clear current only.
        match list_opened_change_requests(http, &token, &org_id, &tracked.project_id.to_string())
            .await
        {
            Ok(items) => {
                let still_open = items.iter().any(|item| {
                    let project_id = item
                        .get("projectId")
                        .and_then(json_i64)
                        .unwrap_or(tracked.project_id);
                    let local_id = item.get("localId").and_then(json_i64);
                    local_id == Some(tracked.local_id) && project_id == tracked.project_id
                });
                if !still_open {
                    let mut runtime = state.inner.lock().await;
                    if !runtime.running {
                        return 1;
                    }
                    if runtime
                        .current
                        .as_ref()
                        .map(|c| c.project_id == tracked.project_id && c.local_id == tracked.local_id)
                        .unwrap_or(false)
                    {
                        append_log(
                            &mut runtime,
                            "info",
                            format!(
                                "跟踪中的 !{} 已不在 opened，清除当前跟踪",
                                tracked.local_id
                            ),
                        );
                        runtime.current = None;
                        emit_snapshot(app, &runtime);
                    }
                    return 1;
                }
            }
            Err(error) => {
                let mut runtime = state.inner.lock().await;
                if runtime.running {
                    append_log(
                        &mut runtime,
                        "warn",
                        format!(
                            "检查 !{} opened 状态失败，继续拉评论：{}",
                            tracked.local_id, error
                        ),
                    );
                    emit_snapshot(app, &runtime);
                }
            }
        }

        match list_change_request_comments(
            http,
            &token,
            &org_id,
            tracked.project_id,
            tracked.local_id,
        )
        .await
        {
            Ok(comments) => {
                if is_ai_review_complete(&comments) {
                    // Confirm under lock before notify/write so stop between fetch and
                    // write cannot notify without updating state (or after abandon).
                    let mut runtime = state.inner.lock().await;
                    if !runtime.running {
                        return 1;
                    }
                    if runtime
                        .current
                        .as_ref()
                        .map(|c| c.project_id == tracked.project_id && c.local_id == tracked.local_id)
                        .unwrap_or(false)
                    {
                        crate::system_notify::show_system_notification(
                            app,
                            "合并监控 · AI评审完成",
                            &format!(
                                "{} · {} · {}",
                                tracked.author_name, tracked.title, tracked.repo_name
                            ),
                        );
                        runtime
                            .processed
                            .insert((tracked.project_id, tracked.local_id));
                        runtime.todos.push(todo_from_tracked(&tracked));
                        runtime.current = None;
                        append_log(
                            &mut runtime,
                            "info",
                            format!(
                                "AI评审完成：!{} {} · {}",
                                tracked.local_id, tracked.title, tracked.author_name
                            ),
                        );
                        emit_snapshot(app, &runtime);
                    }
                    return 1;
                }

                return ai_poll;
            }
            Err(error) => {
                let mut runtime = state.inner.lock().await;
                if runtime.running {
                    append_log(
                        &mut runtime,
                        "error",
                        format!("拉取 !{} 评论失败：{}", tracked.local_id, error),
                    );
                    emit_snapshot(app, &runtime);
                }
                return ai_poll;
            }
        }
    }

    // No current: scan opened lists.
    let mut opened_keys = HashSet::new();
    let mut all_candidates = Vec::new();
    let mut repo_summaries = HashMap::new();
    let mut any_repo_list_failed = false;

    for (repo_id, repo_name) in &enabled_repos {
        let fallback_project_id = repo_id.trim().parse::<i64>().unwrap_or(0);
        match list_opened_change_requests(http, &token, &org_id, repo_id.trim()).await {
            Ok(items) => {
                let mut parsed = 0usize;
                for item in &items {
                    if let Some(candidate) =
                        candidate_from_change_request(item, fallback_project_id, repo_name)
                    {
                        opened_keys.insert((candidate.project_id, candidate.local_id));
                        all_candidates.push(candidate);
                        parsed += 1;
                    }
                }
                repo_summaries.insert(repo_id.clone(), format!("opened: {parsed}"));
            }
            Err(error) => {
                any_repo_list_failed = true;
                repo_summaries.insert(repo_id.clone(), format!("错误: {error}"));
                let mut runtime = state.inner.lock().await;
                if runtime.running {
                    append_log(
                        &mut runtime,
                        "error",
                        format!("仓库 {repo_name}({repo_id}) 拉取列表失败：{error}"),
                    );
                    emit_snapshot(app, &runtime);
                }
            }
        }
    }

    let mut runtime = state.inner.lock().await;
    if !runtime.running {
        return 1;
    }

    runtime.repo_summaries = repo_summaries;
    if should_cleanup_stale(any_repo_list_failed) {
        cleanup_stale(&mut runtime, &opened_keys);

        if let Some(tracked) = runtime.current.clone() {
            if !opened_keys.contains(&(tracked.project_id, tracked.local_id)) {
                append_log(
                    &mut runtime,
                    "info",
                    format!(
                        "跟踪中的 !{} 已不在 opened，清除当前跟踪",
                        tracked.local_id
                    ),
                );
                runtime.current = None;
            }
        }
    }

    if runtime.current.is_some() {
        emit_snapshot(app, &runtime);
        return ai_poll;
    }

    let filtered =
        filter_whitelist_candidates(&all_candidates, &allowed_authors, &runtime.processed);
    if let Some(picked) = pick_earliest_by_created_at(&filtered) {
        append_log(
            &mut runtime,
            "info",
            format!(
                "开始跟踪 !{} {} · {} · {}",
                picked.local_id, picked.title, picked.author_name, picked.repo_name
            ),
        );
        runtime.current = Some(tracked_from_candidate(&picked));
        emit_snapshot(app, &runtime);
        return ai_poll;
    }

    emit_snapshot(app, &runtime);
    list_poll
}

pub fn spawn_background(app: AppHandle) {
    let state = app.state::<MergeMonitorState>();
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
            let state = app_handle.state::<MergeMonitorState>();
            let http = state.http_client();
            let sleep_secs = run_monitor_cycle(&app_handle, &http, &state).await;
            tokio::time::sleep(Duration::from_secs(sleep_secs.max(1))).await;
        }
    });
}

pub async fn start_merge_monitor(
    app: AppHandle,
    state: State<'_, MergeMonitorState>,
) -> Result<MergeSnapshot, String> {
    let mut runtime = state.inner.lock().await;
    if runtime.running {
        return Err("合并监控已在运行".to_string());
    }
    validate_merge_monitor_config(&runtime.config)?;
    runtime.running = true;
    append_log(&mut runtime, "info", "合并监控已启动");
    let snapshot = build_snapshot(&runtime);
    emit_snapshot(&app, &runtime);
    Ok(snapshot)
}

pub async fn stop_merge_monitor(
    app: AppHandle,
    state: State<'_, MergeMonitorState>,
) -> Result<MergeSnapshot, String> {
    let mut runtime = state.inner.lock().await;
    runtime.running = false;
    runtime.current = None;
    append_log(&mut runtime, "info", "合并监控已停止");
    let snapshot = build_snapshot(&runtime);
    emit_snapshot(&app, &runtime);
    Ok(snapshot)
}

pub fn open_merge_request_page(app: AppHandle, detail_url: String) -> Result<(), String> {
    let url = detail_url.trim().to_string();
    if url.is_empty() {
        return Err("合并请求链接为空".to_string());
    }
    match app.opener().open_url(url.as_str(), None::<&str>) {
        Ok(()) => Ok(()),
        Err(opener_error) => open_url_fallback(&url)
            .map_err(|fallback_error| format!("打开页面失败：{opener_error}；{fallback_error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dummy_mr(author: &str, local_id: i64) -> Candidate {
        Candidate {
            project_id: 1,
            local_id,
            author_name: author.to_string(),
            title: format!("MR-{local_id}"),
            repo_name: "demo".to_string(),
            detail_url: format!("https://example.com/mr/{local_id}"),
            created_at: "2026-01-02T00:00:00Z".to_string(),
        }
    }

    fn dummy_mr_at(author: &str, local_id: i64, created_at: &str) -> Candidate {
        let mut c = dummy_mr(author, local_id);
        c.created_at = created_at.to_string();
        c
    }

    #[test]
    fn ai_complete_when_assistant_report_present() {
        let comments = vec![json!({
            "author": {"name": "云效AI助手"},
            "content": "## 🔎 代码评审报告\n..."
        })];
        assert!(is_ai_review_complete(&comments));
    }

    #[test]
    fn ai_incomplete_without_assistant() {
        let comments = vec![json!({
            "author": {"name": "张三"},
            "content": "LGTM"
        })];
        assert!(!is_ai_review_complete(&comments));
    }

    #[test]
    fn ai_complete_with_report_text_only() {
        let comments = vec![json!({
            "author": {"name": "云效AI助手"},
            "content": "代码评审报告：通过"
        })];
        assert!(is_ai_review_complete(&comments));
    }

    #[test]
    fn ai_complete_with_emoji_only() {
        let comments = vec![json!({
            "author": {"name": "云效AI助手"},
            "content": "🔎 已完成"
        })];
        assert!(is_ai_review_complete(&comments));
    }

    #[test]
    fn ai_incomplete_assistant_without_markers() {
        let comments = vec![json!({
            "author": {"name": "云效AI助手"},
            "content": "正在评审中"
        })];
        assert!(!is_ai_review_complete(&comments));
    }

    #[test]
    fn empty_whitelist_matches_nobody() {
        let items = vec![dummy_mr("刘锦涛", 1)];
        let out = filter_whitelist_candidates(&items, &[], &HashSet::new());
        assert!(out.is_empty());
    }

    #[test]
    fn whitelist_matches_exact_trimmed_name() {
        let items = vec![
            dummy_mr("刘锦涛", 1),
            dummy_mr(" 刘锦涛 ", 2),
            dummy_mr("其他人", 3),
        ];
        let allowed = vec!["刘锦涛".to_string()];
        let out = filter_whitelist_candidates(&items, &allowed, &HashSet::new());
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].local_id, 1);
        assert_eq!(out[1].local_id, 2);
    }

    #[test]
    fn whitelist_excludes_processed() {
        let items = vec![dummy_mr("刘锦涛", 1), dummy_mr("刘锦涛", 2)];
        let allowed = vec!["刘锦涛".to_string()];
        let mut processed = HashSet::new();
        processed.insert((1, 1));
        let out = filter_whitelist_candidates(&items, &allowed, &processed);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].local_id, 2);
    }

    #[test]
    fn pick_earliest_created_at() {
        let candidates = vec![
            dummy_mr_at("A", 2, "2026-01-03T10:00:00Z"),
            dummy_mr_at("B", 1, "2026-01-01T08:00:00Z"),
            dummy_mr_at("C", 3, "2026-01-02T12:00:00Z"),
        ];
        let picked = pick_earliest_by_created_at(&candidates).expect("should pick");
        assert_eq!(picked.local_id, 1);
        assert_eq!(picked.author_name, "B");
    }

    #[test]
    fn pick_earliest_puts_unparseable_last() {
        let candidates = vec![
            dummy_mr_at("A", 2, "not-a-date"),
            dummy_mr_at("B", 1, "2026-01-01T08:00:00Z"),
        ];
        let picked = pick_earliest_by_created_at(&candidates).expect("should pick");
        assert_eq!(picked.local_id, 1);
    }

    #[test]
    fn pick_earliest_empty_returns_none() {
        assert!(pick_earliest_by_created_at(&[]).is_none());
    }

    #[test]
    fn should_cleanup_when_all_repo_lists_ok() {
        assert!(should_cleanup_stale(false));
    }

    #[test]
    fn should_skip_cleanup_when_any_repo_list_failed() {
        assert!(!should_cleanup_stale(true));
    }

    #[test]
    fn cleanup_removes_missing_from_processed_and_todos() {
        let mut processed = HashSet::new();
        processed.insert((1, 24));
        processed.insert((1, 25));

        let todos = vec![
            MergeTodo {
                project_id: 1,
                local_id: 24,
                author_name: "A".into(),
                title: "old".into(),
                repo_name: "r".into(),
                detail_url: "u".into(),
            },
            MergeTodo {
                project_id: 1,
                local_id: 25,
                author_name: "B".into(),
                title: "keep".into(),
                repo_name: "r".into(),
                detail_url: "u".into(),
            },
        ];

        let mut opened = HashSet::new();
        opened.insert((1, 25));

        let mut runtime = MergeRuntime {
            config: MergeMonitorConfig::default(),
            running: false,
            current: None,
            processed,
            todos,
            logs: VecDeque::new(),
            repo_summaries: HashMap::new(),
        };
        cleanup_stale(&mut runtime, &opened);

        assert!(!runtime.processed.contains(&(1, 24)));
        assert!(runtime.processed.contains(&(1, 25)));
        assert_eq!(runtime.todos.len(), 1);
        assert_eq!(runtime.todos[0].local_id, 25);
    }
}
