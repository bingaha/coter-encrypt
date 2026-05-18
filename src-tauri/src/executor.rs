pub use coter_core::executor::{BatchExecutionRequest, EncryptionRequest, EncryptionResponse};

use crate::crypto;

pub fn execute_batch(request: BatchExecutionRequest) -> Vec<EncryptionResponse> {
 coter_core::executor::execute_batch(request, process_algorithm)
}

fn process_algorithm(request: &EncryptionRequest) -> Result<String, String> {
 match request.algorithm.to_uppercase().as_str() {
 "BASE64" => crypto::process_base64(&request.data, &request.operation),
 "HEX" => crypto::process_hex(&request.data, &request.operation),
 "RADIX" => crypto::process_radix(
 &request.data,
 request.input_base,
 request.output_base,
 &request.hex_case,
 ),
 "URL" => crypto::process_url(&request.data, &request.charset, &request.operation),
 "UNICODE" => crypto::process_unicode(&request.data, &request.format, &request.operation),
 "MD5" => crypto::process_md5(&request.data, request.output_length, &request.result_format),
 "SHA" => crypto::process_sha(
 &request.data,
 request.sha_type.as_deref(),
 &request.result_format,
 ),
 "SHA256" => crypto::process_sha(&request.data, Some("SHA256"), &request.result_format),
 "SM3" => crypto::process_sm3(&request.data, &request.result_format),
 "HMACSHA" => crypto::process_hmac_sha(
 &request.data,
 request.key.as_deref(),
 request.hmac_sha_type.as_deref(),
 &request.result_format,
 ),
 "HMACSHA256" => crypto::process_hmac_sha(
 &request.data,
 request.key.as_deref(),
 Some("HmacSHA256"),
 &request.result_format,
 ),
 "HMACMD5" => crypto::process_hmac_md5(
 &request.data,
 request.key.as_deref(),
 request.output_length,
 &request.result_format,
 ),
 "AES" => crypto::process_aes(
 request.key.as_deref(),
 request.iv.as_deref(),
 request.mode.as_deref(),
 request.padding.as_deref(),
 &request.input_format,
 &request.output_format,
 &request.key_format,
 &request.iv_format,
 &request.hex_case,
 &request.data,
 &request.operation,
 ),
 "BLOWFISH" => crypto::process_blowfish(
 request.key.as_deref(),
 request.iv.as_deref(),
 request.mode.as_deref(),
 request.padding.as_deref(),
 &request.input_format,
 &request.output_format,
 &request.key_format,
 &request.iv_format,
 &request.hex_case,
 &request.data,
 &request.operation,
 ),
 "SM4" => crypto::process_sm4(
 request.key.as_deref(),
 request.iv.as_deref(),
 request.mode.as_deref(),
 request.padding.as_deref(),
 &request.input_format,
 &request.output_format,
 &request.key_format,
 &request.iv_format,
 &request.hex_case,
 &request.data,
 &request.operation,
 ),
 "SM2" => crypto::process_sm2(
 request.public_key.as_deref(),
 request.private_key.as_deref(),
 request.mode.as_deref(),
 &request.input_format,
 &request.output_format,
 &request.hex_case,
 &request.data,
 &request.operation,
 ),
 "RSA" => crypto::process_rsa(
 request.public_key.as_deref(),
 request.private_key.as_deref(),
 request.padding.as_deref(),
 &request.input_format,
 &request.output_format,
 &request.hex_case,
 &request.data,
 &request.operation,
 ),
 algorithm => Err(format!(
 "算法 {algorithm} 尚未迁移到 Rust，后续 M2-3b/M4 将实现该本地算法"
 )),
 }
}

#[cfg(test)]
mod tests {
 use std::collections::HashMap;

 use coter_core::executor::{BatchExecutionRequest, EncryptionRequest};
 use coter_core::schema::ProjectConfig;
 use serde_json::{Map, Value};

 use super::execute_batch;

