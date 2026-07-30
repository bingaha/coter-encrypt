//! 后道险种订单订阅：配置落盘、按日自动/手动执行 operateOrderIns/secondPageList、结果快照与首页待办摘要。

use std::{collections::BTreeMap, fs, path::PathBuf};

use chrono::{Datelike, NaiveDate};
use directories::ProjectDirs;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tauri::AppHandle;

use crate::http_client;
use crate::system_notify;
use crate::yunsheng_auth;

const CONFIG_FILE_NAME: &str = "order-ins-subscribe.json";
const RESULT_FILE_NAME: &str = "order-ins-subscribe-result.json";
const MANAGE_API_BASE: &str = "https://gateway.yunsheng.cn/shebaotong7-manage-api";
const SECOND_PAGE_LIST_PATH: &str = "/operateOrderIns/secondPageList";

/// 新建订阅默认订单状态：待受理/已受理/反馈中/待审核/受理中。
#[allow(dead_code)] // 供领域测试与前端新建语义对齐；空 orderStates 表示不限，故不挂 serde default
pub fn default_order_states() -> Vec<i32> {
    vec![1, 2, 3, 7, 8]
}

fn default_bill_month_token() -> String {
    "current".to_string()
}

fn default_true() -> bool {
    true
}

/// 规范化账单月令牌：`prev` / `current` / `next` / `YYYYMM`。
pub fn normalize_bill_month_token(raw: &str) -> String {
    let t = raw.trim();
    if is_valid_bill_month(t) {
        return t.to_string();
    }
    match t.to_ascii_lowercase().as_str() {
        "prev" | "previous" | "last" | "上月" => "prev".to_string(),
        "next" | "下月" => "next".to_string(),
        _ => "current".to_string(),
    }
}

fn bill_month_token_delta(token: &str) -> Option<i32> {
    match token {
        "prev" => Some(-1),
        "current" => Some(0),
        "next" => Some(1),
        _ => None,
    }
}

/// 订阅内选中的主体（落盘 id + 名称，供回显）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrgAccountRef {
    pub org_account_id: i64,
    #[serde(default)]
    pub account_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub area_id: i64,
    #[serde(default)]
    pub area_name: String,
    /// 可选多选；空 = 该地区全部主体。
    #[serde(default)]
    pub org_accounts: Vec<OrgAccountRef>,
    /// 起始账单月：`prev` / `current` / `next` / `YYYYMM`；默认当月。
    #[serde(default = "default_bill_month_token")]
    pub bill_month1: String,
    /// 结束账单月：`prev` / `current` / `next` / `YYYYMM`；默认当月。
    #[serde(default = "default_bill_month_token")]
    pub bill_month2: String,
    /// 办理类型：1报增 2在缴 3停缴 4补缴 5特殊补缴；0=未选（必填，无默认勾选）。
    #[serde(default)]
    pub account_status: i32,
    #[serde(default)]
    pub order_states: Vec<i32>,
    #[serde(default)]
    pub ins_codes: Vec<i32>,
}

impl Subscription {
    /// 新建订阅默认值。
    #[allow(dead_code)]
    pub fn new_default() -> Self {
        Self {
            id: generate_subscription_id(),
            enabled: true,
            area_id: 0,
            area_name: String::new(),
            org_accounts: Vec::new(),
            bill_month1: default_bill_month_token(),
            bill_month2: default_bill_month_token(),
            account_status: 0,
            order_states: default_order_states(),
            ins_codes: Vec::new(),
        }
    }

    pub fn org_account_ids(&self) -> Vec<i64> {
        self.org_accounts
            .iter()
            .map(|o| o.org_account_id)
            .filter(|id| *id > 0)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct OrderInsSubscribeConfig {
    /// 启动/进入首页时是否按日自动查询；默认关闭。
    #[serde(default)]
    pub auto_run_on_startup: bool,
    #[serde(default)]
    pub subscriptions: Vec<Subscription>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaOption {
    pub area_id: i64,
    pub area_name: String,
    #[serde(default)]
    pub province_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrgAccountHit {
    pub org_account_id: i64,
    pub account_name: String,
    /// 机构公积金业务账期 YYYYMM 数值，如 202608；无则为 0。
    #[serde(default)]
    pub order_month_gjj: i64,
    /// 机构社保业务账期 YYYYMM 数值；无则为 0。
    #[serde(default)]
    pub order_month_sb: i64,
    /// 由社保/公积金业务月合成的 YYYYMM（仅展示用）。
    #[serde(default)]
    pub business_bill_month: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOrgAccountsRequest {
    pub area_id: i64,
    #[serde(default)]
    pub account_name: String,
    #[serde(default = "default_page_no")]
    pub page_no: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page_no() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

/// 执行结果表固定分组列（顺序即请求与展示顺序）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InsGroupKey {
    /// 公积金
    Gjj,
    /// 医保
    Medical,
    /// 工伤
    Injury,
    /// 采暖
    Heating,
    /// 养老
    Pension,
    /// 失业
    Unemployment,
    /// 其他
    Other,
}

pub const INS_GROUP_ORDER: [InsGroupKey; 7] = [
    InsGroupKey::Gjj,
    InsGroupKey::Medical,
    InsGroupKey::Injury,
    InsGroupKey::Heating,
    InsGroupKey::Pension,
    InsGroupKey::Unemployment,
    InsGroupKey::Other,
];

/// 将险种码映射到业务分组；未知码归「其他」。
pub fn ins_code_group(code: i32) -> InsGroupKey {
    match code {
        20 | 21 => InsGroupKey::Gjj,
        40 | 41 | 42 | 43 | 44 | 45 | 100 | 124 => InsGroupKey::Medical,
        60 | 61 => InsGroupKey::Injury,
        110 => InsGroupKey::Heating,
        30 => InsGroupKey::Pension,
        50 => InsGroupKey::Unemployment,
        _ => InsGroupKey::Other,
    }
}

/// 分组单元格：未选 / 成功计数 / 错误。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum GroupCell {
    Dash,
    Count { value: i64 },
    Error { message: String },
}

impl Default for GroupCell {
    fn default() -> Self {
        Self::Dash
    }
}

/// 七个固定分组列的明细（缺省全为未选，兼容旧快照）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct InsGroupBreakdown {
    #[serde(default)]
    pub gjj: GroupCell,
    #[serde(default)]
    pub medical: GroupCell,
    #[serde(default)]
    pub injury: GroupCell,
    #[serde(default)]
    pub heating: GroupCell,
    #[serde(default)]
    pub pension: GroupCell,
    #[serde(default)]
    pub unemployment: GroupCell,
    #[serde(default)]
    pub other: GroupCell,
}

impl InsGroupBreakdown {
    pub fn set(&mut self, key: InsGroupKey, cell: GroupCell) {
        match key {
            InsGroupKey::Gjj => self.gjj = cell,
            InsGroupKey::Medical => self.medical = cell,
            InsGroupKey::Injury => self.injury = cell,
            InsGroupKey::Heating => self.heating = cell,
            InsGroupKey::Pension => self.pension = cell,
            InsGroupKey::Unemployment => self.unemployment = cell,
            InsGroupKey::Other => self.other = cell,
        }
    }
}

/// 从已选险种中取出某组的码（保持已选顺序，去重由 normalize 保证）。
pub fn selected_codes_in_group(ins_codes: &[i32], group: InsGroupKey) -> Vec<i32> {
    ins_codes
        .iter()
        .copied()
        .filter(|code| ins_code_group(*code) == group)
        .collect()
}

/// 按固定组顺序生成需要发起的查询订阅（覆盖 `ins_codes`）；空筛选返回原订阅一次。
pub fn planned_fetch_subs(sub: &Subscription) -> Vec<Subscription> {
    if sub.ins_codes.is_empty() {
        return vec![sub.clone()];
    }
    INS_GROUP_ORDER
        .iter()
        .filter_map(|group| {
            let codes = selected_codes_in_group(&sub.ins_codes, *group);
            if codes.is_empty() {
                None
            } else {
                let mut clone = sub.clone();
                clone.ins_codes = codes;
                Some(clone)
            }
        })
        .collect()
}

fn fetch_cache_key(sub: &Subscription) -> String {
    let mut codes = sub.ins_codes.clone();
    codes.sort_unstable();
    let codes_part = codes
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("{}|{codes_part}", sub.id)
}

/// 单条订阅的执行结果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionRunResult {
    pub subscription_id: String,
    #[serde(default)]
    pub area_name: String,
    #[serde(default)]
    pub org_count: i64,
    /// 展示用账单月，如 `202607` 或 `202606~202607`
    pub bill_month: String,
    /// 办理类型（accountStatus）
    #[serde(default)]
    pub account_status: i32,
    /// 已选险种编码；空 = 不限
    #[serde(default)]
    pub ins_codes: Vec<i32>,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 成功时 = 响应 `data.total`；有分组筛选时 = 成功组计数之和
    pub subscribed_total: i64,
    /// 分组明细；旧快照缺省为全未选
    #[serde(default)]
    pub group_breakdown: InsGroupBreakdown,
}

/// 一次执行的结果快照（独立落盘）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSnapshot {
    /// 本地执行时间，如 `2026-07-28T14:30:00`
    #[serde(default)]
    pub executed_at: String,
    /// 本地自然日 `YYYY-MM-DD`，供自动执行门控
    #[serde(default)]
    pub executed_date: String,
    #[serde(default)]
    pub total: i64,
    #[serde(default)]
    pub subscriptions: Vec<SubscriptionRunResult>,
}

/// 首页待办摘要中的单一业务来源。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PendingSource {
    pub id: String,
    pub label: String,
    pub count: i64,
}

