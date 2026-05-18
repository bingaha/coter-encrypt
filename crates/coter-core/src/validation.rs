use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
 expression::{self, ExpressionError},
 schema::{Component, InputMapping, ProjectConfig},
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigValidationResult {
 pub is_valid: bool,
 pub errors: Vec<ConfigValidationError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigValidationError {
 pub component_id: String,
 pub component_name: String,
 pub field: String,
 pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldValidationError {
 field: String,
 message: String,
}

pub fn validate_config(config: &ProjectConfig) -> ConfigValidationResult {
 let mut errors = Vec::new();

 for component in &config.components {
 for error in validate_component(component, config) {
 errors.push(ConfigValidationError {
 component_id: component.id.clone(),
 component_name: component.name.clone(),
 field: error.field,
 message: error.message,
 });
 }
 }

 ConfigValidationResult {
 is_valid: errors.is_empty(),
 errors,
 }
}

pub fn is_valid_ref(ref_value: &str, component_id: &str, config: &ProjectConfig) -> bool {
 let Some((ref_type, id)) = ref_value.split_once(':') else {
 return false;
 };

 match ref_type {
 "input" => config
 .input_mappings
 .iter()
 .any(|mapping| mapping.input_ref == id),
 "output" => get_available_input_sources(component_id, &config.components)
 .iter()
 .any(|source| source.output_ref == id),
 _ => false,
 }
}

fn validate_component(component: &Component, config: &ProjectConfig) -> Vec<FieldValidationError> {
 let mut errors = Vec::new();
 let component_config = &component.config;
 let input_source_type = config_string(component_config, "inputSourceType");

 if input_source_type.is_none() {
 errors.push(field_error("inputSourceType", "请选择输入来源类型"));
 } else {
 match input_source_type.as_deref().unwrap_or_default() {
 "reference" | "inputMapping" | "component" => {
 if config_string(component_config, "inputMappingRef")
 .is_none_or(|value| value.is_empty())
 {
 errors.push(field_error("inputMappingRef", "请选择输入来源"));
 }
 }
 "expression" => {
 if let Some(input_expression) = config_string(component_config, "inputExpression")
 .filter(|value| !value.is_empty())
 {
 let current_index = component_index(&config.components, &component.id);
 let available_refs = available_refs(current_index, config);
 let subsequent_refs = subsequent_refs(current_index, &config.components);
 let validation = expression::validate_expression_refs(
 &input_expression,
 &available_refs,
 &subsequent_refs,
 );

 for error in validation.errors {
 errors.push(field_error("inputExpression", &error.message));
 }
 }
 }
 _ => {}
 }
 }

 match component.component_type.as_str() {
 "aes" | "blowfish" | "sm4" => {
 validate_key_like_ref(
 &mut errors,
 component_config,
 "key",
 "keyRef",
 "请选择密钥来源",
 "密钥引用无效",
 component,
 config,
 );

 if let Some(mode) = config_string(component_config, "mode").filter(|mode| mode != "ECB")
 {
 validate_key_like_ref(
 &mut errors,
 component_config,
 "iv",
 "ivRef",
 &format!("{mode}模式下请选择IV向量来源"),
 "IV向量引用无效",
 component,
 config,
 );
 }
 }
 "sm2" | "rsa" => match config_string(component_config, "operation").as_deref() {
 Some("encrypt") => validate_key_like_ref(
 &mut errors,
 component_config,
 "publicKey",
 "publicKeyRef",
 "加密操作请选择公钥来源",
 "公钥引用无效",
 component,
 config,
 ),
 Some("decrypt") => validate_key_like_ref(
 &mut errors,
 component_config,
 "privateKey",
 "privateKeyRef",
 "解密操作请选择私钥来源",
 "私钥引用无效",
 component,
 config,
 ),
 _ => {}
 },
 "hmacmd5" | "hmacsha" => validate_key_like_ref(
 &mut errors,
 component_config,
 "key",
 "keyRef",
 "请选择密钥来源",
 "密钥引用无效",
 component,
 config,
 ),
 _ => {}
 }

 errors
}

fn validate_key_like_ref(
 errors: &mut Vec<FieldValidationError>,
 component_config: &Map<String, Value>,
 direct_field: &str,
 ref_field: &str,
 required_message: &str,
 invalid_message: &str,
 component: &Component,
 config: &ProjectConfig,
) {
 match config_string(component_config, ref_field).filter(|value| !value.is_empty()) {
 Some(ref_value) => {
 if !is_valid_ref(&ref_value, &component.id, config) {
 errors.push(field_error(ref_field, invalid_message));
 }
 }
 None => {
 if config_string(component_config, direct_field).is_none_or(|value| value.is_empty()) {
 errors.push(field_error(ref_field, required_message));
 }
 }
 }
}

fn get_available_input_sources(component_id: &str, components: &[Component]) -> Vec<Component> {
 let Some(current_index) = components
 .iter()
 .position(|component| component.id == component_id)
 else {
 return Vec::new();
 };

 if current_index == 0 {
 return Vec::new();
 }

 components[..current_index].to_vec()
}

fn available_refs(current_index: Option<usize>, config: &ProjectConfig) -> Vec<String> {
 let mut refs = Vec::new();

 if let Some(index) = current_index {
 refs.extend(
 config.components[..index]
 .iter()
 .map(|component| component.output_ref.clone()),
 );
 }

 refs.extend(
 config
 .input_mappings
 .iter()
 .map(|mapping: &InputMapping| mapping.input_ref.clone()),
 );

 refs
}

fn subsequent_refs(current_index: Option<usize>, components: &[Component]) -> Vec<String> {
 current_index.map_or_else(Vec::new, |index| {
 components[index + 1..]
 .iter()
 .map(|component| component.output_ref.clone())
 .collect()
 })
}

fn component_index(components: &[Component], component_id: &str) -> Option<usize> {
 components
 .iter()
 .position(|component| component.id == component_id)
}

fn config_string(config: &Map<String, Value>, key: &str) -> Option<String> {
 config.get(key).and_then(value_to_string)
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

fn field_error(field: &str, message: &str) -> FieldValidationError {
 FieldValidationError {
 field: field.to_string(),
 message: message.to_string(),
 }
}

#[allow(dead_code)]
fn _keep_expression_error_import(_: &ExpressionError) {}

#[cfg(test)]
mod tests {
 use serde_json::json;

 use super::{is_valid_ref, validate_config};
 use crate::schema::ProjectConfig;

 fn config(value: serde_json::Value) -> ProjectConfig {
 serde_json::from_value(value).unwrap()
 }

 #[test]
 fn validates_missing_input_source_type_and_input_mapping_ref() {
 let config = config(json!({
 "components": [
 {
 "id": "c1",
 "type": "base64",
 "name": "Base64",
 "outputRef": "out1",
 "config": {}
 },
 {
 "id": "c2",
 "type": "base64",
 "name": "Base64 2",
 "outputRef": "out2",
 "config": {
 "inputSourceType": "reference"
 }
 }
 ]
 }));

 let result = validate_config(&config);

 assert!(!result.is_valid);
 assert_eq!(result.errors.len(), 2);
 assert_eq!(result.errors[0].field, "inputSourceType");
 assert_eq!(result.errors[0].message, "请选择输入来源类型");
 assert_eq!(result.errors[1].field, "inputMappingRef");
 assert_eq!(result.errors[1].message, "请选择输入来源");
 }

 #[test]
 fn validates_expression_refs_against_inputs_previous_and_subsequent_outputs() {
 let config = config(json!({
 "components": [
 {
 "id": "c1",
 "type": "base64",
 "name": "Base64",
 "outputRef": "out1",
 "config": {
 "inputSourceType": "inputMapping",
 "inputMappingRef": "plain"
 }
 },
 {
 "id": "c2",
 "type": "base64",
 "name": "Base64 2",
 "outputRef": "out2",
 "config": {
 "inputSourceType": "expression",
 "inputExpression": "${plain}-${out1}-${out3}-${missing}"
 }
 },
 {
 "id": "c3",
 "type": "base64",
 "name": "Base64 3",
 "outputRef": "out3",
 "config": {
 "inputSourceType": "inputMapping",
 "inputMappingRef": "plain"
 }
 }
 ],
 "inputMappings": [
 { "inputRef": "plain" }
 ]
 }));

 let result = validate_config(&config);
 let expression_errors = result
 .errors
 .iter()
 .filter(|error| error.component_id == "c2" && error.field == "inputExpression")
 .map(|error| error.message.as_str())
 .collect::<Vec<_>>();

 assert_eq!(
 expression_errors,
 vec![
 "不能引用后续组件的输出：\"out3\"",
 "引用的变量 \"missing\" 不存在（请检查输入映射或组件输出）"
 ]
 );
 }

 #[test]
 fn validates_symmetric_key_and_iv_refs_like_pinia() {
 let config = config(json!({
 "components": [
 {
 "id": "c1",
 "type": "aes",
 "name": "AES",
 "outputRef": "out1",
 "config": {
 "inputSourceType": "expression",
 "inputExpression": "plain",
 "mode": "CBC"
 }
 }
 ]
 }));

 let result = validate_config(&config);
 let fields = result
 .errors
 .iter()
 .map(|error| (error.field.as_str(), error.message.as_str()))
 .collect::<Vec<_>>();

 assert!(fields.contains(&("keyRef", "请选择密钥来源")));
 assert!(fields.contains(&("ivRef", "CBC模式下请选择IV向量来源")));
 }

 #[test]
 fn validates_asymmetric_key_by_operation() {
 let config = config(json!({
 "components": [
 {
 "id": "rsa-1",
 "type": "rsa",
 "name": "RSA",
 "outputRef": "rsaOut",
 "config": {
 "inputSourceType": "expression",
 "inputExpression": "plain",
 "operation": "encrypt"
 }
 },
 {
 "id": "sm2-1",
 "type": "sm2",
 "name": "SM2",
 "outputRef": "sm2Out",
 "config": {
 "inputSourceType": "expression",
 "inputExpression": "plain",
 "operation": "decrypt"
 }
 }
 ]
 }));

 let result = validate_config(&config);
 let fields = result
 .errors
 .iter()
 .map(|error| {
 (
 error.component_id.as_str(),
 error.field.as_str(),
 error.message.as_str(),
 )
 })
 .collect::<Vec<_>>();

 assert!(fields.contains(&("rsa-1", "publicKeyRef", "加密操作请选择公钥来源")));
 assert!(fields.contains(&("sm2-1", "privateKeyRef", "解密操作请选择私钥来源")));
 }

 #[test]
 fn validates_hmac_key_ref_and_accepts_direct_key() {
 let config = config(json!({
 "components": [
 {
 "id": "hmac-1",
 "type": "hmacsha",
 "name": "HMAC",
 "outputRef": "hmacOut",
 "config": {
 "inputSourceType": "expression",
 "inputExpression": "plain"
 }
 },
 {
 "id": "hmac-2",
 "type": "hmacsha",
 "name": "HMAC 2",
 "outputRef": "hmacOut2",
 "config": {
 "inputSourceType": "expression",
 "inputExpression": "plain",
 "key": "secret"
 }
 }
 ]
 }));

 let result = validate_config(&config);

 assert!(result
 .errors
 .iter()
 .any(|error| error.component_id == "hmac-1" && error.field == "keyRef"));
 assert!(!result
 .errors
 .iter()
 .any(|error| error.component_id == "hmac-2" && error.field == "keyRef"));
 }

 #[test]
 fn validates_input_and_output_ref_formats() {
 let config = config(json!({
 "components": [
 {
 "id": "c1",
 "type": "base64",
 "name": "Base64",
 "outputRef": "out1",
 "config": {}
 },
 {
 "id": "c2",
 "type": "base64",
 "name": "Base64 2",
 "outputRef": "out2",
 "config": {}
 }
 ],
 "inputMappings": [
 { "inputRef": "plain" }
 ]
 }));

 assert!(is_valid_ref("input:plain", "c2", &config));
 assert!(!is_valid_ref("input:missing", "c2", &config));
 assert!(is_valid_ref("output:out1", "c2", &config));
 assert!(!is_valid_ref("output:out2", "c2", &config));
 assert!(!is_valid_ref("bad:out1", "c2", &config));
 assert!(!is_valid_ref("output", "c2", &config));
 }
}
