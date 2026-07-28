//! 后道订单订阅：配置落盘、按日自动/手动执行 operateOrder/pageList、结果快照与首页待办摘要。

use std::{collections::BTreeMap, fs, path::PathBuf};

use chrono::NaiveDate;
use directories::ProjectDirs;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;

use crate::http_client;
use crate::system_notify;
use crate::yunsheng_auth;

const CONFIG_FILE_NAME: &str = "order-subscribe.json";
const RESULT_FILE_NAME: &str = "order-subscribe-result.json";
const MANAGE_API_BASE: &str = "https://gateway.yunsheng.cn/shebaotong7-manage-api";

/// 可配置且可展示的业务类型（不含 keep/在缴）。
pub const DISPLAY_BIZ_TYPES: &[&str] = &[
    "sbAdd", "sbFill", "sbStop", "gjjAdd", "gjjFill", "gjjStop",
];

const ALLOWED_BIZ_TYPES: &[&str] = DISPLAY_BIZ_TYPES;

/// 新建订阅默认订单状态：待受理/已受理/反馈中/待审核/受理中。
#[allow(dead_code)] // 供领域测试与前端新建语义对齐；空 orderStates 表示不限，故不挂 serde default
pub fn default_order_states() -> Vec<i32> {
    vec![1, 2, 3, 7, 8]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    pub id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub org_account_id: i64,
    #[serde(default)]
    pub account_name: String,
    pub area_id: i64,
    #[serde(default)]
    pub area_name: String,
    /// `"current"` | `"fixed"`
    #[serde(default = "default_bill_month_mode")]
    pub bill_month_mode: String,
    /// fixed 时必填，YYYYMM
    #[serde(default)]
    pub bill_month: String,
    /// 机构侧业务月份提示（来自 orgAccount 的 orderMonthGjj / orderMonthSb）；仅展示，不影响 `current` 解析。
    #[serde(default)]
    pub business_bill_month: String,
    #[serde(default)]
    pub order_states: Vec<i32>,
    #[serde(default)]
    pub biz_types: Vec<String>,
    #[serde(default)]
    pub ins_codes: Vec<i32>,
}

fn default_true() -> bool {
    true
}

fn default_bill_month_mode() -> String {
    "current".to_string()
}

impl Subscription {
    /// 新建订阅（默认 orderStates = [1,2,3,7,8]）。
    #[allow(dead_code)] // 领域构造入口；前端也可本地生成同形默认值
    pub fn new_default() -> Self {
        Self {
            id: generate_subscription_id(),
            enabled: true,
            org_account_id: 0,
            account_name: String::new(),
            area_id: 0,
            area_name: String::new(),
            bill_month_mode: default_bill_month_mode(),
            bill_month: String::new(),
            business_bill_month: String::new(),
            order_states: default_order_states(),
            biz_types: DISPLAY_BIZ_TYPES
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            ins_codes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct OrderSubscribeConfig {
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

/// 地区 × 业务类型计数项（高亮=已订阅计入总数）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BizTypeCountItem {
    pub biz_type: String,
    pub count: i64,
    pub highlighted: bool,
}

/// 按订单 `areaName` 聚合的明细行。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AreaBizBreakdown {
    pub area_name: String,
    pub counts: Vec<BizTypeCountItem>,
}

/// 单条订阅的执行结果（成功含明细；失败含 error）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionRunResult {
    pub subscription_id: String,
    pub account_name: String,
    pub org_account_id: i64,
    #[serde(default)]
    pub config_area_name: String,
    pub bill_month: String,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub subscribed_total: i64,
    #[serde(default)]
    pub areas: Vec<AreaBizBreakdown>,
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

/// 首页待办摘要中的单一业务来源（预留多业务汇总）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PendingSource {
    pub id: String,
    pub label: String,
    pub count: i64,
}

/// 首页顶栏待办读模型：v1 仅后道订单；`sources` 预留日后多业务加总。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HomePendingSummary {
    pub total: i64,
    #[serde(default)]
    pub sources: Vec<PendingSource>,
}

/// 启动/进入首页时「或许自动执行」的返回：是否实际跑了一轮 + 当前快照。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MaybeAutoRunOutcome {
    pub did_run: bool,
    pub snapshot: ExecutionSnapshot,
}

