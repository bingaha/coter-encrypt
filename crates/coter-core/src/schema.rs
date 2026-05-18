use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
 #[serde(default)]
 pub components: Vec<Component>,
 #[serde(default)]
 pub input_mappings: Vec<InputMapping>,
 #[serde(default)]
 pub output_mappings: Vec<OutputMapping>,
 #[serde(default)]
 pub execution_config: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Component {
 #[serde(default)]
 pub id: String,
 #[serde(default, rename = "type")]
 pub component_type: String,
 #[serde(default)]
 pub name: String,
 #[serde(default)]
 pub icon: String,
 #[serde(default)]
 pub output_ref: String,
 #[serde(default)]
 pub config: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputMapping {
 #[serde(default)]
 pub id: String,
 #[serde(default)]
 pub name: String,
 #[serde(default)]
 pub input_ref: String,
 #[serde(default)]
 pub default_value: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputMapping {
 #[serde(default)]
 pub id: String,
 #[serde(default)]
 pub name: String,
 #[serde(default)]
 pub component_ref: String,
}

#[cfg(test)]
mod tests {
 use super::ProjectConfig;

 #[test]
 fn project_config_accepts_current_frontend_shape() {
 let config: ProjectConfig = serde_json::from_str(
 r#"{
 "components": [
 {
 "id": "component-1",
 "type": "BASE64",
 "name": "Base64",
 "icon": "lock",
 "outputRef": "out-1",
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
 "defaultValue": "aGVsbG8="
 }
 ],
 "outputMappings": [
 {
 "id": "result",
 "name": "结果",
 "componentRef": "out-1"
 }
 ],
 "executionConfig": {
 "stopOnError": true
 }
 }"#,
 )
 .unwrap();

 assert_eq!(config.components.len(), 1);
 assert_eq!(config.components[0].component_type, "BASE64");
 assert_eq!(config.components[0].output_ref, "out-1");
 assert_eq!(config.input_mappings[0].input_ref, "plain");
 assert_eq!(config.output_mappings[0].component_ref, "out-1");
 assert_eq!(config.execution_config["stopOnError"], true);
 }

 #[test]
 fn project_config_defaults_missing_collections() {
 let config: ProjectConfig = serde_json::from_str("{}").unwrap();

 assert!(config.components.is_empty());
 assert!(config.input_mappings.is_empty());
 assert!(config.output_mappings.is_empty());
 assert!(config.execution_config.is_empty());
 }
}
