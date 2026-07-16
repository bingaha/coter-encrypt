use std::{collections::HashMap, time::Instant};

use serde::{Deserialize, Deserializer, Serialize};

use crate::expression;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchExecutionRequest {
 #[serde(default)]
 pub components: Vec<EncryptionRequest>,
 #[serde(default)]
 pub input_values: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionRequest {
 pub algorithm: String,
 #[serde(default = "default_operation")]
 #[serde(deserialize_with = "deserialize_null_default_operation")]
 pub operation: String,
 #[serde(default)]
 pub data: String,
 #[serde(default)]
 pub key: Option<String>,
 #[serde(default)]
 pub iv: Option<String>,
 #[serde(default)]
 pub mode: Option<String>,
 #[serde(default)]
 pub padding: Option<String>,
 #[serde(default = "default_charset")]
 #[serde(deserialize_with = "deserialize_null_default_charset")]
 pub charset: String,
 #[serde(default)]
 pub public_key: Option<String>,
 #[serde(default)]
 pub private_key: Option<String>,
 #[serde(default = "default_input_format")]
 #[serde(deserialize_with = "deserialize_null_default_input_format")]
 pub input_format: String,
 #[serde(default = "default_output_format")]
 #[serde(deserialize_with = "deserialize_null_default_output_format")]
 pub output_format: String,
 #[serde(default = "default_key_format")]
 #[serde(deserialize_with = "deserialize_null_default_key_format")]
 pub key_format: String,
 #[serde(default = "default_iv_format")]
 #[serde(deserialize_with = "deserialize_null_default_iv_format")]
 pub iv_format: String,
 #[serde(default = "default_result_format")]
 #[serde(deserialize_with = "deserialize_null_default_result_format")]
 pub result_format: String,
 #[serde(default)]
 pub output_length: Option<u32>,
 #[serde(default)]
 pub input_base: Option<u32>,
 #[serde(default)]
 pub output_base: Option<u32>,
 #[serde(default)]
 pub sha_type: Option<String>,
 #[serde(default)]
 pub hmac_sha_type: Option<String>,
 #[serde(default = "default_unicode_format")]
 #[serde(deserialize_with = "deserialize_null_default_unicode_format")]
 pub format: String,
 #[serde(default = "default_hex_case")]
 #[serde(deserialize_with = "deserialize_null_default_hex_case")]
 pub hex_case: String,
 #[serde(default)]
 pub key_ref: Option<String>,
 #[serde(default)]
 pub iv_ref: Option<String>,
 #[serde(default)]
 pub public_key_ref: Option<String>,
 #[serde(default)]
 pub private_key_ref: Option<String>,
 #[serde(default)]
 pub output_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionResponse {
 pub result: String,
 pub status: String,
 pub message: String,
 pub processing_time: u128,
}

pub fn execute_batch<F>(request: BatchExecutionRequest, mut processor: F) -> Vec<EncryptionResponse>
where
 F: FnMut(&EncryptionRequest) -> Result<String, String>,
{
 let mut results = Vec::with_capacity(request.components.len());
 let mut component_outputs = HashMap::new();

 for component in request.components {
 let started = Instant::now();
 let response = execute_component(
 component,
 &request.input_values,
 &mut component_outputs,
 &mut processor,
 );
 results.push(EncryptionResponse {
 processing_time: started.elapsed().as_millis(),
 ..response
 });
 }

 results
}

fn execute_component<F>(
 mut component: EncryptionRequest,
 input_values: &HashMap<String, String>,
 component_outputs: &mut HashMap<String, String>,
 processor: &mut F,
) -> EncryptionResponse
where
 F: FnMut(&EncryptionRequest) -> Result<String, String>,
{
 match resolve_all_refs(&mut component, input_values, component_outputs)
 .and_then(|_| processor(&component))
 {
 Ok(result) => {
 if let Some(output_ref) = component
 .output_ref
 .as_deref()
 .filter(|value| !value.is_empty())
 {
 component_outputs.insert(output_ref.to_string(), result.clone());
 }

 EncryptionResponse {
 result,
 status: "success".to_string(),
 message: String::new(),
 processing_time: 0,
 }
 }
 Err(message) => EncryptionResponse {
 result: String::new(),
 status: "error".to_string(),
 message,
 processing_time: 0,
 },
 }
}

pub fn resolve_all_refs(
 request: &mut EncryptionRequest,
 input_values: &HashMap<String, String>,
 component_outputs: &HashMap<String, String>,
) -> Result<(), String> {
 if let Some(value) =
 resolve_optional_ref(request.key_ref.as_deref(), input_values, component_outputs)?
 {
 request.key = Some(value);
 }

 if let Some(value) =
 resolve_optional_ref(request.iv_ref.as_deref(), input_values, component_outputs)?
 {
 request.iv = Some(value);
 }

 if let Some(value) = resolve_optional_ref(
 request.public_key_ref.as_deref(),
 input_values,
 component_outputs,
 )? {
 request.public_key = Some(value);
 }

 if let Some(value) = resolve_optional_ref(
 request.private_key_ref.as_deref(),
 input_values,
 component_outputs,
 )? {
 request.private_key = Some(value);
 }

 if !request.data.is_empty() {
 if request.data.starts_with("input:") || request.data.starts_with("output:") {
 request.data = resolve_ref(&request.data, input_values, component_outputs)?;
 } else if request.data.contains("${") {
 request.data =
 resolve_expression_with_values(&request.data, input_values, component_outputs);
 } else if let Some(value) = component_outputs.get(&request.data) {
 request.data = value.clone();
 } else if let Some(value) = input_values.get(&request.data) {
 request.data = value.clone();
 }
 }

 Ok(())
}

fn resolve_optional_ref(
 ref_value: Option<&str>,
 input_values: &HashMap<String, String>,
 component_outputs: &HashMap<String, String>,
) -> Result<Option<String>, String> {
 match ref_value.filter(|value| !value.is_empty()) {
 Some(value) => resolve_ref(value, input_values, component_outputs).map(Some),
 None => Ok(None),
 }
}

pub fn resolve_ref(
 ref_value: &str,
 input_values: &HashMap<String, String>,
 component_outputs: &HashMap<String, String>,
) -> Result<String, String> {
 let Some((ref_type, id)) = ref_value.split_once(':') else {
 return Err(format!("无效的参数引用格式: {ref_value}"));
 };

 if id.is_empty() {
 return Err(format!("无效的参数引用格式: {ref_value}（标识符为空）"));
 }

 match ref_type {
 "input" => input_values
 .get(id)
 .cloned()
 .ok_or_else(|| format!("输入映射不存在: {id}")),
 "output" => component_outputs
 .get(id)
 .cloned()
 .ok_or_else(|| format!("组件输出不存在: {id}")),
 _ => Err(format!("未知的引用类型: {ref_type}")),
 }
}

pub fn resolve_expression_with_values(
 expression: &str,
 input_values: &HashMap<String, String>,
 component_outputs: &HashMap<String, String>,
) -> String {
 let mut values = input_values.clone();
 values.extend(component_outputs.clone());
 expression::resolve_expression(expression, &values)
}

fn default_operation() -> String {
 "encrypt".to_string()
}

fn default_charset() -> String {
 "UTF-8".to_string()
}

fn default_input_format() -> String {
 "hex".to_string()
}

fn default_output_format() -> String {
 "hex".to_string()
}

fn default_key_format() -> String {
 "text".to_string()
}

fn default_iv_format() -> String {
 "text".to_string()
}

fn default_result_format() -> String {
 "lowercase".to_string()
}

fn default_unicode_format() -> String {
 "standard".to_string()
}

fn default_hex_case() -> String {
 "uppercase".to_string()
}

fn deserialize_null_string_default<'de, D>(
 deserializer: D,
 default: fn() -> String,
) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_else(default))
}

