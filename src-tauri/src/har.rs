use std::{
 collections::{HashMap, HashSet},
 fmt::Write as _,
 fs,
 path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
 executor::{self, BatchExecutionRequest, EncryptionRequest},
 project_store,
};
use coter_core::schema::ProjectConfig;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarProcessRequest {
 pub input_path: String,
 pub output_path: String,
 #[serde(default)]
 pub project_id: Option<u64>,
 #[serde(default)]
 pub project_name: Option<String>,
 #[serde(default)]
 pub input_original_ref: String,
 #[serde(default)]
 pub final_output_mapping_id: String,
 #[serde(default = "default_regex_preset")]
 pub regex_preset: String,
 #[serde(default)]
 pub input_values: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarProcessResponse {
 pub input_path: String,
 pub output_path: String,
 pub file_name: String,
 pub stats: HarStats,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarStats {
 pub total_text_fields: u64,
 pub matched: u64,
 pub success: u64,
 pub failed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegexPreset {
 Base64,
 Hex,
}

struct ProcessingContext {
 regex_preset: RegexPreset,
 input_original_ref: String,
 final_output_index: usize,
 component_prototypes: Vec<EncryptionRequest>,
 base_input_values: HashMap<String, String>,
}

pub fn process_har(request: HarProcessRequest) -> Result<HarProcessResponse, String> {
 validate_request(&request)?;

 let input_path = PathBuf::from(request.input_path.trim());
 let output_path = PathBuf::from(request.output_path.trim());

 if !input_path.is_file() {
 return Err(format!("HAR 输入文件不存在: {}", input_path.display()));
 }

 let content = fs::read_to_string(&input_path)
 .map_err(|error| format!("读取 HAR 文件失败 {}: {error}", input_path.display()))?;

 let parsed: Value =
 serde_json::from_str(&content).map_err(|error| format!("HAR 文件解析失败: {error}"))?;

 let config = load_project_config(&request)?;
 let (processed, stats) = process_har_json_with_config(parsed, &request, &config)?;

 if let Some(parent) = output_path
 .parent()
 .filter(|path| !path.as_os_str().is_empty())
 {
 fs::create_dir_all(parent)
 .map_err(|error| format!("创建 HAR 输出目录失败 {}: {error}", parent.display()))?;
 }

 fs::write(
 &output_path,
 serde_json::to_string_pretty(&processed)
 .map_err(|error| format!("序列化 HAR 文件失败: {error}"))?,
 )
 .map_err(|error| format!("写入 HAR 输出文件失败 {}: {error}", output_path.display()))?;

 Ok(HarProcessResponse {
 input_path: input_path.display().to_string(),
 file_name: file_name(&output_path),
 output_path: output_path.display().to_string(),
 stats,
 })
}

fn validate_request(request: &HarProcessRequest) -> Result<(), String> {
 if request.input_path.trim().is_empty() {
 return Err("HAR 输入文件路径不能为空".to_string());
 }

 if request.output_path.trim().is_empty() {
 return Err("HAR 输出文件路径不能为空".to_string());
 }

 if request.input_original_ref.trim().is_empty() {
 return Err("inputOriginalRef 不能为空".to_string());
 }

 if request.final_output_mapping_id.trim().is_empty() {
 return Err("finalOutputMappingId 不能为空".to_string());
 }

 parse_regex_preset(&request.regex_preset).map(|_| ())
}

fn load_project_config(request: &HarProcessRequest) -> Result<ProjectConfig, String> {
 let mut project = None;

 if let Some(id) = request.project_id {
 project = project_store::get_project_by_id(id)?;
 }

 if project.is_none() {
 if let Some(name) = request
 .project_name
 .as_deref()
 .map(str::trim)
 .filter(|value| !value.is_empty())
 {
 project = project_store::get_project_by_name(name)?;
 }
 }

 let project = project.ok_or_else(|| "找不到项目配置，请先保存当前配置".to_string())?;
 if project.config.trim().is_empty() {
 return Err("找不到项目配置，请先保存当前配置".to_string());
 }

 serde_json::from_str(&project.config).map_err(|error| format!("项目配置解析失败: {error}"))
}

fn process_har_json_with_config(
 mut root: Value,
 request: &HarProcessRequest,
 config: &ProjectConfig,
) -> Result<(Value, HarStats), String> {
 let context = build_processing_context(request, config)?;
 let mut stats = HarStats::default();

 process_tree(&mut root, &context, &mut stats);

 Ok((root, stats))
}

fn build_processing_context(
 request: &HarProcessRequest,
 config: &ProjectConfig,
) -> Result<ProcessingContext, String> {
 if config.components.is_empty() {
 return Err("项目未配置组件，无法执行".to_string());
 }

 if config.output_mappings.is_empty() {
 return Err("项目未配置输出映射，请先配置'最终输出'".to_string());
 }

 let final_output_mapping_id = request.final_output_mapping_id.trim();
 let target_mapping = config
 .output_mappings
 .iter()
 .find(|mapping| mapping.id == final_output_mapping_id)
 .ok_or_else(|| format!("找不到指定的输出映射: {final_output_mapping_id}"))?;

 let final_output_index = config
 .components
 .iter()
 .position(|component| component.output_ref == target_mapping.component_ref)
 .ok_or_else(|| format!("输出映射引用的组件不存在: {}", target_mapping.component_ref))?;

 let component_prototypes = convert_components(config);
 let mut base_input_values = HashMap::new();

 for mapping in &config.input_mappings {
 if !mapping.input_ref.is_empty() {
 base_input_values.insert(mapping.input_ref.clone(), mapping.default_value.clone());
 }
 }
 base_input_values.extend(request.input_values.clone());

 Ok(ProcessingContext {
 regex_preset: parse_regex_preset(&request.regex_preset)?,
 input_original_ref: request.input_original_ref.trim().to_string(),
 final_output_index,
 component_prototypes,
 base_input_values,
 })
}

fn convert_components(config: &ProjectConfig) -> Vec<EncryptionRequest> {
 let input_refs = config
 .input_mappings
 .iter()
 .filter_map(|mapping| (!mapping.input_ref.is_empty()).then(|| mapping.input_ref.clone()))
 .collect::<HashSet<_>>();

 config
 .components
 .iter()
 .map(|component| {
 let component_config = &component.config;
 let input_source_type =
 config_string_or_default(component_config, "inputSourceType", "reference");
 let input_ref = config_string_or_default(component_config, "inputMappingRef", "");
 let data = match input_source_type.to_ascii_lowercase().as_str() {
 "reference" | "inputmapping" | "component" => {
 if input_ref.is_empty() {
 String::new()
 } else if input_refs.contains(&input_ref) {
 format!("input:{input_ref}")
 } else {
 format!("output:{input_ref}")
 }
 }
 "expression" => config_string_or_default(component_config, "inputExpression", ""),
 _ => String::new(),
 };

 EncryptionRequest {
 algorithm: component.component_type.to_ascii_uppercase(),
 operation: config_string_or_default(component_config, "operation", "encrypt"),
 data,
 key: config_string(component_config, "key"),
 iv: config_string(component_config, "iv"),
 mode: config_string(component_config, "mode"),
 padding: config_string(component_config, "padding"),
 charset: config_string_or_default(component_config, "charset", "UTF-8"),
 public_key: config_string(component_config, "publicKey"),
 private_key: config_string(component_config, "privateKey"),
 input_format: config_string_or_default(component_config, "inputFormat", "hex"),
 output_format: config_string_or_default(component_config, "outputFormat", "hex"),
 key_format: config_string_or_default(component_config, "keyFormat", "text"),
 iv_format: config_string_or_default(component_config, "ivFormat", "text"),
 result_format: config_string_or_default(
 component_config,
 "resultFormat",
 "lowercase",
 ),
 output_length: config_u32(component_config, "outputLength"),
 input_base: config_u32(component_config, "inputBase"),
 output_base: config_u32(component_config, "outputBase"),
 sha_type: config_string(component_config, "shaType"),
 hmac_sha_type: config_string(component_config, "hmacShaType"),
 format: config_string_or_default(component_config, "format", "standard"),
 hex_case: config_string_or_default(component_config, "hexCase", "uppercase"),
 key_ref: config_string(component_config, "keyRef"),
 iv_ref: config_string(component_config, "ivRef"),
 public_key_ref: config_string(component_config, "publicKeyRef"),
 private_key_ref: config_string(component_config, "privateKeyRef"),
 output_ref: (!component.output_ref.is_empty())
 .then(|| component.output_ref.clone()),
 }
 })
 .collect()
}

fn process_tree(node: &mut Value, context: &ProcessingContext, stats: &mut HarStats) {
 match node {
 Value::Object(object) => {
 let keys = object.keys().cloned().collect::<Vec<_>>();

 for key in keys {
 let Some(child) = object.get_mut(&key) else {
 continue;
 };

 if key == "text" {
 if let Some(original) = child.as_str().map(str::to_owned) {
 stats.total_text_fields += 1;
 let mut handled = false;

 if matches_preset(&original, context.regex_preset) {
 stats.matched += 1;
 if let Some(replaced) = try_process_one(&original, context) {
 *child = Value::String(replaced);
 stats.success += 1;
 } else {
 stats.failed += 1;
 }
 handled = true;
 }

 if !handled {
 if let Some(rewritten) =
 try_rewrite_structured_text(&original, context, stats)
 {
 *child = Value::String(rewritten);
 }
 }

 continue;
 }
 }

 process_tree(child, context, stats);
 }
 }
 Value::Array(items) => {
 for child in items {
 process_tree(child, context, stats);
 }
 }
 _ => {}
 }
}

fn try_rewrite_structured_text(
 text: &str,
 context: &ProcessingContext,
 stats: &mut HarStats,
) -> Option<String> {
 if let Ok(mut inner) = serde_json::from_str::<Value>(text) {
 if process_embedded_json(&mut inner, context, stats) {
 return serde_json::to_string(&inner).ok();
 }
 }

 rewrite_form_url_encoded_text(text, context.regex_preset, |candidate| {
 stats.matched += 1;
 let replaced = try_process_one(candidate, context);
 if replaced.is_some() {
 stats.success += 1;
 } else {
 stats.failed += 1;
 }
 replaced
 })
}

fn process_embedded_json(
 node: &mut Value,
 context: &ProcessingContext,
 stats: &mut HarStats,
) -> bool {
 match node {
 Value::String(text) => {
 if !matches_preset(text, context.regex_preset) {
 return false;
 }

 stats.matched += 1;
 if let Some(replaced) = try_process_one(text, context) {
 *text = replaced;
 stats.success += 1;
 true
 } else {
 stats.failed += 1;
 false
 }
 }
 Value::Object(object) => {
 let keys = object.keys().cloned().collect::<Vec<_>>();
 let mut changed = false;

 for key in keys {
 if let Some(child) = object.get_mut(&key) {
 changed |= process_embedded_json(child, context, stats);
 }
 }

 changed
 }
 Value::Array(items) => {
 let mut changed = false;

 for child in items {
 changed |= process_embedded_json(child, context, stats);
 }

 changed
 }
 _ => false,
 }
}

fn try_process_one(text: &str, context: &ProcessingContext) -> Option<String> {
 let mut input_values = context.base_input_values.clone();
 input_values.insert(context.input_original_ref.clone(), text.to_string());

 let responses = executor::execute_batch(BatchExecutionRequest {
 components: context.component_prototypes.clone(),
 input_values,
 });

 responses
 .get(context.final_output_index)
 .filter(|response| response.status.eq_ignore_ascii_case("success"))
 .map(|response| response.result.clone())
}

fn rewrite_form_url_encoded_text<F>(
 text: &str,
 regex_preset: RegexPreset,
 mut processor: F,
) -> Option<String>
where
 F: FnMut(&str) -> Option<String>,
{
 if text.is_empty() || !text.contains('=') {
 return None;
 }

 let pairs = text.split('&').collect::<Vec<_>>();
 let mut saw_assignment = false;
 let mut changed = false;
 let mut rebuilt = String::with_capacity(text.len());

 for (index, pair) in pairs.iter().enumerate() {
 if index > 0 {
 rebuilt.push('&');
 }

 let Some(eq_index) = pair.find('=') else {
 rebuilt.push_str(pair);
 continue;
 };

 saw_assignment = true;
 let key = &pair[..eq_index];
 let raw_value = &pair[eq_index + 1..];
 let rewritten_value = rewrite_form_field_value(raw_value, regex_preset, &mut processor);

 if raw_value != rewritten_value {
 changed = true;
 }

 rebuilt.push_str(key);
 rebuilt.push('=');
 rebuilt.push_str(&rewritten_value);
 }

 (saw_assignment && changed).then_some(rebuilt)
}

fn rewrite_form_field_value<F>(
 raw_value: &str,
 regex_preset: RegexPreset,
 processor: &mut F,
) -> String
where
 F: FnMut(&str) -> Option<String>,
{
 if matches_preset(raw_value, regex_preset) {
 return processor(raw_value)
 .map(|replaced| encode_form_value(&replaced))
 .unwrap_or_else(|| raw_value.to_string());
 }

 if let Some(decoded_value) = try_decode_form_value(raw_value) {
 if decoded_value != raw_value && matches_preset(&decoded_value, regex_preset) {
 return processor(&decoded_value)
 .map(|replaced| encode_form_value(&replaced))
 .unwrap_or_else(|| raw_value.to_string());
 }
 }

 raw_value.to_string()
}

fn parse_regex_preset(regex_preset: &str) -> Result<RegexPreset, String> {
 match regex_preset.trim().to_ascii_uppercase().as_str() {
 "BASE64" => Ok(RegexPreset::Base64),
 "HEX" => Ok(RegexPreset::Hex),
 _ => Err(format!("不支持的 regexPreset: {regex_preset}")),
 }
}

fn matches_preset(text: &str, regex_preset: RegexPreset) -> bool {
 if text.is_empty() {
 return false;
 }

 match regex_preset {
 RegexPreset::Base64 => is_base64_full(text) && !is_hex_full(text),
 RegexPreset::Hex => is_hex_full(text) && text.len() >= 8,
 }
}

fn is_base64_full(text: &str) -> bool {
 let bytes = text.as_bytes();
 if bytes.is_empty() || bytes.len() % 4 != 0 {
 return false;
 }

 let mut padding_started = false;
 let mut padding_count = 0usize;

 for &byte in bytes {
 match byte {
 b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' if !padding_started => {}
 b'=' => {
 padding_started = true;
 padding_count += 1;
 if padding_count > 2 {
 return false;
 }
 }
 _ => return false,
 }
 }

 true
}

fn is_hex_full(text: &str) -> bool {
 !text.is_empty() && text.as_bytes().iter().all(u8::is_ascii_hexdigit)
}

fn encode_form_value(value: &str) -> String {
 let mut encoded = String::new();

 for &byte in value.as_bytes() {
 match byte {
 b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'*' | b'_' => {
 encoded.push(byte as char);
 }
 b' ' => encoded.push('+'),
 _ => {
 let _ = write!(encoded, "%{byte:02X}");
 }
 }
 }

 encoded
}

fn try_decode_form_value(raw_value: &str) -> Option<String> {
 if raw_value.is_empty() || (!raw_value.contains('%') && !raw_value.contains('+')) {
 return Some(raw_value.to_string());
 }

 let bytes = raw_value.as_bytes();
 let mut decoded = Vec::with_capacity(bytes.len());
 let mut index = 0usize;

 while index < bytes.len() {
 match bytes[index] {
 b'+' => {
 decoded.push(b' ');
 index += 1;
 }
 b'%' => {
 if index + 2 >= bytes.len() {
 return None;
 }

 let high = from_hex_digit(bytes[index + 1])?;
 let low = from_hex_digit(bytes[index + 2])?;
 decoded.push((high << 4) | low);
 index += 3;
 }
 byte => {
 decoded.push(byte);
 index += 1;
 }
 }
 }

 String::from_utf8(decoded).ok()
}

fn from_hex_digit(byte: u8) -> Option<u8> {
 match byte {
 b'0'..=b'9' => Some(byte - b'0'),
 b'a'..=b'f' => Some(byte - b'a' + 10),
 b'A'..=b'F' => Some(byte - b'A' + 10),
 _ => None,
 }
}

fn config_string(config: &Map<String, Value>, key: &str) -> Option<String> {
 config.get(key).and_then(value_to_string)
}

fn config_string_or_default(config: &Map<String, Value>, key: &str, default: &str) -> String {
 config_string(config, key).unwrap_or_else(|| default.to_string())
}

fn config_u32(config: &Map<String, Value>, key: &str) -> Option<u32> {
 match config.get(key)? {
 Value::Number(value) => value.as_u64().and_then(|value| u32::try_from(value).ok()),
 Value::String(value) => value.trim().parse::<u32>().ok(),
 _ => None,
 }
}

fn value_to_string(value: &Value) -> Option<String> {
 match value {
 Value::Null => None,
 Value::String(value) => Some(value.clone()),
 Value::Bool(value) => Some(value.to_string()),
 Value::Number(value) => Some(value.to_string()),
 Value::Array(_) | Value::Object(_) => Some(value.to_string()),
 }
}

fn default_regex_preset() -> String {
 "BASE64".to_string()
}

fn file_name(path: &Path) -> String {
 path.file_name()
 .map(|value| value.to_string_lossy().into_owned())
 .unwrap_or_else(|| "processed.har".to_string())
}

#[cfg(test)]
mod tests {
 use std::{collections::HashMap, fs};

 use serde_json::json;
 use tempfile::tempdir;

 use super::{
 matches_preset, parse_regex_preset, process_har, process_har_json_with_config,
 rewrite_form_url_encoded_text, HarProcessRequest, RegexPreset,
 };

 fn request(input_path: String, output_path: String) -> HarProcessRequest {
 HarProcessRequest {
 input_path,
 output_path,
 project_id: None,
 project_name: None,
 input_original_ref: "plain".to_string(),
 final_output_mapping_id: "result".to_string(),
 regex_preset: "BASE64".to_string(),
 input_values: Default::default(),
 }
 }

 fn base64_decode_config() -> super::ProjectConfig {
 serde_json::from_value(json!({
 "components": [
 {
 "id": "component-1",
 "type": "BASE64",
 "name": "Base64解码",
 "outputRef": "decoded",
 "config": {
 "operation": "decode",
 "inputSourceType": "inputMapping",
 "inputMappingRef": "plain"
 }
 }
 ],
 "inputMappings": [
 {
 "id": "input-1",
 "name": "原文",
 "inputRef": "plain",
 "defaultValue": ""
 }
 ],
 "outputMappings": [
 {
 "id": "result",
 "name": "结果",
 "componentRef": "decoded"
 }
 ]
 }))
 .unwrap()
 }

 #[test]
 fn process_har_validates_required_paths_and_refs() {
 let mut req = request(String::new(), "out.har".to_string());
 assert!(process_har(req.clone())
 .unwrap_err()
 .contains("输入文件路径不能为空"));

 req.input_path = "in.har".to_string();
 req.output_path = String::new();
 assert!(process_har(req.clone())
 .unwrap_err()
 .contains("输出文件路径不能为空"));

 req.output_path = "out.har".to_string();
 req.input_original_ref = String::new();
 assert!(process_har(req.clone())
 .unwrap_err()
 .contains("inputOriginalRef 不能为空"));

 req.input_original_ref = "plain".to_string();
 req.final_output_mapping_id = String::new();
 assert!(process_har(req)
 .unwrap_err()
 .contains("finalOutputMappingId 不能为空"));
 }

 #[test]
 fn process_har_rejects_unsupported_regex_preset() {
 let mut req = request("in.har".to_string(), "out.har".to_string());
 req.regex_preset = "TEXT".to_string();
 assert!(process_har(req)
 .unwrap_err()
 .contains("不支持的 regexPreset"));
 }

 #[test]
 fn process_har_requires_saved_project_config() {
 let dir = tempdir().unwrap();
 let input = dir.path().join("sample.har");
 let output = dir.path().join("nested").join("sample_解密.har");

 fs::write(
 &input,
 r#"{"log":{"entries":[{"response":{"content":{"text":"abc"}}}]}}"#,
 )
 .unwrap();

 let error = process_har(request(
 input.display().to_string(),
 output.display().to_string(),
 ))
 .unwrap_err();

 assert!(error.contains("找不到项目配置"));
 assert!(!output.exists());
 }

 #[test]
 fn matches_preset_rejects_pure_hex_for_base64() {
 let hex_payload = "AF4DEF2573017ADDB35D8E4C96C77E27";

 assert!(!matches_preset(hex_payload, RegexPreset::Base64));
 assert!(matches_preset(hex_payload, RegexPreset::Hex));
 assert!(!matches_preset("2", RegexPreset::Hex));
 }

 #[test]
 fn matches_preset_accepts_non_hex_base64() {
 let base64_payload = "eyJyZXN1bHQiOiJvayJ9";

 assert!(matches_preset(base64_payload, RegexPreset::Base64));
 assert!(!matches_preset(base64_payload, RegexPreset::Hex));
 }

 #[test]
 fn matches_preset_rejects_empty() {
 assert!(!matches_preset("", RegexPreset::Base64));
 }

 #[test]
 fn parse_regex_preset_normalizes_case_and_rejects_unknown() {
 assert_eq!(parse_regex_preset("base64").unwrap(), RegexPreset::Base64);
 assert_eq!(parse_regex_preset(" hex ").unwrap(), RegexPreset::Hex);
 assert!(parse_regex_preset("TEXT").unwrap_err().contains("不支持"));
 }

 #[test]
 fn rewrite_form_url_encoded_text_should_rewrite_raw_hex_field() {
 let form = "ccbParam=CFC40E3F788DA58F&rescrypt=2&reqcrypt=0";

 let rewritten = rewrite_form_url_encoded_text(form, RegexPreset::Hex, |value| {
 Some(format!("decoded-{value}"))
 });

 assert_eq!(
 rewritten.unwrap(),
 "ccbParam=decoded-CFC40E3F788DA58F&rescrypt=2&reqcrypt=0"
 );
 }

 #[test]
 fn rewrite_form_url_encoded_text_should_encode_rewritten_raw_field() {
 let form = "ccbParam=CFC40E3F788DA58F&rescrypt=2";

 let rewritten =
 rewrite_form_url_encoded_text(form, RegexPreset::Hex, |_value| Some("a=1&b=2".into()));

 assert_eq!(rewritten.unwrap(), "ccbParam=a%3D1%26b%3D2&rescrypt=2");
 }

 #[test]
 fn rewrite_form_url_encoded_text_should_rewrite_url_encoded_field() {
 let form = "payload=%43%46%43%34%30%45%33%46%37%38%38%44%41%35%38%46&other=ok";

 let rewritten = rewrite_form_url_encoded_text(form, RegexPreset::Hex, |value| {
 Some(format!(r#"{{"value":"{value}"}}"#))
 });

 assert_eq!(
 rewritten.unwrap(),
 "payload=%7B%22value%22%3A%22CFC40E3F788DA58F%22%7D&other=ok"
 );
 }

 #[test]
 fn process_har_json_rewrites_direct_text_with_workflow_result() {
 let root = json!({
 "log": {
 "entries": [
 {
 "response": {
 "content": {
 "text": "aGVsbG8="
 }
 }
 }
 ]
 }
 });

 let (processed, stats) = process_har_json_with_config(
 root,
 &request("in.har".into(), "out.har".into()),
 &base64_decode_config(),
 )
 .unwrap();

 assert_eq!(
 processed["log"]["entries"][0]["response"]["content"]["text"],
 "hello"
 );
 assert_eq!(stats.total_text_fields, 1);
 assert_eq!(stats.matched, 1);
 assert_eq!(stats.success, 1);
 assert_eq!(stats.failed, 0);
 }

 #[test]
 fn process_har_json_rewrites_embedded_json_text_compactly() {
 let root = json!({
 "text": "{\"payload\":\"aGVsbG8=\",\"keep\":\"ok\"}"
 });

 let (processed, stats) = process_har_json_with_config(
 root,
 &request("in.har".into(), "out.har".into()),
 &base64_decode_config(),
 )
 .unwrap();

 assert_eq!(processed["text"], r#"{"keep":"ok","payload":"hello"}"#);
 assert_eq!(stats.total_text_fields, 1);
 assert_eq!(stats.matched, 1);
 assert_eq!(stats.success, 1);
 assert_eq!(stats.failed, 0);
 }

 #[test]
 fn process_har_json_overlays_input_values_for_workflow() {
 let mut req = request("in.har".into(), "out.har".into());
 req.regex_preset = "HEX".to_string();
 req.input_original_ref = "payload".to_string();
 req.input_values = HashMap::from([("prefix".to_string(), "decoded".to_string())]);

 let config = serde_json::from_value(json!({
 "components": [
 {
 "id": "component-1",
 "type": "HEX",
 "outputRef": "plain",
 "config": {
 "operation": "decode",
 "inputSourceType": "inputMapping",
 "inputMappingRef": "payload"
 }
 },
 {
 "id": "component-2",
 "type": "BASE64",
 "outputRef": "resultRef",
 "config": {
 "operation": "encode",
 "inputSourceType": "expression",
 "inputExpression": "${prefix}:${plain}"
 }
 }
 ],
 "inputMappings": [
 { "inputRef": "payload", "defaultValue": "" },
 { "inputRef": "prefix", "defaultValue": "default" }
 ],
 "outputMappings": [
 { "id": "result", "componentRef": "resultRef" }
 ]
 }))
 .unwrap();

 let root = json!({ "text": "68656c6c6f" });
 let (processed, stats) = process_har_json_with_config(root, &req, &config).unwrap();

 assert_eq!(processed["text"], "ZGVjb2RlZDpoZWxsbw==");
 assert_eq!(stats.matched, 1);
 assert_eq!(stats.success, 1);
 }

 #[test]
 fn component_source_still_resolves_input_mapping_when_ref_points_to_input() {
 let mut req = request("in.har".into(), "out.har".into());
 req.regex_preset = "HEX".to_string();
 req.input_original_ref = "payload".to_string();

 let config = serde_json::from_value(json!({
 "components": [
 {
 "type": "HEX",
 "outputRef": "plain",
 "config": {
 "operation": "decode",
 "inputSourceType": "component",
 "inputMappingRef": "payload"
 }
 }
 ],
 "inputMappings": [
 { "inputRef": "payload", "defaultValue": "" }
 ],
 "outputMappings": [
 { "id": "result", "componentRef": "plain" }
 ]
 }))
 .unwrap();

 let (processed, stats) =
 process_har_json_with_config(json!({ "text": "68656c6c6f" }), &req, &config).unwrap();

 assert_eq!(processed["text"], "hello");
 assert_eq!(stats.success, 1);
 }
}