/// 首页顶栏待办读模型。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HomePendingSummary {
    pub total: i64,
    #[serde(default)]
    pub sources: Vec<PendingSource>,
}

/// 启动/进入首页时「或许自动执行」的返回。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaybeAutoRunOutcome {
    pub did_run: bool,
    pub snapshot: ExecutionSnapshot,
}

const ORDER_INS_SUBSCRIBE_SOURCE_ID: &str = "order-ins-subscribe";
const ORDER_INS_SUBSCRIBE_SOURCE_LABEL: &str = "后道险种订单订阅";

fn generate_subscription_id() -> String {
    let mut rng = rand::thread_rng();
    let n: u64 = rng.gen();
    format!("sub-{n:016x}")
}

fn app_config_dir() -> Result<PathBuf, String> {
    let dirs = ProjectDirs::from("com", "coter", "CoterEncrypt")
        .ok_or_else(|| "无法解析应用配置目录".to_string())?;
    Ok(dirs.config_dir().to_path_buf())
}

fn config_path() -> Result<PathBuf, String> {
    Ok(app_config_dir()?.join(CONFIG_FILE_NAME))
}

fn result_path() -> Result<PathBuf, String> {
    Ok(app_config_dir()?.join(RESULT_FILE_NAME))
}

fn ensure_config_dir(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建配置目录失败: {e}"))?;
    }
    Ok(())
}

pub fn load_order_ins_subscribe_config() -> Result<OrderInsSubscribeConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        let config = OrderInsSubscribeConfig::default();
        save_order_ins_subscribe_config_to_disk(&config)?;
        return Ok(config);
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("读取后道险种订单订阅配置失败: {e}"))?;
    let mut config: OrderInsSubscribeConfig =
        serde_json::from_str(&content).map_err(|e| format!("解析后道险种订单订阅配置失败: {e}"))?;
    normalize_config(&mut config);
    Ok(config)
}

fn save_order_ins_subscribe_config_to_disk(config: &OrderInsSubscribeConfig) -> Result<(), String> {
    let path = config_path()?;
    ensure_config_dir(&path)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化后道险种订单订阅配置失败: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("写入后道险种订单订阅配置失败: {e}"))?;
    Ok(())
}

pub fn save_order_ins_subscribe_config(
    mut config: OrderInsSubscribeConfig,
) -> Result<OrderInsSubscribeConfig, String> {
    normalize_config(&mut config);
    validate_config(&config)?;
    save_order_ins_subscribe_config_to_disk(&config)?;
    Ok(config)
}

/// 仅更新启动自动查询开关（不校验订阅完整性）。
pub fn set_order_ins_subscribe_auto_run(enabled: bool) -> Result<OrderInsSubscribeConfig, String> {
    let mut config = load_order_ins_subscribe_config()?;
    config.auto_run_on_startup = enabled;
    save_order_ins_subscribe_config_to_disk(&config)?;
    Ok(config)
}

/// 规范化订阅配置。
pub fn normalize_config(config: &mut OrderInsSubscribeConfig) {
    for sub in &mut config.subscriptions {
        normalize_subscription(sub);
    }
}

pub fn normalize_subscription(sub: &mut Subscription) {
    sub.id = sub.id.trim().to_string();
    if sub.id.is_empty() {
        sub.id = generate_subscription_id();
    }
    sub.area_name = sub.area_name.trim().to_string();

    let mut seen_org = std::collections::HashSet::new();
    sub.org_accounts = sub
        .org_accounts
        .drain(..)
        .filter_map(|mut org| {
            if org.org_account_id <= 0 {
                return None;
            }
            if !seen_org.insert(org.org_account_id) {
                return None;
            }
            org.account_name = org.account_name.trim().to_string();
            Some(org)
        })
        .collect();

    sub.bill_month1 = normalize_bill_month_token(&sub.bill_month1);
    sub.bill_month2 = normalize_bill_month_token(&sub.bill_month2);

    // 办理类型必选但不预填：非法值归零，由校验拦截。
    if !(1..=5).contains(&sub.account_status) {
        sub.account_status = 0;
    }

    sub.order_states.retain(|s| (1..=8).contains(s) && *s != 6);
    sub.ins_codes.sort_unstable();
    sub.ins_codes.dedup();
}

fn validate_config(config: &OrderInsSubscribeConfig) -> Result<(), String> {
    let today = chrono::Local::now().date_naive();
    for (idx, sub) in config.subscriptions.iter().enumerate() {
        let label = if sub.area_name.is_empty() {
            format!("第 {} 条订阅", idx + 1)
        } else {
            sub.area_name.clone()
        };
        if sub.area_id <= 0 {
            return Err(format!("{label}：请选择地区"));
        }
        if !(1..=5).contains(&sub.account_status) {
            return Err(format!("{label}：请选择办理类型"));
        }
        if let Err(err) = resolve_bill_months(sub, today) {
            return Err(format!("{label}：{err}"));
        }
    }
    Ok(())
}