 const PROJECT_RSA_PRIVATE_KEY: &str = "MIICdgIBADANBgkqhkiG9w0BAQEFAASCAmAwggJcAgEAAoGBAJKf8oCY21bhRnx8nldD2evjW69K4OrbGuG3FUH/b+qrhw95qUBHjaNIH5QB9kO4HHsPAxhV5snGaXHSenuXCxZFBWe5WG9cdW7dI/7YhhPGOY0l0ywjiMe5wHhbycpIRmfbyCfWtpU36KtMv9t75pQTKjbcJjQtPOtQ+v5OejBVAgMBAAECgYAlqia+OAXoHHhh1BVMr2ZUfRP5RI/gZKZUIxa33GkgbC2GoScEFx1gO0+5UoOzQ6E1T1bpMm/Vlz1Q+tNx2gwDqOy1839z65ZAggm9buEN9j9E3NOztia1wRtAgH5C3/Aflo192HZTjYzzVI5ZDLN7A2Y76MmLMzqVxNY7vf+hRQJBAN9nEH9x4ZIncrBhlClObE5aBn00l31LbhoetNFFIIwZ9kAAa/jJAgD2Ddky87YQLYyYiYqQTKUp018dxZN+KbsCQQCoBPC3cEmHLk8D1dAd9VOCUQKDcka0Gfs9eXE/cp9kQ+4AfbfaZcQuYPwqF9ZQqfylLQGARw8xfhX98CEsEaUvAkEAr6puSZh1xCQqxdDk3RoihfW6Noe9OzOt7vIIQqn1rtTXUnpCbI06eyD/wLOU+at89ZoYRRG0gwcBg0B41MKW8wJANurudzbngZzcTMedL72ZHxY1eRtoCsQXP5+rKW7gtFgTuetdpa/vsK0YnvWNom39W0vbmr8fMzEgJRFQ9mOKFwJADY9VWYxOdf/5AjimWA1gt+mtJqZB+2jdJYyoR16cWm5sMVrEz18vVPJeh0qOmXOO7ClFs9UUVtTabiszuO+wRw==";
 const PROJECT_RSA_PUBLIC_KEY_FROM_PRIVATE: &str = "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQCSn/KAmNtW4UZ8fJ5XQ9nr41uvSuDq2xrhtxVB/2/qq4cPealAR42jSB+UAfZDuBx7DwMYVebJxmlx0np7lwsWRQVnuVhvXHVu3SP+2IYTxjmNJdMsI4jHucB4W8nKSEZn28gn1raVN+irTL/be+aUEyo23CY0LTzrUPr+TnowVQIDAQAB";
 const SM2_PRIVATE_D: &str = "9998894D66977D5F2C68B7E0564DFBFB36EE5AFD5520F7FDA1AF6E7D6ACAA874";
 const SM2_PUBLIC_Q: &str = "049031694836FCCD075D20CC284278901F37AF7D1EF8DEA025393C4643CE577C9DB64DF3E331ECC5B105E40E6C65949B6B5F6E8F1D99D28B6E01539DAE723588F0";

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

 fn batch_from_project_config(
 config_json: &str,
 overrides: HashMap<String, String>,
 ) -> BatchExecutionRequest {
 let config: ProjectConfig = serde_json::from_str(config_json).unwrap();
 let input_refs = config
 .input_mappings
 .iter()
 .map(|mapping| mapping.input_ref.clone())
 .collect::<std::collections::HashSet<_>>();
 let mut input_values = config
 .input_mappings
 .iter()
 .map(|mapping| (mapping.input_ref.clone(), mapping.default_value.clone()))
 .collect::<HashMap<_, _>>();
 input_values.extend(overrides);

 let components = config
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
 "expression" => {
 config_string_or_default(component_config, "inputExpression", "")
 }
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
 output_format: config_string_or_default(
 component_config,
 "outputFormat",
 "hex",
 ),
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
 .collect();

 BatchExecutionRequest {
 components,
 input_values,
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

 #[test]
 fn execute_batch_runs_base64_and_passes_output_to_later_components() {
 let mut inputs = HashMap::new();
 inputs.insert("plain".to_string(), "hello".to_string());

 let results = execute_batch(BatchExecutionRequest {
 components: vec![
 component("input:plain", "out1"),
 component("output:out1", "out2"),
 ],
 input_values: inputs,
 });

 assert_eq!(results.len(), 2);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[0].result, "aGVsbG8=");
 assert_eq!(results[1].status, "success");
 assert_eq!(results[1].result, "YUdWc2JHOD0=");
 }

 #[test]
 fn execute_batch_runs_aes_with_key_ref() {
 let mut inputs = HashMap::new();
 inputs.insert("plain".to_string(), "中文ABC".to_string());
 inputs.insert("key".to_string(), "d3k7s8c4n2m1s5b9".to_string());

 let mut encrypt = algorithm_component("AES", "input:plain", "aes");
 encrypt.key_ref = Some("input:key".to_string());
 encrypt.key_format = "text".to_string();
 encrypt.input_format = "text".to_string();
 encrypt.output_format = "base64".to_string();
 encrypt.mode = Some("ECB".to_string());
 encrypt.padding = Some("PKCS5Padding".to_string());

 let mut decrypt = algorithm_component("AES", "output:aes", "plain-out");
 decrypt.operation = "decrypt".to_string();
 decrypt.key_ref = Some("input:key".to_string());
 decrypt.key_format = "text".to_string();
 decrypt.input_format = "base64".to_string();
 decrypt.output_format = "hex".to_string();
 decrypt.mode = Some("ECB".to_string());
 decrypt.padding = Some("PKCS5Padding".to_string());

 let results = execute_batch(BatchExecutionRequest {
 components: vec![encrypt, decrypt],
 input_values: inputs,
 });

 assert_eq!(results.len(), 2);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[0].result, "maSZYCez1P5rOLu0Bpr2BA==");
 assert_eq!(results[1].status, "success");
 assert_eq!(results[1].result, "中文ABC");
 }