const ORDER_SUBSCRIBE_SOURCE_ID: &str = "order-subscribe";
const ORDER_SUBSCRIBE_SOURCE_LABEL: &str = "后道订单订阅";

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

pub fn load_order_subscribe_config() -> Result<OrderSubscribeConfig, String> {
    let path = config_path()?;
    if !path.exists() {
        let config = OrderSubscribeConfig::default();
        save_order_subscribe_config_to_disk(&config)?;
        return Ok(config);
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("读取后道订单订阅配置失败: {e}"))?;
    let mut config: OrderSubscribeConfig =
        serde_json::from_str(&content).map_err(|e| format!("解析后道订单订阅配置失败: {e}"))?;
    normalize_config(&mut config);
    Ok(config)
}

fn save_order_subscribe_config_to_disk(config: &OrderSubscribeConfig) -> Result<(), String> {
    let path = config_path()?;
    ensure_config_dir(&path)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化后道订单订阅配置失败: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("写入后道订单订阅配置失败: {e}"))?;
    Ok(())
}

pub fn save_order_subscribe_config(
    mut config: OrderSubscribeConfig,
) -> Result<OrderSubscribeConfig, String> {
    normalize_config(&mut config);
    validate_config(&config)?;
    save_order_subscribe_config_to_disk(&config)?;
    Ok(config)
}

/// 仅更新启动自动查询开关（不校验订阅完整性）。
pub fn set_order_subscribe_auto_run(enabled: bool) -> Result<OrderSubscribeConfig, String> {
    let mut config = load_order_subscribe_config()?;
    config.auto_run_on_startup = enabled;
    save_order_subscribe_config_to_disk(&config)?;
    Ok(config)
}

/// 规范化订阅配置：账期模式、业务类型白名单、字段 trim。
pub fn normalize_config(config: &mut OrderSubscribeConfig) {
    for sub in &mut config.subscriptions {
        normalize_subscription(sub);
    }
}

pub fn normalize_subscription(sub: &mut Subscription) {
    sub.id = sub.id.trim().to_string();
    if sub.id.is_empty() {
        sub.id = generate_subscription_id();
    }
    sub.account_name = sub.account_name.trim().to_string();
    sub.area_name = sub.area_name.trim().to_string();
    sub.bill_month = sub.bill_month.trim().to_string();
    sub.business_bill_month = sub.business_bill_month.trim().to_string();

    let mode = sub.bill_month_mode.trim().to_ascii_lowercase();
    sub.bill_month_mode = if mode == "fixed" {
        "fixed".to_string()
    } else {
        "current".to_string()
    };

    sub.biz_types = sub
        .biz_types
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| ALLOWED_BIZ_TYPES.contains(&t.as_str()))
        .collect();
    // 去重保序
    let mut seen = std::collections::HashSet::new();
    sub.biz_types.retain(|t| seen.insert(t.clone()));

    sub.order_states.retain(|s| (1..=8).contains(s) && *s != 6);
    sub.ins_codes.sort_unstable();
    sub.ins_codes.dedup();
}