pub fn is_valid_bill_month(value: &str) -> bool {
    if value.len() != 6 || !value.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let month: u32 = value[4..6].parse().unwrap_or(0);
    (1..=12).contains(&month)
}

/// 从机构业务月字段合成 YYYYMM（取社保/公积金中较大的有效值）。
pub fn business_bill_month_from_org_months(order_month_gjj: i64, order_month_sb: i64) -> String {
    [order_month_gjj, order_month_sb]
        .into_iter()
        .map(|n| n.to_string())
        .filter(|s| is_valid_bill_month(s))
        .max()
        .unwrap_or_default()
}

fn shift_month(now: NaiveDate, delta_months: i32) -> String {
    let mut year = now.year();
    let mut month = now.month() as i32 + delta_months;
    while month <= 0 {
        month += 12;
        year -= 1;
    }
    while month > 12 {
        month -= 12;
        year += 1;
    }
    format!("{year}{month:02}")
}

/// 将单个账单月令牌解析为 YYYYMM。
pub fn resolve_bill_month_token(token: &str, now: NaiveDate) -> Result<String, String> {
    let normalized = normalize_bill_month_token(token);
    if is_valid_bill_month(&normalized) {
        return Ok(normalized);
    }
    match bill_month_token_delta(&normalized) {
        Some(delta) => Ok(shift_month(now, delta)),
        None => Err("账单月无效，请选择上月/当月/下月或指定 YYYYMM".to_string()),
    }
}

/// 解析起止账单月为 (YYYYMM, YYYYMM)；起始不得大于结束。
pub fn resolve_bill_months(sub: &Subscription, now: NaiveDate) -> Result<(String, String), String> {
    let start = resolve_bill_month_token(&sub.bill_month1, now)?;
    let end = resolve_bill_month_token(&sub.bill_month2, now)?;
    if start > end {
        return Err("起始账单月不能大于结束账单月".to_string());
    }
    Ok((start, end))
}

/// 展示用账单月字符串。
pub fn format_bill_month_display(bill_month1: &str, bill_month2: &str) -> String {
    if bill_month1 == bill_month2 {
        bill_month1.to_string()
    } else {
        format!("{bill_month1}~{bill_month2}")
    }
}

/// 构造 operateOrderIns/secondPageList 请求体。
/// 无主体时省略 `orgAccountIds` 键。
pub fn build_second_page_list_body(sub: &Subscription, bill_month1: &str, bill_month2: &str) -> Value {
    let mut map = Map::new();
    map.insert("pageNo".into(), json!(1));
    map.insert("pageSize".into(), json!(20));
    map.insert("cancelFlag".into(), json!(0));
    map.insert("latest".into(), json!(1));
    map.insert("noCheckQuery".into(), json!(0));
    map.insert("accountStatus".into(), json!(sub.account_status));
    map.insert("areaIds".into(), json!([sub.area_id]));
    map.insert("billMonth1".into(), json!(bill_month1));
    map.insert("billMonth2".into(), json!(bill_month2));
    map.insert("billMonths".into(), json!([bill_month1, bill_month2]));
    map.insert("insCode".into(), json!(sub.ins_codes));
    map.insert("orderStateList".into(), json!(sub.order_states));
    map.insert("handleTime".into(), json!([]));
    map.insert("oprtStateList".into(), json!([]));
    map.insert("oprtTimes".into(), json!([]));
    map.insert("handleFlags".into(), json!([]));

    let org_ids = sub.org_account_ids();
    if !org_ids.is_empty() {
        map.insert("orgAccountIds".into(), json!(org_ids));
    }

    Value::Object(map)
}

/// 自动执行门控：从未执行 / 非今日 → 应跑；今日已执行（成败不论）→ 不跑。
pub fn should_auto_run(last_executed_date: Option<&str>, today: NaiveDate) -> bool {
    let Some(raw) = last_executed_date.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    match NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        Ok(date) => date != today,
        Err(_) => true,
    }
}

pub fn should_auto_run_from_snapshot(snapshot: &ExecutionSnapshot, today: NaiveDate) -> bool {
    if snapshot.executed_date.trim().is_empty() && snapshot.executed_at.trim().is_empty() {
        return true;
    }
    if !snapshot.executed_date.trim().is_empty() {
        return should_auto_run(Some(snapshot.executed_date.as_str()), today);
    }
    let prefix = snapshot.executed_at.get(..10).unwrap_or("");
    should_auto_run(Some(prefix), today)
}

/// 从业务成功响应提取 `data.total`。
pub fn extract_data_total(body: &Value) -> i64 {
    body.pointer("/data/total")
        .or_else(|| body.pointer("/data/data/total"))
        .or_else(|| body.get("total"))
        .and_then(value_to_i64)
        .unwrap_or(0)
        .max(0)
}

pub fn aggregate_subscription_success(
    sub: &Subscription,
    bill_month_display: &str,
    total: i64,
) -> SubscriptionRunResult {
    SubscriptionRunResult {
        subscription_id: sub.id.clone(),
        area_name: sub.area_name.clone(),
        org_count: sub.org_accounts.len() as i64,
        bill_month: bill_month_display.to_string(),
        account_status: sub.account_status,
        ins_codes: sub.ins_codes.clone(),
        success: true,
        error: None,
        subscribed_total: total.max(0),
        group_breakdown: InsGroupBreakdown::default(),
    }
}

pub fn aggregate_subscription_error(
    sub: &Subscription,
    bill_month_display: &str,
    error: String,
) -> SubscriptionRunResult {
    SubscriptionRunResult {
        subscription_id: sub.id.clone(),
        area_name: sub.area_name.clone(),
        org_count: sub.org_accounts.len() as i64,
        bill_month: bill_month_display.to_string(),
        account_status: sub.account_status,
        ins_codes: sub.ins_codes.clone(),
        success: false,
        error: Some(error),
        subscribed_total: 0,
        group_breakdown: InsGroupBreakdown::default(),
    }
}

fn apply_group_fetch_result(
    breakdown: &mut InsGroupBreakdown,
    group: InsGroupKey,
    fetched: Result<Value, String>,
) -> Result<i64, String> {
    match fetched {
        Ok(body) => match yunsheng_auth::ensure_yunsheng_business_ok(&body) {
            Ok(()) => {
                let total = extract_data_total(&body).max(0);
                breakdown.set(group, GroupCell::Count { value: total });
                Ok(total)
            }
            Err(err) => {
                breakdown.set(
                    group,
                    GroupCell::Error {
                        message: err.clone(),
                    },
                );
                Err(err)
            }
        },
        Err(err) => {
            breakdown.set(
                group,
                GroupCell::Error {
                    message: err.clone(),
                },
            );
            Err(err)
        }
    }
}

/// 组装快照：总数 = 各成功启用订阅的 subscribed_total 之和。
pub fn build_execution_snapshot(
    executed_at: String,
    executed_date: String,
    results: Vec<SubscriptionRunResult>,
) -> ExecutionSnapshot {
    let total = results
        .iter()
        .filter(|r| r.success)
        .map(|r| r.subscribed_total)
        .sum();
    ExecutionSnapshot {
        executed_at,
        executed_date,
        total,
        subscriptions: results,
    }
}