fn deserialize_null_default_operation<'de, D>(deserializer: D) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 deserialize_null_string_default(deserializer, default_operation)
}

fn deserialize_null_default_charset<'de, D>(deserializer: D) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 deserialize_null_string_default(deserializer, default_charset)
}

fn deserialize_null_default_input_format<'de, D>(deserializer: D) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 deserialize_null_string_default(deserializer, default_input_format)
}

fn deserialize_null_default_output_format<'de, D>(deserializer: D) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 deserialize_null_string_default(deserializer, default_output_format)
}

fn deserialize_null_default_key_format<'de, D>(deserializer: D) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 deserialize_null_string_default(deserializer, default_key_format)
}

fn deserialize_null_default_iv_format<'de, D>(deserializer: D) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 deserialize_null_string_default(deserializer, default_iv_format)
}

fn deserialize_null_default_result_format<'de, D>(deserializer: D) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 deserialize_null_string_default(deserializer, default_result_format)
}

fn deserialize_null_default_unicode_format<'de, D>(deserializer: D) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 deserialize_null_string_default(deserializer, default_unicode_format)
}

fn deserialize_null_default_hex_case<'de, D>(deserializer: D) -> Result<String, D::Error>
where
 D: Deserializer<'de>,
{
 deserialize_null_string_default(deserializer, default_hex_case)
}

