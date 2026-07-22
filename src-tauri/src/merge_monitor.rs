use std::{
    collections::{HashSet, VecDeque},
    fs,
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::Duration,
};

use directories::ProjectDirs;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

const CONFIG_FILE_NAME: &str = "yunxiao-merge.json";
const STATE_EVENT: &str = "merge-monitor-state";
const MAX_LOGS: usize = 200;
#[allow(dead_code)]
const OPENAPI_BASE: &str = "https://openapi-rdc.aliyuncs.com";

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
    /// Reserved for the state machine (Task 4); only initialized in this scaffold.
    #[allow(dead_code)]
    processed: HashSet<(i64, i64)>,
    todos: Vec<MergeTodo>,
    logs: VecDeque<LogEntry>,
}

pub struct MergeMonitorState {
    inner: Mutex<MergeRuntime>,
    /// Background loop gate; used when Task 4 adds `spawn_background`.
    #[allow(dead_code)]
    pub loop_started: AtomicBool,
    http: std::sync::Mutex<Client>,
}

impl MergeMonitorState {
    #[allow(dead_code)]
    pub fn http_client(&self) -> Client {
        self.http
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    #[allow(dead_code)]
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
            summary: String::new(),
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
        }),
        loop_started: AtomicBool::new(false),
        http: std::sync::Mutex::new(http),
    }
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

/// Internal MR candidate used by list filtering / earliest pick (wired in Task 4).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
struct Candidate {
    project_id: i64,
    local_id: i64,
    author_name: String,
    title: String,
    repo_name: String,
    detail_url: String,
    created_at: String,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn parse_created_at(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn cleanup_stale(
    processed: &mut HashSet<(i64, i64)>,
    todos: &mut Vec<MergeTodo>,
    opened_keys: &HashSet<(i64, i64)>,
) {
    processed.retain(|k| opened_keys.contains(k));
    todos.retain(|t| opened_keys.contains(&(t.project_id, t.local_id)));
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
    fn cleanup_removes_missing_from_processed_and_todos() {
        let mut processed = HashSet::new();
        processed.insert((1, 24));
        processed.insert((1, 25));

        let mut todos = vec![
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

        cleanup_stale(&mut processed, &mut todos, &opened);

        assert!(!processed.contains(&(1, 24)));
        assert!(processed.contains(&(1, 25)));
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].local_id, 25);
    }
}