/// 纯领域：对启用订阅用注入的 secondPageList 响应（或错误）聚合；disabled 跳过。
/// 有险种筛选时按组串行多次调用 fetch（每次 `sub.ins_codes` 仅为该组码）；空筛选调用一次。
/// fetch 签名：(sub, bill_month1, bill_month2) → 响应或错误。
pub fn execute_subscriptions_pure(
    config: &OrderInsSubscribeConfig,
    now: NaiveDate,
    executed_at: &str,
    fetch: &dyn Fn(&Subscription, &str, &str) -> Result<Value, String>,
) -> ExecutionSnapshot {
    let executed_date = now.format("%Y-%m-%d").to_string();
    let mut results = Vec::new();
    for sub in &config.subscriptions {
        if !sub.enabled {
            continue;
        }
        let (bill1, bill2) = match resolve_bill_months(sub, now) {
            Ok(pair) => pair,
            Err(err) => {
                results.push(aggregate_subscription_error(sub, "", err));
                continue;
            }
        };
        let display = format_bill_month_display(&bill1, &bill2);

        if sub.ins_codes.is_empty() {
            match fetch(sub, &bill1, &bill2) {
                Ok(body) => match yunsheng_auth::ensure_yunsheng_business_ok(&body) {
                    Ok(()) => {
                        let total = extract_data_total(&body);
                        results.push(aggregate_subscription_success(sub, &display, total));
                    }
                    Err(err) => {
                        results.push(aggregate_subscription_error(sub, &display, err));
                    }
                },
                Err(err) => {
                    results.push(aggregate_subscription_error(sub, &display, err));
                }
            }
            continue;
        }

        let mut breakdown = InsGroupBreakdown::default();
        let mut subscribed_total: i64 = 0;
        let mut ok_groups = 0usize;
        let mut errors: Vec<String> = Vec::new();

        for group in INS_GROUP_ORDER {
            let codes = selected_codes_in_group(&sub.ins_codes, group);
            if codes.is_empty() {
                continue;
            }
            let mut group_sub = sub.clone();
            group_sub.ins_codes = codes;
            match apply_group_fetch_result(&mut breakdown, group, fetch(&group_sub, &bill1, &bill2))
            {
                Ok(total) => {
                    subscribed_total += total;
                    ok_groups += 1;
                }
                Err(err) => {
                    errors.push(err);
                }
            }
        }

        let success = ok_groups > 0;
        let error = if errors.is_empty() {
            None
        } else {
            Some(errors.join("；"))
        };
        results.push(SubscriptionRunResult {
            subscription_id: sub.id.clone(),
            area_name: sub.area_name.clone(),
            org_count: sub.org_accounts.len() as i64,
            bill_month: display,
            account_status: sub.account_status,
            ins_codes: sub.ins_codes.clone(),
            success,
            error,
            subscribed_total: if success { subscribed_total } else { 0 },
            group_breakdown: breakdown,
        });
    }
    build_execution_snapshot(executed_at.to_string(), executed_date, results)
}

fn value_to_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64().or_else(|| n.as_f64().map(|f| f as i64)),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
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

/// 将 orgAccount/pageList 的单条 record 映射为 OrgAccountHit。
pub fn map_org_record(record: &Value) -> Option<OrgAccountHit> {
    let org_account_id = record
        .get("id")
        .and_then(value_to_i64)
        .filter(|id| *id > 0)?;
    let account_name = record
        .get("accountName")
        .or_else(|| record.get("account_name"))
        .map(value_as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let order_month_gjj = record
        .get("orderMonthGjj")
        .or_else(|| record.get("order_month_gjj"))
        .and_then(value_to_i64)
        .unwrap_or(0);
    let order_month_sb = record
        .get("orderMonthSb")
        .or_else(|| record.get("order_month_sb"))
        .and_then(value_to_i64)
        .unwrap_or(0);
    Some(OrgAccountHit {
        org_account_id,
        account_name,
        order_month_gjj,
        order_month_sb,
        business_bill_month: business_bill_month_from_org_months(order_month_gjj, order_month_sb),
    })
}

/// 从 pageList 响应提取机构命中列表。
pub fn map_org_page_list(body: &Value) -> Vec<OrgAccountHit> {
    let records = extract_records(body);
    records.iter().filter_map(map_org_record).collect()
}

fn extract_records(body: &Value) -> Vec<Value> {
    if let Some(records) = body
        .pointer("/data/records")
        .or_else(|| body.pointer("/data/data/records"))
        .or_else(|| body.pointer("/records"))
        .and_then(|v| v.as_array())
    {
        return records.clone();
    }
    if let Some(data) = body.get("data") {
        if let Some(arr) = data.as_array() {
            return arr.clone();
        }
    }
    Vec::new()
}

/// 从 areaSetting/selectList 响应扁平化出地区选项。
pub fn flatten_area_options(body: &Value) -> Vec<AreaOption> {
    let mut out = Vec::new();
    let root = body.get("data").cloned().unwrap_or_else(|| body.clone());
    walk_areas(&root, "", &mut out);
    let mut seen = std::collections::HashSet::new();
    out.retain(|a| seen.insert(a.area_id));
    out
}

fn walk_areas(node: &Value, province_hint: &str, out: &mut Vec<AreaOption>) {
    match node {
        Value::Array(arr) => {
            for item in arr {
                walk_areas(item, province_hint, out);
            }
        }
        Value::Object(map) => {
            let province_name = map
                .get("provinceName")
                .or_else(|| map.get("province_name"))
                .or_else(|| map.get("name"))
                .map(value_as_str)
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| province_hint.to_string());

            let area_id = map
                .get("areaId")
                .or_else(|| map.get("area_id"))
                .and_then(value_to_i64);
            let area_name = map
                .get("areaName")
                .or_else(|| map.get("area_name"))
                .map(value_as_str)
                .unwrap_or_default();

            if let Some(area_id) = area_id {
                if area_id > 0 && !area_name.is_empty() {
                    out.push(AreaOption {
                        area_id,
                        area_name,
                        province_name: province_name.clone(),
                    });
                }
            }

            for key in ["areas", "children", "list", "areaList", "cityList"] {
                if let Some(child) = map.get(key) {
                    walk_areas(child, &province_name, out);
                }
            }
            if area_id.is_none() {
                for (key, child) in map {
                    if matches!(
                        key.as_str(),
                        "areas" | "children" | "list" | "areaList" | "cityList"
                    ) {
                        continue;
                    }
                    if child.is_array() {
                        walk_areas(child, &province_name, out);
                    }
                }
            }
        }
        _ => {}
    }
}