#[cfg(test)]
mod tests {
 use std::collections::HashMap;

 use super::{
 execute_batch, resolve_expression_with_values, resolve_ref, BatchExecutionRequest,
 EncryptionRequest,
 };

 fn algorithm_component(algorithm: &str, data: &str, output_ref: &str) -> EncryptionRequest {
 EncryptionRequest {
 algorithm: algorithm.to_string(),
 operation: "encrypt".to_string(),
 data: data.to_string(),
 key: None,
 iv: None,
 mode: None,
 padding: None,
 charset: "UTF-8".to_string(),
 public_key: None,
 private_key: None,
 input_format: "hex".to_string(),
 output_format: "hex".to_string(),
 key_format: "text".to_string(),
 iv_format: "text".to_string(),
 result_format: "lowercase".to_string(),
 output_length: None,
 input_base: None,
 output_base: None,
 sha_type: None,
 hmac_sha_type: None,
 format: "standard".to_string(),
 hex_case: "uppercase".to_string(),
 key_ref: None,
 iv_ref: None,
 public_key_ref: None,
 private_key_ref: None,
 output_ref: Some(output_ref.to_string()),
 }
 }

 fn component(data: &str, output_ref: &str) -> EncryptionRequest {
 algorithm_component("BASE64", data, output_ref)
 }

 #[test]
 fn resolves_explicit_refs() {
 let mut inputs = HashMap::new();
 inputs.insert("plain".to_string(), "hello".to_string());
 let mut outputs = HashMap::new();
 outputs.insert("prev".to_string(), "world".to_string());

 assert_eq!(
 resolve_ref("input:plain", &inputs, &outputs).unwrap(),
 "hello"
 );
 assert_eq!(
 resolve_ref("output:prev", &inputs, &outputs).unwrap(),
 "world"
 );
 assert!(resolve_ref("output:missing", &inputs, &outputs)
 .unwrap_err()
 .contains("组件输出不存在"));
 }

 #[test]
 fn resolves_expression_like_frontend_behavior() {
 let mut inputs = HashMap::new();
 inputs.insert("name".to_string(), "Rust".to_string());
 let outputs = HashMap::new();

 assert_eq!(
 resolve_expression_with_values(
 "hello ${name} ${missing} /$ /{ /} //",
 &inputs,
 &outputs
 ),
 "hello Rust  $ { } /"
 );
 }

 #[test]
 fn execute_batch_runs_processor_and_passes_output_to_later_components() {
 let mut inputs = HashMap::new();
 inputs.insert("plain".to_string(), "hello".to_string());

 let results = execute_batch(
 BatchExecutionRequest {
 components: vec![
 component("input:plain", "out1"),
 component("output:out1", "out2"),
 ],
 input_values: inputs,
 },
 |component| Ok(format!("{}:{}", component.algorithm, component.data)),
 );

 assert_eq!(results.len(), 2);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[0].result, "BASE64:hello");
 assert_eq!(results[1].status, "success");
 assert_eq!(results[1].result, "BASE64:BASE64:hello");
 }

 #[test]
 fn execute_batch_keeps_processor_error_as_per_component_error() {
 let results = execute_batch(
 BatchExecutionRequest {
 components: vec![EncryptionRequest {
 algorithm: "AES".to_string(),
 ..component("abc", "out1")
 }],
 input_values: HashMap::new(),
 },
 |component| Err(format!("算法 {} 尚未迁移到 Rust", component.algorithm)),
 );

 assert_eq!(results.len(), 1);
 assert_eq!(results[0].status, "error");
 assert!(results[0].message.contains("AES 尚未迁移到 Rust"));
 }

 #[test]
 fn deserializes_null_string_fields_with_defaults() {
 let request: BatchExecutionRequest = serde_json::from_str(
 r#"{
 "components": [
 {
 "algorithm": "SHA",
 "operation": null,
 "data": "abc",
 "charset": null,
 "inputFormat": null,
 "outputFormat": null,
 "keyFormat": null,
 "ivFormat": null,
 "resultFormat": null,
 "format": null,
 "hexCase": null
 }
 ],
 "inputValues": {}
 }"#,
 )
 .unwrap();

 let component = &request.components[0];
 assert_eq!(component.operation, "encrypt");
 assert_eq!(component.charset, "UTF-8");
 assert_eq!(component.input_format, "hex");
 assert_eq!(component.output_format, "hex");
 assert_eq!(component.key_format, "text");
 assert_eq!(component.iv_format, "text");
 assert_eq!(component.result_format, "lowercase");
 assert_eq!(component.format, "standard");
 assert_eq!(component.hex_case, "uppercase");
 }
}