 #[test]
 fn execute_batch_runs_sm4_with_key_ref() {
 let mut inputs = HashMap::new();
 inputs.insert("plain".to_string(), "中文ABC".to_string());
 inputs.insert("key".to_string(), "Li8CW5HpIDSwMKog".to_string());

 let mut encrypt = algorithm_component("SM4", "input:plain", "sm4");
 encrypt.key_ref = Some("input:key".to_string());
 encrypt.key_format = "text".to_string();
 encrypt.input_format = "text".to_string();
 encrypt.output_format = "base64".to_string();
 encrypt.mode = Some("ECB".to_string());
 encrypt.padding = Some("pkcs7".to_string());

 let mut decrypt = algorithm_component("SM4", "output:sm4", "plain-out");
 decrypt.operation = "decrypt".to_string();
 decrypt.key_ref = Some("input:key".to_string());
 decrypt.key_format = "text".to_string();
 decrypt.input_format = "base64".to_string();
 decrypt.output_format = "hex".to_string();
 decrypt.mode = Some("ECB".to_string());
 decrypt.padding = Some("pkcs7".to_string());

 let results = execute_batch(BatchExecutionRequest {
 components: vec![encrypt, decrypt],
 input_values: inputs,
 });

 assert_eq!(results.len(), 2);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[0].result, "ozajx+RY9q4hdBsTbwlFog==");
 assert_eq!(results[1].status, "success");
 assert_eq!(results[1].result, "中文ABC");
 }

 #[test]
 fn execute_batch_runs_blowfish_with_key_and_iv_refs() {
 let mut inputs = HashMap::new();
 inputs.insert("plain".to_string(), "00000000000319cc".to_string());
 inputs.insert(
 "key".to_string(),
 "FZvIAvgfQRfKKzFjHi_hndypTFCSWVRl".to_string(),
 );
 inputs.insert("iv".to_string(), "0000000000000000".to_string());

 let mut encrypt = algorithm_component("BLOWFISH", "input:plain", "blowfish");
 encrypt.key_ref = Some("input:key".to_string());
 encrypt.iv_ref = Some("input:iv".to_string());
 encrypt.mode = Some("CBC".to_string());
 encrypt.padding = Some("None".to_string());
 encrypt.key_format = "text".to_string();
 encrypt.iv_format = "hex".to_string();
 encrypt.input_format = "hex".to_string();
 encrypt.output_format = "base64".to_string();

 let results = execute_batch(BatchExecutionRequest {
 components: vec![encrypt],
 input_values: inputs,
 });

 assert_eq!(results.len(), 1);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[0].result, "DQ8foMjgxwQ=");
 }

 #[test]
 fn execute_batch_runs_rsa_pkcs1_signature_with_private_key_ref() {
 let mut inputs = HashMap::new();
 inputs.insert(
 "hash".to_string(),
 "a9993e364706816aba3e25717850c26c9cd0d89d".to_string(),
 );
 inputs.insert(
 "private-key".to_string(),
 PROJECT_RSA_PRIVATE_KEY.to_string(),
 );

 let mut sign = algorithm_component("RSA", "input:hash", "sign");
 sign.operation = "decrypt".to_string();
 sign.padding = Some("PKCS1签名".to_string());
 sign.input_format = "hex".to_string();
 sign.output_format = "hex".to_string();
 sign.hex_case = "lowercase".to_string();
 sign.private_key_ref = Some("input:private-key".to_string());

 let results = execute_batch(BatchExecutionRequest {
 components: vec![sign],
 input_values: inputs,
 });

 assert_eq!(results.len(), 1);
 assert_eq!(results[0].status, "success");
 assert_eq!(
 results[0].result,
 "90fe104f9480b49f7b239502d59daa03e828a27fcc9ff2d18ce2fe9088f2f7536b1a10d67863471070923974152d8e3624efff4a577f3f02591dc7e47cac40f94f074cf1e05a22bfbdff4e668c81e70ee83b11ded1379ff9c25b121b58aadc71bb64d3876227c4d168027befa6b7bc87674a48877cf1c850c5f537c3581b4306"
 );
 }

 #[test]
 fn execute_batch_runs_rsa_oaep_with_key_refs() {
 let mut inputs = HashMap::new();
 inputs.insert("plain".to_string(), "中文ABC".to_string());
 inputs.insert(
 "public-key".to_string(),
 PROJECT_RSA_PUBLIC_KEY_FROM_PRIVATE.to_string(),
 );
 inputs.insert(
 "private-key".to_string(),
 PROJECT_RSA_PRIVATE_KEY.to_string(),
 );

 let mut encrypt = algorithm_component("RSA", "input:plain", "cipher");
 encrypt.padding = Some("OAEP".to_string());
 encrypt.output_format = "base64".to_string();
 encrypt.public_key_ref = Some("input:public-key".to_string());

 let mut decrypt = algorithm_component("RSA", "output:cipher", "plain-out");
 decrypt.operation = "decrypt".to_string();
 decrypt.padding = Some("OAEP".to_string());
 decrypt.input_format = "base64".to_string();
 decrypt.private_key_ref = Some("input:private-key".to_string());

 let results = execute_batch(BatchExecutionRequest {
 components: vec![encrypt, decrypt],
 input_values: inputs,
 });

 assert_eq!(results.len(), 2);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[1].status, "success");
 assert_eq!(results[1].result, "中文ABC");
 }

 #[test]
 fn execute_batch_runs_sm2_with_key_refs() {
 let mut inputs = HashMap::new();
 inputs.insert("plain".to_string(), "中文ABC".to_string());
 inputs.insert("public-key".to_string(), SM2_PUBLIC_Q.to_string());
 inputs.insert("private-key".to_string(), SM2_PRIVATE_D.to_string());

 let mut encrypt = algorithm_component("SM2", "input:plain", "cipher");
 encrypt.mode = Some("C1C3C2".to_string());
 encrypt.input_format = "text".to_string();
 encrypt.output_format = "base64".to_string();
 encrypt.public_key_ref = Some("input:public-key".to_string());

 let mut decrypt = algorithm_component("SM2", "output:cipher", "plain-out");
 decrypt.operation = "decrypt".to_string();
 decrypt.mode = Some("C1C3C2".to_string());
 decrypt.input_format = "base64".to_string();
 decrypt.private_key_ref = Some("input:private-key".to_string());

 let results = execute_batch(BatchExecutionRequest {
 components: vec![encrypt, decrypt],
 input_values: inputs,
 });

 assert_eq!(results.len(), 2);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[1].status, "success");
 assert_eq!(results[1].result, "中文ABC");
 }

 #[test]
 fn execute_batch_keeps_unsupported_algorithm_as_per_component_error() {
 let results = execute_batch(BatchExecutionRequest {
 components: vec![EncryptionRequest {
 algorithm: "TWOFISH".to_string(),
 ..component("abc", "out1")
 }],
 input_values: HashMap::new(),
 });

 assert_eq!(results.len(), 1);
 assert_eq!(results[0].status, "error");
 assert!(results[0].message.contains("TWOFISH 尚未迁移到 Rust"));
 }

 #[test]
 fn execute_batch_runs_url_and_unicode_algorithms() {
 let mut unicode_component = algorithm_component("UNICODE", r"\u4E2D\u6587", "plain");
 unicode_component.operation = "decode".to_string();

 let mut url_component = algorithm_component("URL", "output:plain", "url");
 url_component.operation = "encode".to_string();

 let results = execute_batch(BatchExecutionRequest {
 components: vec![unicode_component, url_component],
 input_values: HashMap::new(),
 });

 assert_eq!(results.len(), 2);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[0].result, "中文");
 assert_eq!(results[1].status, "success");
 assert_eq!(results[1].result, "%E4%B8%AD%E6%96%87");
 }

 #[test]
 fn execute_batch_runs_radix_algorithm() {
 let mut radix_component = algorithm_component("RADIX", "input:number", "radix");
 radix_component.input_base = Some(10);
 radix_component.output_base = Some(16);
 radix_component.hex_case = "uppercase".to_string();

 let results = execute_batch(BatchExecutionRequest {
 components: vec![radix_component],
 input_values: HashMap::from([("number".to_string(), "255".to_string())]),
 });

 assert_eq!(results.len(), 1);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[0].result, "FF");
 }

 #[test]
 fn execute_batch_runs_digest_and_hmac_algorithms() {
 let mut inputs = HashMap::new();
 inputs.insert("key".to_string(), "secret".to_string());

 let mut md5_component = algorithm_component("MD5", "abc", "md5");
 md5_component.output_length = Some(16);
 md5_component.result_format = "uppercase".to_string();

 let mut sha_component = algorithm_component("SHA", "abc", "sha");
 sha_component.sha_type = Some("SHA256".to_string());

 let mut hmac_component = algorithm_component("HMACSHA", "output:sha", "hmac");
 hmac_component.hmac_sha_type = Some("HmacSHA256".to_string());
 hmac_component.key_ref = Some("input:key".to_string());

 let results = execute_batch(BatchExecutionRequest {
 components: vec![md5_component, sha_component, hmac_component],
 input_values: inputs,
 });

 assert_eq!(results.len(), 3);
 assert_eq!(results[0].status, "success");
 assert_eq!(results[0].result, "3CD24FB0D6963F7D");
 assert_eq!(results[1].status, "success");
 assert_eq!(
 results[1].result,
 "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
 );
 assert_eq!(results[2].status, "success");
 assert_eq!(
 results[2].result,
 "936fad23863c049e73b4a957ee741164bce88e45d79da05058f7d0de1de62219"
 );
 }

 #[test]
 fn execute_batch_runs_sm3_algorithm() {
 let mut inputs = HashMap::new();
 inputs.insert("plain".to_string(), "中文ABC".to_string());

 let mut sm3_component = algorithm_component("SM3", "input:plain", "sm3");
 sm3_component.result_format = "uppercase".to_string();

 let results = execute_batch(BatchExecutionRequest {
 components: vec![sm3_component],
 input_values: inputs,
 });

 assert_eq!(results.len(), 1);
 assert_eq!(results[0].status, "success");
 assert_eq!(
 results[0].result,
 "71FB681B35DFF823FF2CAC1CE1A0ED2511CF1E59CE49B36B1FBFEC49D17C7691"
 );
 }

 #[test]
 fn actual_anhui_sso_project_chain_matches_golden_output() {
 let request = batch_from_project_config(
 include_str!("../../projects/安徽SSO.json"),
 HashMap::from([("input-1773973650210".to_string(), "abc".to_string())]),
 );

 let results = execute_batch(request);

 assert_eq!(results.len(), 3);
 assert!(results.iter().all(|result| result.status == "success"));
 assert_eq!(
 results[0].result,
 "a9993e364706816aba3e25717850c26c9cd0d89d"
 );
 assert_eq!(
 results[1].result,
 "90fe104f9480b49f7b239502d59daa03e828a27fcc9ff2d18ce2fe9088f2f7536b1a10d67863471070923974152d8e3624efff4a577f3f02591dc7e47cac40f94f074cf1e05a22bfbdff4e668c81e70ee83b11ded1379ff9c25b121b58aadc71bb64d3876227c4d168027befa6b7bc87674a48877cf1c850c5f537c3581b4306"
 );
 assert_eq!(
 results[2].result,
 "26D04FA023CF4C336563C74EECEA111EE304C40A9A10929321D2745C7FA8EAD1BCB19C781E5DA1026E0BFBCE476F22C1313F46387C0872CCAF1E0212B0B06E73732EF24CECD09FC776027361578C3E08C4EE906A2631411ADB76BD9042897C4FEA460FD6BE2F7EC80168571A6076659D94A87B69AC8E77728B418E45849B573C5B0D239FB5317919B1DF7CECB80D9578F2F355F0BBAB7281FEF67547D38F570CC9C3102CB7BE18F31601246B56164EBD1EE42FD242AA73313F28EDB16814F34E01C5A25037BE12A96C4D87C8924239992221B89A418998E02A3A2F5237F01ED8D00A11D01DFFE57C1F080E65CBCEAD4628F2BA31C5F5B3DC85175297C2DEB9E81AC4D28B5ED41EF810E31063D455EAD6"
 );
 }

 #[test]
 fn actual_linyi_sm4_project_chain_matches_golden_output() {
 let request = batch_from_project_config(
 include_str!("../../projects/临沂加密.json"),
 HashMap::from([(
 "input-1770197520573".to_string(),
 "QTMzNkEzQzdFNDU4RjZBRTIxNzQxQjEzNkYwOTQ1QTI=".to_string(),
 )]),
 );

 let results = execute_batch(request);

 assert_eq!(results.len(), 2);
 assert!(results.iter().all(|result| result.status == "success"));
 assert_eq!(results[0].result, "A336A3C7E458F6AE21741B136F0945A2");
 assert_eq!(results[1].result, "中文ABC");
 }
}