fn validate_config(config: &OrderSubscribeConfig) -> Result<(), String> {
    for (idx, sub) in config.subscriptions.iter().enumerate() {
        let label = if sub.account_name.is_empty() {
            format!("第 {} 条订阅", idx + 1)
        } else {
            sub.account_name.clone()
        };
        if sub.org_account_id <= 0 {
            return Err(format!("{label}：请选择机构"));
        }
        if sub.area_id <= 0 {
            return Err(format!("{label}：请选择地区"));
        }
        if sub.bill_month_mode == "fixed" {
            if !is_valid_bill_month(&sub.bill_month) {
                return Err(format!("{label}：固定月份须为 YYYYMM"));
            }
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

/// 从机构业务账期字段合成 YYYYMM（取社保/公积金中较大的有效值）。
pub fn business_bill_month_from_org_months(order_month_gjj: i64, order_month_sb: i64) -> String {
    [order_month_gjj, order_month_sb]
        .into_iter()
        .map(|n| n.to_string())
        .filter(|s| is_valid_bill_month(s))
        .max()
        .unwrap_or_default()
}

/// 月份解析：
/// - `fixed`：用订阅上的 YYYYMM
/// - `current`：本地系统当前自然月
pub fn resolve_bill_month(sub: &Subscription, now: NaiveDate) -> String {
    if sub.bill_month_mode == "fixed" && is_valid_bill_month(&sub.bill_month) {
        return sub.bill_month.clone();
    }
    now.format("%Y%m").to_string()
}

/// 构造 operateOrder/pageList 请求体（仅第 1 页，pageSize=100）。
pub fn build_operate_order_body(sub: &Subscription, bill_month: &str) -> Value {
    json!({
        "pageNo": 1,
        "pageSize": 100,
        "billMonth": bill_month,
        "orgAccountIds": [sub.org_account_id],
        "orderStates": sub.order_states,
        "insCodes": sub.ins_codes,
        "areaIds": [],
        "accountTypes": [],
        "handleStates": [],
        "managerUserIds": [],
    })
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
    // 兼容仅有 executed_at 的情况：取日期前缀
    let prefix = snapshot.executed_at.get(..10).unwrap_or("");
    should_auto_run(Some(prefix), today)
}

/// 业务类型 → 上游 count 字段名。
pub fn biz_type_count_field(biz_type: &str) -> Option<&'static str> {
    match biz_type {
        "sbAdd" => Some("sbAddCount"),
        "sbFill" => Some("sbFillCount"),
        "sbStop" => Some("sbStopCount"),
        "gjjAdd" => Some("gjjAddCount"),
        "gjjFill" => Some("gjjFillCount"),
        "gjjStop" => Some("gjjStopCount"),
        _ => None,
    }
}

pub fn is_biz_type_highlighted(biz_type: &str, subscribed: &[String]) -> bool {
    subscribed.iter().any(|t| t == biz_type)
}

fn record_count_field(record: &Value, field: &str) -> i64 {
    record
        .get(field)
        .and_then(value_to_i64)
        .unwrap_or(0)
        .max(0)
}

fn record_area_name(record: &Value) -> String {
    record
        .get("areaName")
        .or_else(|| record.get("area_name"))
        .map(value_as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "未知地区".to_string())
}

/// 跨 records 对所选 bizTypes 对应 count 字段求和；空 bizTypes → 0。
pub fn sum_subscribed_counts(records: &[Value], biz_types: &[String]) -> i64 {
    if biz_types.is_empty() {
        return 0;
    }
    let mut total = 0i64;
    for record in records {
        for biz in biz_types {
            if let Some(field) = biz_type_count_field(biz) {
                total += record_count_field(record, field);
            }
        }
    }
    total
}

/// 按 areaName × 展示业务类型拆分，并标记高亮（keep 永不出现）。
pub fn build_area_breakdowns(
    records: &[Value],
    biz_types: &[String],
) -> Vec<AreaBizBreakdown> {
    let mut area_map: BTreeMap<String, BTreeMap<&'static str, i64>> = BTreeMap::new();
    for record in records {
        let area = record_area_name(record);
        let entry = area_map.entry(area).or_default();
        for &biz in DISPLAY_BIZ_TYPES {
            if let Some(field) = biz_type_count_field(biz) {
                *entry.entry(biz).or_insert(0) += record_count_field(record, field);
            }
        }
    }

    area_map
        .into_iter()
        .map(|(area_name, counts_map)| {
            let counts = DISPLAY_BIZ_TYPES
                .iter()
                .map(|&biz| BizTypeCountItem {
                    biz_type: biz.to_string(),
                    count: *counts_map.get(biz).unwrap_or(&0),
                    highlighted: is_biz_type_highlighted(biz, biz_types),
                })
                .collect();
            AreaBizBreakdown { area_name, counts }
        })
        .collect()
}

/// 从 pageList records 汇总单条订阅成功结果。
pub fn aggregate_subscription_success(
    sub: &Subscription,
    bill_month: &str,
    records: &[Value],
) -> SubscriptionRunResult {
    let areas = build_area_breakdowns(records, &sub.biz_types);
    let subscribed_total = sum_subscribed_counts(records, &sub.biz_types);
    SubscriptionRunResult {
        subscription_id: sub.id.clone(),
        account_name: sub.account_name.clone(),
        org_account_id: sub.org_account_id,
        config_area_name: sub.area_name.clone(),
        bill_month: bill_month.to_string(),
        success: true,
        error: None,
        subscribed_total,
        areas,
    }
}

pub fn aggregate_subscription_error(
    sub: &Subscription,
    bill_month: &str,
    error: String,
) -> SubscriptionRunResult {
    SubscriptionRunResult {
        subscription_id: sub.id.clone(),
        account_name: sub.account_name.clone(),
        org_account_id: sub.org_account_id,
        config_area_name: sub.area_name.clone(),
        bill_month: bill_month.to_string(),
        success: false,
        error: Some(error),
        subscribed_total: 0,
        areas: Vec::new(),
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

/// 纯领域：对启用订阅用注入的 pageList 响应（或错误）聚合；disabled 跳过。
pub fn execute_subscriptions_pure(
    config: &OrderSubscribeConfig,
    now: NaiveDate,
    executed_at: &str,
    fetch: &dyn Fn(&Subscription, &str) -> Result<Value, String>,
) -> ExecutionSnapshot {
    let executed_date = now.format("%Y-%m-%d").to_string();
    let mut results = Vec::new();
    for sub in &config.subscriptions {
        if !sub.enabled {
            continue;
        }
        let bill_month = resolve_bill_month(sub, now);
        match fetch(sub, &bill_month) {
            Ok(body) => match yunsheng_auth::ensure_yunsheng_business_ok(&body) {
                Ok(()) => {
                    let records = extract_records(&body);
                    results.push(aggregate_subscription_success(sub, &bill_month, &records));
                }
                Err(err) => {
                    results.push(aggregate_subscription_error(sub, &bill_month, err));
                }
            },
            Err(err) => {
                results.push(aggregate_subscription_error(sub, &bill_month, err));
            }
        }
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

/// 将 orgAccount/pageList 的单条 record 映射为 OrgAccountHit（id → orgAccountId）。
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
    let root = body
        .get("data")
        .cloned()
        .unwrap_or_else(|| body.clone());
    walk_areas(&root, "", &mut out);
    // 去重保序
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
            // 若当前对象没有 areaId，仍递归其余数组字段以兼容未知嵌套
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

async fn post_manage_api(path: &str, body: Value) -> Result<Value, String> {
    let cookies = yunsheng_auth::get_cookies()?;
    let proxy = http_client::load_http_proxy_config().unwrap_or_default();
    let client = yunsheng_auth::build_yunsheng_http_client(&proxy)?;
    let url = format!("{MANAGE_API_BASE}{path}");

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

    let parsed: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }));

    if let Some(err) = yunsheng_auth::map_auth_error(status, Some(&parsed)) {
        return Err(err);
    }
    if !(200..300).contains(&status) {
        let msg = parsed
            .get("msg")
            .or_else(|| parsed.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("请求失败");
        return Err(format!("云生接口错误 ({status}): {msg}"));
    }

    // 业务层失败（含鉴权 30000、status:false、code!=0）不得当成成功空列表
    yunsheng_auth::ensure_yunsheng_business_ok(&parsed)?;

    Ok(parsed)
}

pub async fn list_order_subscribe_areas() -> Result<Vec<AreaOption>, String> {
    let body = post_manage_api("/areaSetting/selectList", json!({})).await?;
    Ok(flatten_area_options(&body))
}

pub async fn search_order_subscribe_orgs(
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

pub fn load_order_subscribe_result() -> Result<ExecutionSnapshot, String> {
    let path = result_path()?;
    if !path.exists() {
        return Ok(ExecutionSnapshot::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("读取后道订单执行结果失败: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("解析后道订单执行结果失败: {e}"))
}

/// 删除落盘的执行结果快照（首页待办随之归零）。
pub fn clear_order_subscribe_result() -> Result<(), String> {
    let path = result_path()?;
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("删除后道订单执行结果失败: {e}"))?;
    }
    Ok(())
}

fn save_order_subscribe_result_to_disk(snapshot: &ExecutionSnapshot) -> Result<(), String> {
    let path = result_path()?;
    ensure_config_dir(&path)?;
    let content = serde_json::to_string_pretty(snapshot)
        .map_err(|e| format!("序列化后道订单执行结果失败: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("写入后道订单执行结果失败: {e}"))?;
    Ok(())
}

/// 手动「立即执行」：对所有启用订阅各查第 1 页并覆盖结果快照。
pub async fn run_order_subscribe_now() -> Result<ExecutionSnapshot, String> {
    let config = load_order_subscribe_config()?;
    let now = chrono::Local::now();
    let today = now.date_naive();
    let executed_at = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut responses: BTreeMap<String, Result<Value, String>> = BTreeMap::new();
    for sub in &config.subscriptions {
        if !sub.enabled {
            continue;
        }
        let bill_month = resolve_bill_month(sub, today);
        let body = build_operate_order_body(sub, &bill_month);
        responses.insert(
            sub.id.clone(),
            post_manage_api("/operateOrder/pageList", body).await,
        );
    }

    let snapshot = execute_subscriptions_pure(&config, today, &executed_at, &|sub, _bill_month| {
        responses.get(&sub.id).cloned().unwrap_or_else(|| {
            Err("内部错误：缺少订阅查询结果".to_string())
        })
    });
    save_order_subscribe_result_to_disk(&snapshot)?;
    Ok(snapshot)
}

/// 由快照构造首页待办摘要。v1：`total` = 后道订单总数；不含流水线/合并。
pub fn pending_summary_from_snapshot(snapshot: &ExecutionSnapshot) -> HomePendingSummary {
    let count = snapshot.total.max(0);
    HomePendingSummary {
        total: count,
        sources: vec![PendingSource {
            id: ORDER_SUBSCRIBE_SOURCE_ID.to_string(),
            label: ORDER_SUBSCRIBE_SOURCE_LABEL.to_string(),
            count,
        }],
    }
}

/// 读取上次快照并返回首页待办摘要（不触发网络）。
pub fn get_home_pending_summary() -> Result<HomePendingSummary, String> {
    let snapshot = load_order_subscribe_result()?;
    Ok(pending_summary_from_snapshot(&snapshot))
}

/// 启动/进入首页：仅当配置开启「启动自动查询」且本地自然日门控通过时才执行；
/// 实际执行完成后走与流水线/合并监控相同的系统通知（桌面横幅 + 原生弹窗）。
pub async fn maybe_auto_run_order_subscribe(
    app: &AppHandle,
) -> Result<MaybeAutoRunOutcome, String> {
    let snapshot = load_order_subscribe_result()?;
    let config = load_order_subscribe_config()?;
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
    match run_order_subscribe_now().await {
        Ok(snapshot) => {
            let failed = snapshot
                .subscriptions
                .iter()
                .filter(|item| !item.success)
                .count();
            let title = "后道订单订阅 · 启动查询完成";
            let body = if failed > 0 {
                format!(
                    "待办总数 {} · {} 条订阅失败",
                    snapshot.total, failed
                )
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
                "后道订单订阅 · 启动查询失败",
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
    fn new_subscription_uses_default_order_states() {
        let sub = Subscription::new_default();
        assert_eq!(sub.order_states, vec![1, 2, 3, 7, 8]);
        assert!(sub.enabled);
        assert_eq!(sub.bill_month_mode, "current");
        assert_eq!(
            sub.biz_types,
            DISPLAY_BIZ_TYPES
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        );
        assert!(sub.ins_codes.is_empty());
        assert!(!sub.id.is_empty());
    }

    #[test]
    fn normalize_strips_keep_and_invalid_biz_types() {
        let mut sub = Subscription::new_default();
        sub.biz_types = vec![
            "sbAdd".into(),
            "sbKeep".into(),
            "gjjKeep".into(),
            "keep".into(),
            "gjjStop".into(),
            "unknown".into(),
            " sbFill ".into(),
        ];
        normalize_subscription(&mut sub);
        assert_eq!(sub.biz_types, vec!["sbAdd", "gjjStop", "sbFill"]);
    }

    #[test]
    fn normalize_bill_month_mode_defaults_to_current() {
        let mut sub = Subscription::new_default();
        sub.bill_month_mode = "FIXED".into();
        normalize_subscription(&mut sub);
        assert_eq!(sub.bill_month_mode, "fixed");

        sub.bill_month_mode = "anything".into();
        normalize_subscription(&mut sub);
        assert_eq!(sub.bill_month_mode, "current");
    }

    #[test]
    fn normalize_config_dedupes_ins_codes() {
        let mut config = OrderSubscribeConfig {
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
    fn validate_rejects_missing_org_and_bad_fixed_month() {
        let mut config = OrderSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![{
                let mut s = Subscription::new_default();
                s.org_account_id = 0;
                s.area_id = 85;
                s.account_name = "测试".into();
                s
            }],
        };
        let err = validate_config(&config).expect_err("missing org");
        assert!(err.contains("机构"));

        config.subscriptions[0].org_account_id = 1;
        config.subscriptions[0].bill_month_mode = "fixed".into();
        config.subscriptions[0].bill_month = "2026".into();
        let err = validate_config(&config).expect_err("bad month");
        assert!(err.contains("YYYYMM"));
    }

    #[test]
    fn subscription_serde_camel_case_roundtrip() {
        let sub = Subscription {
            id: "sub-1".into(),
            enabled: false,
            org_account_id: 42,
            account_name: "Acme".into(),
            area_id: 85,
            area_name: "济南市".into(),
            bill_month_mode: "fixed".into(),
            bill_month: "202607".into(),
            business_bill_month: "202608".into(),
            order_states: default_order_states(),
            biz_types: vec!["sbAdd".into(), "gjjStop".into()],
            ins_codes: vec![20, 30],
        };
        let value = serde_json::to_value(&sub).unwrap();
        assert_eq!(value["orgAccountId"], 42);
        assert_eq!(value["billMonthMode"], "fixed");
        assert_eq!(value["orderStates"], json!([1, 2, 3, 7, 8]));
        assert!(value.get("keep").is_none());
        let back: Subscription = serde_json::from_value(value).unwrap();
        assert_eq!(back, sub);
    }

    fn sample_sub(id: &str, org_id: i64, biz_types: &[&str]) -> Subscription {
        let mut s = Subscription::new_default();
        s.id = id.into();
        s.org_account_id = org_id;
        s.account_name = format!("主体{org_id}");
        s.area_id = 85;
        s.area_name = "济南市".into();
        s.biz_types = biz_types.iter().map(|t| (*t).to_string()).collect();
        s
    }

    /// 规格验证过的 operateOrder/pageList 形态 fixture（含 keep 字段，汇总时须忽略）。
    fn operate_order_fixture() -> Value {
        json!({
            "code": 0,
            "data": {
                "records": [
                    {
                        "areaName": "济南市",
                        "sbAddCount": 2,
                        "sbFillCount": 1,
                        "sbStopCount": 0,
                        "sbKeepCount": 99,
                        "gjjAddCount": 0,
                        "gjjFillCount": 3,
                        "gjjStopCount": 4,
                        "gjjKeepCount": 88
                    },
                    {
                        "areaName": "青岛市",
                        "sbAddCount": 1,
                        "sbFillCount": 0,
                        "sbStopCount": 5,
                        "sbKeepCount": 1,
                        "gjjAddCount": 2,
                        "gjjFillCount": 0,
                        "gjjStopCount": 1,
                        "gjjKeepCount": 1
                    },
                    {
                        "areaName": "济南市",
                        "sbAddCount": 1,
                        "sbFillCount": 2,
                        "sbStopCount": 0,
                        "gjjAddCount": 0,
                        "gjjFillCount": 0,
                        "gjjStopCount": 2
                    }
                ],
                "total": 3
            }
        })
    }

    #[test]
    fn resolve_bill_month_current_uses_injected_now() {
        let mut sub = sample_sub("a", 1, &[]);
        sub.bill_month_mode = "current".into();
        sub.business_bill_month.clear();
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        assert_eq!(resolve_bill_month(&sub, now), "202607");
    }

    #[test]
    fn resolve_bill_month_current_ignores_business_month() {
        let mut sub = sample_sub("a", 1, &[]);
        sub.bill_month_mode = "current".into();
        sub.business_bill_month = "202608".into();
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        assert_eq!(resolve_bill_month(&sub, now), "202607");
    }

    #[test]
    fn business_bill_month_from_org_months_picks_max_valid() {
        assert_eq!(
            business_bill_month_from_org_months(202607, 202608),
            "202608"
        );
        assert_eq!(business_bill_month_from_org_months(0, 0), "");
        assert_eq!(business_bill_month_from_org_months(202613, 202607), "202607");
    }

    #[test]
    fn resolve_bill_month_fixed_uses_subscription_value() {
        let mut sub = sample_sub("a", 1, &[]);
        sub.bill_month_mode = "fixed".into();
        sub.bill_month = "202601".into();
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        assert_eq!(resolve_bill_month(&sub, now), "202601");
    }

    #[test]
    fn resolve_bill_month_invalid_fixed_falls_back_to_current() {
        let mut sub = sample_sub("a", 1, &[]);
        sub.bill_month_mode = "fixed".into();
        sub.bill_month = "bad".into();
        let now = NaiveDate::from_ymd_opt(2026, 3, 5).unwrap();
        assert_eq!(resolve_bill_month(&sub, now), "202603");
    }

    #[test]
    fn should_auto_run_never_yesterday_today_success_today_failure() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        assert!(should_auto_run(None, today));
        assert!(should_auto_run(Some(""), today));
        assert!(should_auto_run(Some("2026-07-27"), today));
        assert!(!should_auto_run(Some("2026-07-28"), today));

        // 今日失败也视为已执行：门控只看日期
        let failed_today = ExecutionSnapshot {
            executed_at: "2026-07-28T09:00:00".into(),
            executed_date: "2026-07-28".into(),
            total: 0,
            subscriptions: vec![aggregate_subscription_error(
                &sample_sub("x", 1, &["sbAdd"]),
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
        let config = OrderSubscribeConfig::default();
        assert!(!config.auto_run_on_startup);
    }

    #[test]
    fn pending_summary_v1_only_order_subscribe() {
        let snapshot = ExecutionSnapshot {
            executed_at: "2026-07-28T10:00:00".into(),
            executed_date: "2026-07-28".into(),
            total: 42,
            subscriptions: vec![],
        };
        let summary = pending_summary_from_snapshot(&snapshot);
        assert_eq!(summary.total, 42);
        assert_eq!(summary.sources.len(), 1);
        assert_eq!(summary.sources[0].id, "order-subscribe");
        assert_eq!(summary.sources[0].label, "后道订单订阅");
        assert_eq!(summary.sources[0].count, 42);

        let empty = pending_summary_from_snapshot(&ExecutionSnapshot::default());
        assert_eq!(empty.total, 0);
        assert_eq!(empty.sources.len(), 1);
        assert_eq!(empty.sources[0].count, 0);
    }

    #[test]
    fn build_operate_order_body_shape() {
        let mut sub = sample_sub("s1", 12345, &["sbAdd"]);
        sub.order_states = vec![1, 2];
        sub.ins_codes = vec![20];
        let body = build_operate_order_body(&sub, "202607");
        assert_eq!(body["pageNo"], 1);
        assert_eq!(body["pageSize"], 100);
        assert_eq!(body["billMonth"], "202607");
        assert_eq!(body["orgAccountIds"], json!([12345]));
        assert_eq!(body["orderStates"], json!([1, 2]));
        assert_eq!(body["insCodes"], json!([20]));
        assert_eq!(body["areaIds"], json!([]));
        assert_eq!(body["accountTypes"], json!([]));
        assert_eq!(body["handleStates"], json!([]));
        assert_eq!(body["managerUserIds"], json!([]));
        // insCodes 与 bizTypes 独立：请求体不含 bizTypes
        assert!(body.get("bizTypes").is_none());
    }

    #[test]
    fn build_operate_order_body_empty_filters() {
        let mut sub = sample_sub("s1", 9, &[]);
        sub.order_states = vec![];
        sub.ins_codes = vec![];
        let body = build_operate_order_body(&sub, "202601");
        assert_eq!(body["orderStates"], json!([]));
        assert_eq!(body["insCodes"], json!([]));
    }

    #[test]
    fn sum_subscribed_counts_across_records_ignores_keep() {
        let records = extract_records(&operate_order_fixture());
        // 济南 2+1 sbAdd + 青岛 1 = 4；keep 不计入
        assert_eq!(
            sum_subscribed_counts(&records, &["sbAdd".into()]),
            4
        );
        // gjjStop: 济南 4+2 + 青岛 1 = 7
        assert_eq!(
            sum_subscribed_counts(&records, &["gjjStop".into()]),
            7
        );
        assert_eq!(sum_subscribed_counts(&records, &[]), 0);
    }

    #[test]
    fn highlight_classification_and_no_keep() {
        let records = extract_records(&operate_order_fixture());
        let areas = build_area_breakdowns(&records, &["sbAdd".into(), "gjjStop".into()]);
        assert_eq!(areas.len(), 2);
        let jinan = areas.iter().find(|a| a.area_name == "济南市").unwrap();
        assert_eq!(jinan.counts.len(), 6);
        assert!(jinan.counts.iter().all(|c| c.biz_type != "sbKeep"));
        assert!(jinan.counts.iter().all(|c| !c.biz_type.contains("Keep")));

        let sb_add = jinan.counts.iter().find(|c| c.biz_type == "sbAdd").unwrap();
        assert!(sb_add.highlighted);
        assert_eq!(sb_add.count, 3); // 2+1

        let sb_fill = jinan.counts.iter().find(|c| c.biz_type == "sbFill").unwrap();
        assert!(!sb_fill.highlighted);
        assert_eq!(sb_fill.count, 3); // 1+2

        let empty_biz = build_area_breakdowns(&records, &[]);
        assert!(empty_biz[0].counts.iter().all(|c| !c.highlighted));
    }

    #[test]
    fn execute_pure_rejects_auth_failure_body_as_error_not_empty_success() {
        // 无效 token 实测响应：不得记为 success=true、待办 0
        let sub = sample_sub("auth-bad", 1, &["sbAdd"]);
        let config = OrderSubscribeConfig {
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
            &|_sub, _bill| Ok(auth_fail.clone()),
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
    fn execute_pure_skips_disabled_and_keeps_partial_success() {
        let enabled = sample_sub("ok", 1, &["sbAdd"]);
        let mut disabled = sample_sub("off", 2, &["gjjStop"]);
        disabled.enabled = false;
        let failing = sample_sub("bad", 3, &["sbFill"]);

        let config = OrderSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![enabled, disabled, failing],
        };
        let now = NaiveDate::from_ymd_opt(2026, 7, 28).unwrap();
        let fixture = operate_order_fixture();

        let snapshot = execute_subscriptions_pure(
            &config,
            now,
            "2026-07-28T12:00:00",
            &|sub, bill_month| {
                assert_eq!(bill_month, "202607");
                if sub.id == "bad" {
                    Err("模拟失败".into())
                } else {
                    Ok(fixture.clone())
                }
            },
        );

        assert_eq!(snapshot.subscriptions.len(), 2); // disabled 跳过
        assert!(snapshot.subscriptions.iter().all(|r| r.subscription_id != "off"));
        let ok = snapshot
            .subscriptions
            .iter()
            .find(|r| r.subscription_id == "ok")
            .unwrap();
        assert!(ok.success);
        assert_eq!(ok.subscribed_total, 4);
        let bad = snapshot
            .subscriptions
            .iter()
            .find(|r| r.subscription_id == "bad")
            .unwrap();
        assert!(!bad.success);
        assert_eq!(bad.error.as_deref(), Some("模拟失败"));
        assert_eq!(bad.subscribed_total, 0);
        // 总数仅计成功
        assert_eq!(snapshot.total, 4);
        assert_eq!(snapshot.executed_date, "2026-07-28");
    }

    #[test]
    fn multi_subscription_totals_sum_enabled_success() {
        let a = sample_sub("a", 1, &["gjjStop"]);
        let b = sample_sub("b", 2, &["sbStop"]);
        let config = OrderSubscribeConfig {
            auto_run_on_startup: false,
            subscriptions: vec![a, b],
        };
        let now = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let fixture = operate_order_fixture();
        let snapshot = execute_subscriptions_pure(
            &config,
            now,
            "2026-07-01T08:00:00",
            &|_, _| Ok(fixture.clone()),
        );
        // gjjStop=7 + sbStop=5 → 12
        assert_eq!(snapshot.total, 12);
    }

    #[test]
    fn empty_biz_types_contribute_zero_but_show_gray_counts() {
        let sub = sample_sub("disc", 1, &[]);
        let records = extract_records(&operate_order_fixture());
        let result = aggregate_subscription_success(&sub, "202607", &records);
        assert_eq!(result.subscribed_total, 0);
        assert!(!result.areas.is_empty());
        assert!(result.areas[0].counts.iter().all(|c| !c.highlighted));
        assert!(result.areas[0].counts.iter().any(|c| c.count > 0));
    }
}