async fn post_manage_api(
    app: Option<&AppHandle>,
    path: &str,
    body: Value,
) -> Result<Value, String> {
    let proxy = http_client::load_http_proxy_config().unwrap_or_default();
    let client = yunsheng_auth::build_yunsheng_http_client(&proxy)?;
    let url = format!("{MANAGE_API_BASE}{path}");

    yunsheng_auth::with_auth_retry(app, |cookies| {
        let client = client.clone();
        let url = url.clone();
        let body = body.clone();
        async move {
            let builder = client.post(&url).json(&body);
            let builder = yunsheng_auth::apply_auth_headers(builder, &cookies);
            let response = builder
                .send()
                .await
                .map_err(|e| format!("请求云生接口失败: {e}"))?;
            let status = response.status().as_u16();
            let text = response
                .text()
                .await
                .map_err(|e| format!("读取云生接口响应失败: {e}"))?;
            let parsed: Value =
                serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));
            Ok((status, parsed))
        }
    })
    .await
}

pub async fn list_order_ins_subscribe_areas(app: &AppHandle) -> Result<Vec<AreaOption>, String> {
    let body = post_manage_api(Some(app), "/areaSetting/selectList", json!({})).await?;
    Ok(flatten_area_options(&body))
}

pub async fn search_order_ins_subscribe_orgs(
    app: &AppHandle,
    request: SearchOrgAccountsRequest,
) -> Result<Vec<OrgAccountHit>, String> {
    if request.area_id <= 0 {
        return Err("请先选择地区".to_string());
    }
    let page_no = if request.page_no == 0 {
        1
    } else {
        request.page_no
    };
    let page_size = if request.page_size == 0 {
        20
    } else {
        request.page_size.min(100)
    };
    let body = post_manage_api(
        Some(app),
        "/orgAccount/pageList",
        json!({
            "pageNo": page_no,
            "pageSize": page_size,
            "authFlag": 1,
            "areaId": request.area_id,
            "accountName": request.account_name.trim(),
        }),
    )
    .await?;
    Ok(map_org_page_list(&body))
}

pub fn load_order_ins_subscribe_result() -> Result<ExecutionSnapshot, String> {
    let path = result_path()?;
    if !path.exists() {
        return Ok(ExecutionSnapshot::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("读取后道险种订单执行结果失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析后道险种订单执行结果失败: {e}"))
}

/// 删除落盘的执行结果快照（首页待办随之归零）。
pub fn clear_order_ins_subscribe_result() -> Result<(), String> {
    let path = result_path()?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("删除后道险种订单执行结果失败: {e}"))?;
    }
    Ok(())
}

fn save_order_ins_subscribe_result_to_disk(snapshot: &ExecutionSnapshot) -> Result<(), String> {
    let path = result_path()?;
    ensure_config_dir(&path)?;
    let content = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("序列化后道险种订单执行结果失败: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("写入后道险种订单执行结果失败: {e}"))?;
    Ok(())
}

/// 手动「立即执行」：对所有启用订阅按组串行查 secondPageList 并覆盖结果快照。
pub async fn run_order_ins_subscribe_now(app: &AppHandle) -> Result<ExecutionSnapshot, String> {
    let config = load_order_ins_subscribe_config()?;
    let now = chrono::Local::now();
    let today = now.date_naive();
    let executed_at = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut responses: BTreeMap<String, Result<Value, String>> = BTreeMap::new();
    for sub in &config.subscriptions {
        if !sub.enabled {
            continue;
        }
        let (bill1, bill2) = match resolve_bill_months(sub, today) {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        for fetch_sub in planned_fetch_subs(sub) {
            let key = fetch_cache_key(&fetch_sub);
            if responses.contains_key(&key) {
                continue;
            }
            let body = build_second_page_list_body(&fetch_sub, &bill1, &bill2);
            responses.insert(
                key,
                post_manage_api(Some(app), SECOND_PAGE_LIST_PATH, body).await,
            );
        }
    }

    let snapshot =
        execute_subscriptions_pure(&config, today, &executed_at, &|sub, _b1, _b2| {
            responses
                .get(&fetch_cache_key(sub))
                .cloned()
                .unwrap_or_else(|| Err("内部错误：缺少订阅查询结果".to_string()))
        });
    save_order_ins_subscribe_result_to_disk(&snapshot)?;
    Ok(snapshot)
}

/// 由快照构造首页待办摘要。
pub fn pending_summary_from_snapshot(snapshot: &ExecutionSnapshot) -> HomePendingSummary {
    let count = snapshot.total.max(0);
    HomePendingSummary {
        total: count,
        sources: vec![PendingSource {
            id: ORDER_INS_SUBSCRIBE_SOURCE_ID.to_string(),
            label: ORDER_INS_SUBSCRIBE_SOURCE_LABEL.to_string(),
            count,
        }],
    }
}

/// 读取上次快照并返回首页待办摘要（不触发网络）。
pub fn get_home_pending_summary() -> Result<HomePendingSummary, String> {
    let snapshot = load_order_ins_subscribe_result()?;
    Ok(pending_summary_from_snapshot(&snapshot))
}

/// 启动/进入首页：仅当配置开启「启动自动查询」且本地自然日门控通过时才执行。
pub async fn maybe_auto_run_order_ins_subscribe(
    app: &AppHandle,
) -> Result<MaybeAutoRunOutcome, String> {
    let snapshot = load_order_ins_subscribe_result()?;
    let config = load_order_ins_subscribe_config()?;
    if !config.auto_run_on_startup {
        return Ok(MaybeAutoRunOutcome {
            did_run: false,
            snapshot,
        });
    }
    let today = chrono::Local::now().date_naive();
    if !should_auto_run_from_snapshot(&snapshot, today) {
        return Ok(MaybeAutoRunOutcome {
            did_run: false,
            snapshot,
        });
    }
    match run_order_ins_subscribe_now(app).await {
        Ok(snapshot) => {
            let failed = snapshot
                .subscriptions
                .iter()
                .filter(|item| !item.success)
                .count();
            let title = "后道险种订单订阅 · 启动查询完成";
            let body = if failed > 0 {
                format!("待办总数 {} · {} 条订阅失败", snapshot.total, failed)
            } else {
                format!("查询成功 · 待办总数 {}", snapshot.total)
            };
            let _ = system_notify::show_system_notification(app, title, &body);
            Ok(MaybeAutoRunOutcome {
                did_run: true,
                snapshot,
            })
        }
        Err(err) => {
            let _ = system_notify::show_system_notification(
                app,
                "后道险种订单订阅 · 启动查询失败",
                &err,
            );
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_order_states_are_pending_work() {
        assert_eq!(default_order_states(), vec![1, 2, 3, 7, 8]);
    }

    #[test]
    fn new_subscription_uses_spec_defaults() {
        let sub = Subscription::new_default();
        assert_eq!(sub.order_states, vec![1, 2, 3, 7, 8]);
        assert!(sub.enabled);
        assert_eq!(sub.bill_month1, "current");
        assert_eq!(sub.bill_month2, "current");
        assert_eq!(sub.account_status, 0);
        assert!(sub.org_accounts.is_empty());
        assert!(sub.ins_codes.is_empty());
        assert!(!sub.id.is_empty());
    }

    #[test]
    fn normalize_bill_month_tokens_and_account_status() {
        let mut sub = Subscription::new_default();
        sub.bill_month1 = "上月".into();
        sub.bill_month2 = "202601".into();
        sub.account_status = 99;
        normalize_subscription(&mut sub);
        assert_eq!(sub.bill_month1, "prev");
        assert_eq!(sub.bill_month2, "202601");
        assert_eq!(sub.account_status, 0);

        sub.bill_month1 = "NEXT".into();
        normalize_subscription(&mut sub);
        assert_eq!(sub.bill_month1, "next");

        sub.account_status = 3;
        normalize_subscription(&mut sub);
        assert_eq!(sub.account_status, 3);
    }

    #[test]
    fn normalize_config_dedupes_ins_codes() {
        let mut config = OrderInsSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![{
                let mut s = Subscription::new_default();
                s.ins_codes = vec![20, 30, 20, 21];
                s
            }],
        };
        normalize_config(&mut config);
        assert_eq!(config.subscriptions[0].ins_codes, vec![20, 21, 30]);
    }

    #[test]
    fn map_org_record_maps_id_to_org_account_id() {
        let record = json!({
            "id": 12345,
            "accountName": "某某人力资源有限公司",
            "orderMonthGjj": 202608,
            "orderMonthSb": 202607
        });
        let hit = map_org_record(&record).expect("should map");
        assert_eq!(hit.org_account_id, 12345);
        assert_eq!(hit.account_name, "某某人力资源有限公司");
        assert_eq!(hit.order_month_gjj, 202608);
        assert_eq!(hit.order_month_sb, 202607);
    }

    #[test]
    fn map_org_record_accepts_string_id() {
        let record = json!({
            "id": "99001",
            "accountName": "测试主体"
        });
        let hit = map_org_record(&record).expect("string id");
        assert_eq!(hit.org_account_id, 99001);
    }

    #[test]
    fn map_org_record_skips_missing_id() {
        let record = json!({ "accountName": "无 id" });
        assert!(map_org_record(&record).is_none());
    }

    #[test]
    fn map_org_page_list_reads_data_records() {
        let body = json!({
            "code": 0,
            "data": {
                "records": [
                    { "id": 1, "accountName": "甲公司" },
                    { "id": 2, "accountName": "乙公司" },
                    { "accountName": "无 id 跳过" }
                ],
                "total": 3
            }
        });
        let hits = map_org_page_list(&body);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].org_account_id, 1);
        assert_eq!(hits[1].account_name, "乙公司");
    }

    #[test]
    fn flatten_area_options_from_province_groups() {
        let body = json!({
            "data": [
                {
                    "provinceName": "山东省",
                    "areas": [
                        { "areaId": 85, "areaName": "济南市" },
                        { "areaId": 86, "areaName": "青岛市" }
                    ]
                },
                {
                    "provinceName": "山西省",
                    "areas": [
                        { "areaId": 290, "areaName": "运城市" }
                    ]
                }
            ]
        });
        let areas = flatten_area_options(&body);
        assert_eq!(areas.len(), 3);
        assert_eq!(areas[0].area_id, 85);
        assert_eq!(areas[0].area_name, "济南市");
        assert_eq!(areas[0].province_name, "山东省");
        assert_eq!(areas[2].area_id, 290);
        assert_eq!(areas[2].area_name, "运城市");
    }

    #[test]
    fn is_valid_bill_month_checks_yyyymm() {
        assert!(is_valid_bill_month("202607"));
        assert!(!is_valid_bill_month("202613"));
        assert!(!is_valid_bill_month("20267"));
        assert!(!is_valid_bill_month("abcdef"));
    }

    #[test]
    fn validate_allows_empty_org_and_rejects_bad_bill_months() {
        let mut config = OrderInsSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![{
                let mut s = Subscription::new_default();
                s.org_accounts.clear();
                s.area_id = 85;
                s.area_name = "济南市".into();
                s.account_status = 3;
                s
            }],
        };
        assert!(validate_config(&config).is_ok());

        config.subscriptions[0].account_status = 0;
        let err = validate_config(&config).expect_err("need account status");
        assert!(err.contains("办理类型"));

        config.subscriptions[0].account_status = 3;
        config.subscriptions[0].bill_month1 = "202608".into();
        config.subscriptions[0].bill_month2 = "202607".into();
        let err = validate_config(&config).expect_err("start > end");
        assert!(err.contains("起始账单月"));

        config.subscriptions[0].bill_month1 = "next".into();
        config.subscriptions[0].bill_month2 = "prev".into();
        let err = validate_config(&config).expect_err("next > prev");
        assert!(err.contains("起始账单月"));
    }

    #[test]
    fn subscription_serde_camel_case_roundtrip_no_biz_types() {
        let sub = Subscription {
            id: "sub-1".into(),
            enabled: false,
            area_id: 85,
            area_name: "济南市".into(),
            org_accounts: vec![OrgAccountRef {
                org_account_id: 42,
                account_name: "Acme".into(),
            }],
            bill_month1: "prev".into(),
            bill_month2: "202607".into(),
            account_status: 3,
            order_states: default_order_states(),
            ins_codes: vec![20, 30],
        };
        let value = serde_json::to_value(&sub).unwrap();
        assert_eq!(value["orgAccounts"][0]["orgAccountId"], 42);
        assert_eq!(value["billMonth1"], "prev");
        assert_eq!(value["billMonth2"], "202607");
        assert_eq!(value["accountStatus"], 3);
        assert_eq!(value["orderStates"], json!([1, 2, 3, 7, 8]));
        assert!(value.get("bizTypes").is_none());
        assert!(value.get("billMonthMode").is_none());
        assert!(value.get("billMonth").is_none());
        let back: Subscription = serde_json::from_value(value).unwrap();
        assert_eq!(back, sub);
    }

    fn sample_sub(id: &str, org_id: Option<i64>) -> Subscription {
        let mut s = Subscription::new_default();
        s.id = id.into();
        s.area_id = 85;
        s.area_name = "济南市".into();
        if let Some(org_id) = org_id {
            s.org_accounts = vec![OrgAccountRef {
                org_account_id: org_id,
                account_name: format!("主体{org_id}"),
            }];
        }
        s
    }

    fn second_page_fixture(total: i64) -> Value {
        json!({
            "code": 0,
            "data": {
                "records": [
                    { "empName": "张三", "insCode": 20, "orderState": 1 }
                ],
                "total": total
            }
        })
    }

    #[test]
    fn resolve_bill_months_rel_and_fixed() {
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        let mut sub = sample_sub("a", None);
        sub.bill_month1 = "current".into();
        sub.bill_month2 = "current".into();
        assert_eq!(
            resolve_bill_months(&sub, now).unwrap(),
            ("202607".into(), "202607".into())
        );

        sub.bill_month1 = "prev".into();
        sub.bill_month2 = "current".into();
        assert_eq!(
            resolve_bill_months(&sub, now).unwrap(),
            ("202606".into(), "202607".into())
        );

        sub.bill_month1 = "prev".into();
        sub.bill_month2 = "next".into();
        assert_eq!(
            resolve_bill_months(&sub, now).unwrap(),
            ("202606".into(), "202608".into())
        );

        let jan = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        sub.bill_month1 = "prev".into();
        sub.bill_month2 = "current".into();
        assert_eq!(
            resolve_bill_months(&sub, jan).unwrap(),
            ("202512".into(), "202601".into())
        );

        sub.bill_month1 = "202601".into();
        sub.bill_month2 = "202603".into();
        assert_eq!(
            resolve_bill_months(&sub, now).unwrap(),
            ("202601".into(), "202603".into())
        );

        sub.bill_month1 = "next".into();
        sub.bill_month2 = "202607".into();
        assert!(resolve_bill_months(&sub, now).unwrap_err().contains("起始账单月"));
    }

    #[test]
    fn business_bill_month_from_org_months_picks_max_valid() {
        assert_eq!(
            business_bill_month_from_org_months(202607, 202608),
            "202608"
        );
        assert_eq!(business_bill_month_from_org_months(0, 0), "");
        assert_eq!(
            business_bill_month_from_org_months(202613, 202607),
            "202607"
        );
    }

    #[test]
    fn build_body_with_orgs_and_omits_when_empty() {
        let mut sub = sample_sub("s1", Some(11));
        sub.org_accounts = vec![
            OrgAccountRef {
                org_account_id: 11,
                account_name: "甲".into(),
            },
            OrgAccountRef {
                org_account_id: 22,
                account_name: "乙".into(),
            },
        ];
        sub.account_status = 3;
        sub.order_states = vec![1, 2];
        sub.ins_codes = vec![20];
        let body = build_second_page_list_body(&sub, "202607", "202607");
        assert_eq!(body["pageNo"], 1);
        assert_eq!(body["cancelFlag"], 0);
        assert_eq!(body["latest"], 1);
        assert_eq!(body["noCheckQuery"], 0);
        assert_eq!(body["accountStatus"], 3);
        assert_eq!(body["areaIds"], json!([85]));
        assert_eq!(body["billMonth1"], "202607");
        assert_eq!(body["billMonth2"], "202607");
        assert_eq!(body["billMonths"], json!(["202607", "202607"]));
        assert_eq!(body["insCode"], json!([20]));
        assert_eq!(body["orderStateList"], json!([1, 2]));
        assert_eq!(body["orgAccountIds"], json!([11, 22]));
        assert_eq!(body["handleTime"], json!([]));
        assert_eq!(body["oprtStateList"], json!([]));
        assert_eq!(body["oprtTimes"], json!([]));
        assert_eq!(body["handleFlags"], json!([]));
        assert!(body.get("bizTypes").is_none());
        assert!(body.get("orderStates").is_none());
        assert!(body.get("insCodes").is_none());

        let empty_org = sample_sub("s2", None);
        let body2 = build_second_page_list_body(&empty_org, "202601", "202602");
        assert!(body2.get("orgAccountIds").is_none());
        assert_eq!(body2["areaIds"], json!([85]));
        assert_eq!(body2["billMonths"], json!(["202601", "202602"]));
    }

    #[test]
    fn should_auto_run_never_yesterday_today() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        assert!(should_auto_run(None, today));
        assert!(should_auto_run(Some(""), today));
        assert!(should_auto_run(Some("2026-07-27"), today));
        assert!(!should_auto_run(Some("2026-07-28"), today));

        let failed_today = ExecutionSnapshot {
            executed_at: "2026-07-28T09:00:00".into(),
            executed_date: "2026-07-28".into(),
            total: 0,
            subscriptions: vec![aggregate_subscription_error(
                &sample_sub("x", Some(1)),
                "202607",
                "token 失效".into(),
            )],
        };
        assert!(!should_auto_run_from_snapshot(&failed_today, today));

        let yesterday = ExecutionSnapshot {
            executed_at: "2026-07-27T23:00:00".into(),
            executed_date: "2026-07-27".into(),
            total: 10,
            subscriptions: vec![],
        };
        assert!(should_auto_run_from_snapshot(&yesterday, today));

        let never = ExecutionSnapshot::default();
        assert!(should_auto_run_from_snapshot(&never, today));
    }

    #[test]
    fn auto_run_on_startup_defaults_false() {
        let config = OrderInsSubscribeConfig::default();
        assert!(!config.auto_run_on_startup);
    }

    #[test]
    fn pending_summary_uses_new_source_id() {
        let snapshot = ExecutionSnapshot {
            executed_at: "2026-07-28T10:00:00".into(),
            executed_date: "2026-07-28".into(),
            total: 42,
            subscriptions: vec![],
        };
        let summary = pending_summary_from_snapshot(&snapshot);
        assert_eq!(summary.total, 42);
        assert_eq!(summary.sources.len(), 1);
        assert_eq!(summary.sources[0].id, "order-ins-subscribe");
        assert_eq!(summary.sources[0].label, "后道险种订单订阅");
        assert_eq!(summary.sources[0].count, 42);
    }

    #[test]
    fn extract_data_total_from_response() {
        assert_eq!(extract_data_total(&second_page_fixture(128)), 128);
        assert_eq!(
            extract_data_total(&json!({ "code": 0, "data": { "records": [], "total": 0 } })),
            0
        );
    }

    #[test]
    fn execute_pure_rejects_auth_failure_body_as_error() {
        let sub = sample_sub("auth-bad", Some(1));
        let config = OrderInsSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![sub],
        };
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        let auth_fail = json!({
            "code": 30000,
            "message": "非法访问,没有认证",
            "status": false
        });
        let snapshot = execute_subscriptions_pure(
            &config,
            now,
            "2026-07-28T15:00:00",
            &|_sub, _b1, _b2| Ok(auth_fail.clone()),
        );
        assert_eq!(snapshot.subscriptions.len(), 1);
        let row = &snapshot.subscriptions[0];
        assert!(!row.success);
        assert_eq!(row.subscribed_total, 0);
        assert_eq!(snapshot.total, 0);
        let err = row.error.as_deref().unwrap_or("");
        assert!(
            err.contains("登录已失效") || err.contains("没有认证") || err.contains("30000"),
            "unexpected err: {err}"
        );
    }

    #[test]
    fn execute_pure_uses_data_total_skips_disabled_keeps_partial() {
        let enabled = sample_sub("ok", None);
        let mut disabled = sample_sub("off", Some(2));
        disabled.enabled = false;
        let failing = sample_sub("bad", Some(3));

        let config = OrderInsSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![enabled, disabled, failing],
        };
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();

        let snapshot = execute_subscriptions_pure(
            &config,
            now,
            "2026-07-28T12:00:00",
            &|sub, bill1, bill2| {
                assert_eq!(bill1, "202607");
                assert_eq!(bill2, "202607");
                if sub.id == "bad" {
                    Err("模拟失败".into())
                } else {
                    Ok(second_page_fixture(15))
                }
            },
        );

        assert_eq!(snapshot.subscriptions.len(), 2);
        assert!(snapshot
            .subscriptions
            .iter()
            .all(|r| r.subscription_id != "off"));
        let ok = snapshot
            .subscriptions
            .iter()
            .find(|r| r.subscription_id == "ok")
            .unwrap();
        assert!(ok.success);
        assert_eq!(ok.subscribed_total, 15);
        assert_eq!(ok.org_count, 0);
        let bad = snapshot
            .subscriptions
            .iter()
            .find(|r| r.subscription_id == "bad")
            .unwrap();
        assert!(!bad.success);
        assert_eq!(bad.error.as_deref(), Some("模拟失败"));
        assert_eq!(bad.subscribed_total, 0);
        assert_eq!(snapshot.total, 15);
        assert_eq!(snapshot.executed_date, "2026-07-28");
    }

    #[test]
    fn multi_subscription_totals_sum_enabled_success() {
        let a = sample_sub("a", None);
        let b = sample_sub("b", Some(2));
        let config = OrderInsSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![a, b],
        };
        let now = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let snapshot = execute_subscriptions_pure(
            &config,
            now,
            "2026-07-01T08:00:00",
            &|sub, _, _| {
                if sub.id == "a" {
                    Ok(second_page_fixture(7))
                } else {
                    Ok(second_page_fixture(5))
                }
            },
        );
        assert_eq!(snapshot.total, 12);
    }

    #[test]
    fn ins_code_group_mapping_matches_spec() {
        assert_eq!(ins_code_group(20), InsGroupKey::Gjj);
        assert_eq!(ins_code_group(21), InsGroupKey::Gjj);
        assert_eq!(ins_code_group(30), InsGroupKey::Pension);
        assert_eq!(ins_code_group(40), InsGroupKey::Medical);
        assert_eq!(ins_code_group(100), InsGroupKey::Medical);
        assert_eq!(ins_code_group(124), InsGroupKey::Medical);
        assert_eq!(ins_code_group(60), InsGroupKey::Injury);
        assert_eq!(ins_code_group(61), InsGroupKey::Injury);
        assert_eq!(ins_code_group(110), InsGroupKey::Heating);
        assert_eq!(ins_code_group(50), InsGroupKey::Unemployment);
        assert_eq!(ins_code_group(70), InsGroupKey::Other);
        assert_eq!(ins_code_group(9999), InsGroupKey::Other);
    }

    #[test]
    fn planned_fetch_subs_splits_by_group_order() {
        let mut sub = sample_sub("g", None);
        sub.ins_codes = vec![30, 20, 21, 50, 70];
        let planned = planned_fetch_subs(&sub);
        assert_eq!(planned.len(), 4);
        // ORDER: Gjj → Medical → Injury → Heating → Pension → Unemployment → Other
        assert_eq!(planned[0].ins_codes, vec![20, 21]);
        assert_eq!(planned[1].ins_codes, vec![30]);
        assert_eq!(planned[2].ins_codes, vec![50]);
        assert_eq!(planned[3].ins_codes, vec![70]);

        let empty = sample_sub("e", None);
        let once = planned_fetch_subs(&empty);
        assert_eq!(once.len(), 1);
        assert!(once[0].ins_codes.is_empty());
    }

    #[test]
    fn execute_pure_unlimited_keeps_group_dash() {
        let sub = sample_sub("u", None);
        let config = OrderInsSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![sub],
        };
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        let snapshot = execute_subscriptions_pure(
            &config,
            now,
            "2026-07-28T12:00:00",
            &|_, _, _| Ok(second_page_fixture(9)),
        );
        let row = &snapshot.subscriptions[0];
        assert!(row.success);
        assert_eq!(row.subscribed_total, 9);
        assert_eq!(row.group_breakdown.gjj, GroupCell::Dash);
        assert_eq!(row.group_breakdown.medical, GroupCell::Dash);
        assert_eq!(row.group_breakdown.pension, GroupCell::Dash);
    }

    #[test]
    fn execute_pure_groups_requests_and_sums_success_cells() {
        let mut sub = sample_sub("g", None);
        sub.ins_codes = vec![20, 21, 40, 30]; // gjj + medical + pension; no 失业
        let config = OrderInsSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![sub],
        };
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        let seen = std::cell::RefCell::new(Vec::<Vec<i32>>::new());
        let snapshot = execute_subscriptions_pure(
            &config,
            now,
            "2026-07-28T12:00:00",
            &|s, _, _| {
                seen.borrow_mut().push(s.ins_codes.clone());
                let total = match s.ins_codes.as_slice() {
                    [20, 21] => 3,
                    [40] => 0,
                    [30] => 5,
                    other => panic!("unexpected codes: {other:?}"),
                };
                Ok(second_page_fixture(total))
            },
        );
        assert_eq!(
            seen.into_inner(),
            vec![vec![20, 21], vec![40], vec![30]]
        );
        let row = &snapshot.subscriptions[0];
        assert!(row.success);
        assert_eq!(row.subscribed_total, 8);
        assert_eq!(row.group_breakdown.gjj, GroupCell::Count { value: 3 });
        assert_eq!(row.group_breakdown.medical, GroupCell::Count { value: 0 });
        assert_eq!(row.group_breakdown.pension, GroupCell::Count { value: 5 });
        assert_eq!(row.group_breakdown.unemployment, GroupCell::Dash);
        assert_eq!(row.group_breakdown.injury, GroupCell::Dash);
        assert_eq!(snapshot.total, 8);
    }

    #[test]
    fn execute_pure_partial_group_failure_keeps_success_totals() {
        let mut sub = sample_sub("p", None);
        sub.ins_codes = vec![20, 30];
        let config = OrderInsSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![sub],
        };
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        let snapshot = execute_subscriptions_pure(
            &config,
            now,
            "2026-07-28T12:00:00",
            &|s, _, _| {
                if s.ins_codes == vec![30] {
                    Err("养老失败".into())
                } else {
                    Ok(second_page_fixture(4))
                }
            },
        );
        let row = &snapshot.subscriptions[0];
        assert!(row.success);
        assert_eq!(row.subscribed_total, 4);
        assert_eq!(row.group_breakdown.gjj, GroupCell::Count { value: 4 });
        assert_eq!(
            row.group_breakdown.pension,
            GroupCell::Error {
                message: "养老失败".into()
            }
        );
        assert!(row.error.as_deref().unwrap_or("").contains("养老失败"));
        assert_eq!(snapshot.total, 4);
    }

    #[test]
    fn execute_pure_all_groups_fail_marks_row_failed() {
        let mut sub = sample_sub("f", None);
        sub.ins_codes = vec![20, 50];
        let config = OrderInsSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![sub],
        };
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        let snapshot = execute_subscriptions_pure(
            &config,
            now,
            "2026-07-28T12:00:00",
            &|_, _, _| Err("全挂".into()),
        );
        let row = &snapshot.subscriptions[0];
        assert!(!row.success);
        assert_eq!(row.subscribed_total, 0);
        assert_eq!(snapshot.total, 0);
        assert!(matches!(
            &row.group_breakdown.gjj,
            GroupCell::Error { message } if message == "全挂"
        ));
        assert!(matches!(
            &row.group_breakdown.unemployment,
            GroupCell::Error { message } if message == "全挂"
        ));
    }

    #[test]
    fn legacy_result_json_defaults_group_breakdown_to_dash() {
        let raw = json!({
            "subscriptionId": "old",
            "areaName": "济南市",
            "orgCount": 0,
            "billMonth": "202607",
            "accountStatus": 3,
            "insCodes": [20],
            "success": true,
            "subscribedTotal": 11
        });
        let row: SubscriptionRunResult = serde_json::from_value(raw).unwrap();
        assert_eq!(row.subscribed_total, 11);
        assert_eq!(row.group_breakdown, InsGroupBreakdown::default());
        assert_eq!(row.group_breakdown.gjj, GroupCell::Dash);
    }
}
