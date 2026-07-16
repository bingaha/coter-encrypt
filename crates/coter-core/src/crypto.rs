use aes::cipher::{
 block_padding::{NoPadding, Pkcs7, ZeroPadding},
 BlockDecryptMut, BlockEncryptMut, KeyInit, KeyIvInit,
};
use aes::{Aes128, Aes192, Aes256};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use blowfish::Blowfish;
use encoding_rs::GBK;
use hmac::{Hmac, Mac};
use md5::Md5;
use rand::rngs::OsRng;
use rsa::pkcs1::{DecodeRsaPrivateKey, DecodeRsaPublicKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::{BigUint, Oaep, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};
use sm2::elliptic_curve::common::getrandom::SysRng;
use sm2::elliptic_curve::pkcs8::{
 DecodePrivateKey as Sm2DecodePrivateKey, DecodePublicKey as Sm2DecodePublicKey,
};
use sm2::pke::{DecryptingKey as Sm2DecryptingKey, EncryptingKey as Sm2EncryptingKey};
use sm2::{PublicKey as Sm2PublicKey, SecretKey as Sm2SecretKey};
use sm3::Sm3;
use sm4::Sm4;

const AES_BLOCK_SIZE: usize = 16;
const BLOWFISH_BLOCK_SIZE: usize = 8;
const SM4_BLOCK_SIZE: usize = 16;
const SHA1_DIGEST_INFO_PREFIX: [u8; 15] = [
 0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00, 0x04, 0x14,
];

pub fn process_base64(data: &str, operation: &str) -> Result<String, String> {
 match normalize_operation(operation).as_str() {
 "encrypt" | "encode" => Ok(STANDARD.encode(data.as_bytes())),
 "decrypt" | "decode" => {
 let decoded = STANDARD
 .decode(data)
 .map_err(|error| format!("Base64解码失败: {error}"))?;

 String::from_utf8(decoded)
 .map_err(|error| format!("Base64解码结果不是有效UTF-8: {error}"))
 }
 _ => Err(format!("不支持的Base64操作: {operation}")),
 }
}

pub fn process_hex(data: &str, operation: &str) -> Result<String, String> {
 match normalize_operation(operation).as_str() {
 "encrypt" | "encode" => Ok(bytes_to_hex(data.as_bytes())),
 "decrypt" | "decode" => {
 let bytes = hex_to_bytes(data)?;
 String::from_utf8(bytes).map_err(|error| format!("Hex解码结果不是有效UTF-8: {error}"))
 }
 _ => Err(format!("不支持的Hex操作: {operation}")),
 }
}

pub fn process_radix(
 data: &str,
 input_base: Option<u32>,
 output_base: Option<u32>,
 hex_case: &str,
) -> Result<String, String> {
 let input_base = normalize_radix(input_base.unwrap_or(10), "源进制")?;
 let output_base = normalize_radix(output_base.unwrap_or(16), "目标进制")?;
 let (negative, digits) = normalize_radix_input(data, input_base)?;

 let value = BigUint::parse_bytes(digits.as_bytes(), input_base)
 .ok_or_else(|| format!("无法按{input_base}进制解析输入"))?;

 let mut output = value.to_str_radix(output_base);
 if hex_case.eq_ignore_ascii_case("uppercase") {
 output = output.to_ascii_uppercase();
 }

 if negative && value != BigUint::from(0u8) {
 output.insert(0, '-');
 }

 Ok(output)
}

#[derive(Debug, Clone, Copy)]
enum UrlCharset {
 Utf8,
 Gbk,
 Iso88591,
}

pub fn process_url(data: &str, charset: &str, operation: &str) -> Result<String, String> {
 let charset = resolve_url_charset(charset)?;

 match normalize_operation(operation).as_str() {
 "encode" => Ok(url_encode(data, charset)),
 "decode" => url_decode(data, charset),
 _ => Err(format!("不支持的URL操作: {operation}")),
 }
}

pub fn process_unicode(data: &str, format: &str, operation: &str) -> Result<String, String> {
 match normalize_operation(operation).as_str() {
 "encode" => encode_unicode(data, format),
 "decode" => Ok(decode_unicode(data)),
 _ => Err(format!("不支持的Unicode操作: {operation}")),
 }
}

pub fn process_md5(
 data: &str,
 output_length: Option<u32>,
 result_format: &str,
) -> Result<String, String> {
 let digest = Md5::digest(data.as_bytes());
 let result = maybe_shorten_32_hex(bytes_to_hex(&digest), output_length);
 Ok(apply_result_format(result, result_format))
}

pub fn process_sha(
 data: &str,
 sha_type: Option<&str>,
 result_format: &str,
) -> Result<String, String> {
 let sha_type = sha_type
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .unwrap_or("SHA256");

 let result = match sha_type.to_ascii_uppercase().as_str() {
 "SHA1" => bytes_to_hex(&Sha1::digest(data.as_bytes())),
 "SHA256" => bytes_to_hex(&Sha256::digest(data.as_bytes())),
 "SHA384" => bytes_to_hex(&Sha384::digest(data.as_bytes())),
 "SHA512" => bytes_to_hex(&Sha512::digest(data.as_bytes())),
 _ => return Err(format!("不支持的SHA算法类型: {sha_type}")),
 };

 Ok(apply_result_format(result, result_format))
}

pub fn process_hmac_sha(
 data: &str,
 key: Option<&str>,
 hmac_sha_type: Option<&str>,
 result_format: &str,
) -> Result<String, String> {
 let key = key
 .filter(|value| !value.is_empty())
 .ok_or("HmacSHA密钥不能为空")?;
 let hmac_sha_type = hmac_sha_type
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .unwrap_or("HmacSHA256");

 let result = match hmac_sha_type.to_ascii_uppercase().as_str() {
 "HMACSHA1" => hmac_sha1_hex(data, key),
 "HMACSHA256" => hmac_sha256_hex(data, key),
 "HMACSHA384" => hmac_sha384_hex(data, key),
 "HMACSHA512" => hmac_sha512_hex(data, key),
 _ => return Err(format!("不支持的HmacSHA算法类型: {hmac_sha_type}")),
 };

 Ok(apply_result_format(result, result_format))
}

pub fn process_hmac_md5(
 data: &str,
 key: Option<&str>,
 output_length: Option<u32>,
 result_format: &str,
) -> Result<String, String> {
 let key = key
 .filter(|value| !value.is_empty())
 .ok_or("HmacMD5密钥不能为空")?;
 let result = hmac_md5_hex(data, key);
 let result = maybe_shorten_32_hex(result, output_length);
 Ok(apply_result_format(result, result_format))
}

pub fn process_sm3(data: &str, result_format: &str) -> Result<String, String> {
 let digest = <Sm3 as sm3::Digest>::digest(data.as_bytes());
 Ok(apply_result_format(bytes_to_hex(&digest), result_format))
}

#[allow(clippy::too_many_arguments)]
pub fn process_sm2(
 public_key: Option<&str>,
 private_key: Option<&str>,
 mode: Option<&str>,
 input_format: &str,
 output_format: &str,
 hex_case: &str,
 data: &str,
 operation: &str,
) -> Result<String, String> {
 let mode = parse_sm2_mode(mode)?;

 match normalize_operation(operation).as_str() {
 "encrypt" => {
 let public_key = public_key
 .filter(|value| !value.is_empty())
 .ok_or("SM2加密需要公钥")?;
 let public_key = parse_sm2_public_key(public_key, mode)?;
 let data_bytes =
 if input_format.trim().is_empty() || input_format.eq_ignore_ascii_case("text") {
 data.as_bytes().to_vec()
 } else {
 decode_input_bytes(data, input_format)?
 };

 let encrypted = sm2_encrypt(&public_key, mode, &data_bytes)?;
 encode_crypto_output(&encrypted, output_format, hex_case, "SM2加密结果")
 }
 "decrypt" => {
 if data.is_empty() {
 return Err("解密数据不能为空".to_string());
 }

 let private_key = private_key
 .filter(|value| !value.is_empty())
 .ok_or("SM2解密需要私钥")?;
 let private_key = parse_sm2_private_key(private_key, mode)?;
 let data_bytes = decode_input_bytes(data, input_format)?;
 if data_bytes.is_empty() {
 return Err("解码后的数据为空，请检查输入格式和数据".to_string());
 }

 let decrypted = private_key
 .decrypt(&data_bytes)
 .map_err(|error| format!("SM2解密失败: {error}"))?;
 encode_crypto_output(&decrypted, output_format, hex_case, "SM2解密结果")
 }
 _ => Err(format!("不支持的SM2操作: {operation}")),
 }
}

#[allow(clippy::too_many_arguments)]
pub fn process_aes(
 key: Option<&str>,
 iv: Option<&str>,
 mode: Option<&str>,
 padding: Option<&str>,
 input_format: &str,
 output_format: &str,
 key_format: &str,
 iv_format: &str,
 hex_case: &str,
 data: &str,
 operation: &str,
) -> Result<String, String> {
 let key = key
 .filter(|value| !value.is_empty())
 .ok_or("AES密钥不能为空")?;
 let key_bytes = decode_key_or_iv(key, key_format)?;
 validate_aes_key(key, &key_bytes)?;

 let mode = parse_aes_mode(mode)?;
 let padding = parse_aes_padding(padding)?;
 let iv_bytes = match mode {
 AesMode::Ecb => Vec::new(),
 AesMode::Cbc => {
 let iv = iv
 .filter(|value| !value.is_empty())
 .ok_or_else(|| format!("{}模式下IV向量不能为空", mode.as_str()))?;
 decode_key_or_iv(iv, iv_format)?
 }
 };

 match normalize_operation(operation).as_str() {
 "encrypt" | "encode" => {
 let data_bytes =
 if input_format.trim().is_empty() || input_format.eq_ignore_ascii_case("text") {
 data.as_bytes().to_vec()
 } else {
 decode_input_bytes(data, input_format)?
 };

 let encrypted = aes_encrypt(&key_bytes, &iv_bytes, mode, padding, &data_bytes)?;
 encode_crypto_output(&encrypted, output_format, hex_case, "AES加密结果")
 }
 "decrypt" | "decode" => {
 if data.is_empty() {
 return Err("解密数据不能为空".to_string());
 }

 let data_bytes = decode_input_bytes(data, input_format)?;
 if data_bytes.is_empty() {
 return Err("解码后的数据为空，请检查输入格式和数据".to_string());
 }

 let decrypted = aes_decrypt(&key_bytes, &iv_bytes, mode, padding, &data_bytes)?;
 encode_crypto_output(&decrypted, output_format, hex_case, "AES解密结果")
 }
 _ => Err(format!("不支持的AES操作: {operation}")),
 }
}

#[allow(clippy::too_many_arguments)]
pub fn process_blowfish(
 key: Option<&str>,
 iv: Option<&str>,
 mode: Option<&str>,
 padding: Option<&str>,
 input_format: &str,
 output_format: &str,
 key_format: &str,
 iv_format: &str,
 hex_case: &str,
 data: &str,
 operation: &str,
) -> Result<String, String> {
 let key = key
 .filter(|value| !value.is_empty())
 .ok_or("Blowfish密钥不能为空")?;
 let key_bytes = decode_key_or_iv(key, key_format)?;
 validate_blowfish_key(&key_bytes)?;

 let mode = parse_blowfish_mode(mode)?;
 let padding = parse_blowfish_padding(padding)?;
 let iv_bytes = match mode {
 BlowfishMode::Ecb => Vec::new(),
 BlowfishMode::Cbc => {
 let iv = iv
 .filter(|value| !value.is_empty())
 .ok_or("CBC模式下IV向量不能为空")?;
 let iv_bytes = decode_key_or_iv(iv, iv_format)?;
 validate_blowfish_iv(&iv_bytes)?;
 iv_bytes
 }
 };

 match normalize_operation(operation).as_str() {
 "encrypt" | "encode" => {
 let data_bytes =
 if input_format.trim().is_empty() || input_format.eq_ignore_ascii_case("text") {
 data.as_bytes().to_vec()
 } else {
 decode_input_bytes(data, input_format)?
 };

 let encrypted = blowfish_encrypt(&key_bytes, &iv_bytes, mode, padding, &data_bytes)?;
 encode_crypto_output(&encrypted, output_format, hex_case, "Blowfish加密结果")
 }
 "decrypt" | "decode" => {
 if data.is_empty() {
 return Err("解密数据不能为空".to_string());
 }

 let data_bytes = decode_input_bytes(data, input_format)?;
 if data_bytes.is_empty() {
 return Err("解码后的数据为空，请检查输入格式和数据".to_string());
 }

 let decrypted = blowfish_decrypt(&key_bytes, &iv_bytes, mode, padding, &data_bytes)?;
 encode_crypto_output(&decrypted, output_format, hex_case, "Blowfish解密结果")
 }
 _ => Err(format!("不支持的Blowfish操作: {operation}")),
 }
}

#[allow(clippy::too_many_arguments)]
pub fn process_sm4(
 key: Option<&str>,
 iv: Option<&str>,
 mode: Option<&str>,
 padding: Option<&str>,
 input_format: &str,
 output_format: &str,
 key_format: &str,
 iv_format: &str,
 hex_case: &str,
 data: &str,
 operation: &str,
) -> Result<String, String> {
 let key = key
 .filter(|value| !value.is_empty())
 .ok_or("SM4密钥不能为空")?;
 let key_bytes = normalize_sm4_key_or_iv(decode_key_or_iv(key, key_format)?);

 let mode = parse_sm4_mode(mode);
 let padding = parse_sm4_padding(padding);
 let iv_bytes = match mode {
 Sm4Mode::Ecb => Vec::new(),
 Sm4Mode::Cbc => {
 let iv = iv
 .filter(|value| !value.is_empty())
 .ok_or("CBC模式下IV向量不能为空")?;
 normalize_sm4_key_or_iv(decode_key_or_iv(iv, iv_format)?)
 }
 };

 match normalize_operation(operation).as_str() {
 "encrypt" | "encode" => {
 let encrypted = sm4_encrypt(&key_bytes, &iv_bytes, mode, padding, data.as_bytes())?;
 encode_crypto_output(&encrypted, output_format, hex_case, "SM4加密结果")
 }
 "decrypt" | "decode" => {
 if data.is_empty() {
 return Err("解密数据不能为空".to_string());
 }

 let data_bytes = decode_input_bytes(data, input_format)?;
 if data_bytes.is_empty() {
 return Err("解码后的数据为空，请检查输入格式和数据".to_string());
 }

 let decrypted = sm4_decrypt(&key_bytes, &iv_bytes, mode, padding, &data_bytes)?;
 encode_crypto_output(&decrypted, output_format, hex_case, "SM4解密结果")
 }
 _ => Err(format!("不支持的SM4操作: {operation}")),
 }
}

#[allow(clippy::too_many_arguments)]
pub fn process_rsa(
 public_key: Option<&str>,
 private_key: Option<&str>,
 padding: Option<&str>,
 input_format: &str,
 output_format: &str,
 hex_case: &str,
 data: &str,
 operation: &str,
) -> Result<String, String> {
 match normalize_operation(operation).as_str() {
 "encrypt" => {
 let public_key = public_key
 .filter(|value| !value.is_empty())
 .ok_or("RSA加密需要公钥")?;
 let public_key = parse_rsa_public_key(public_key)?;
 let encrypted = if is_rsa_oaep(padding) {
 rsa_oaep_encrypt(&public_key, data.as_bytes())?
 } else {
 rsa_pkcs1_encrypt(&public_key, data.as_bytes())?
 };
 encode_crypto_output(&encrypted, output_format, hex_case, "RSA加密结果")
 }
 "decrypt" => {
 if data.is_empty() {
 return Err("解密数据不能为空".to_string());
 }

 let private_key = private_key
 .filter(|value| !value.is_empty())
 .ok_or("RSA解密需要私钥")?;
 let private_key = parse_rsa_private_key(private_key)?;

 if is_rsa_pkcs1_signature(padding) {
 let signed = rsa_pkcs1_sha1_digest_signature(&private_key, data, input_format)
 .map_err(|error| format!("RSA签名失败: {error}"))?;
 return encode_crypto_output(&signed, output_format, hex_case, "RSA签名结果");
 }

 let data_bytes = decode_input_bytes(data, input_format)?;
 if data_bytes.is_empty() {
 return Err("解码后的数据为空，请检查输入格式和数据".to_string());
 }

 let decrypted = if is_rsa_oaep(padding) {
 rsa_oaep_decrypt(&private_key, &data_bytes)?
 } else {
 rsa_pkcs1_decrypt(&private_key, &data_bytes)?
 };
 encode_crypto_output(&decrypted, output_format, hex_case, "RSA解密结果")
 }
 _ => Err(format!("不支持的RSA操作: {operation}")),
 }
}

fn normalize_operation(operation: &str) -> String {
 operation.to_ascii_lowercase()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
 let mut output = String::with_capacity(bytes.len() * 2);
 for byte in bytes {
 output.push_str(&format!("{byte:02x}"));
 }
 output
}

fn normalize_output_format(output_format: &str) -> String {
 match output_format.trim().to_ascii_lowercase().as_str() {
 "" | "hex" => "hex".to_string(),
 "base64" => "base64".to_string(),
 "text" | "utf8" | "utf-8" => "utf-8".to_string(),
 "gbk" | "gb2312" => "gbk".to_string(),
 other => other.to_string(),
 }
}

fn encode_crypto_output(
 data: &[u8],
 output_format: &str,
 hex_case: &str,
 error_prefix: &str,
) -> Result<String, String> {
 match normalize_output_format(output_format).as_str() {
 "hex" => Ok(apply_hex_case(bytes_to_hex(data), hex_case)),
 "base64" => Ok(STANDARD.encode(data)),
 "utf-8" => String::from_utf8(data.to_vec())
 .map_err(|error| format!("{error_prefix}不是有效UTF-8: {error}")),
 "gbk" => {
 let (decoded, _, had_errors) = GBK.decode(data);
 if had_errors {
 return Err(format!("{error_prefix}不是有效GBK"));
 }
 Ok(decoded.into_owned())
 }
 other => Err(format!("不支持的输出格式: {other}")),
 }
}

fn apply_hex_case(result: String, hex_case: &str) -> String {
 if hex_case.eq_ignore_ascii_case("lowercase") {
 result.to_ascii_lowercase()
 } else {
 result.to_ascii_uppercase()
 }
}

fn apply_result_format(result: String, result_format: &str) -> String {
 if result_format.eq_ignore_ascii_case("uppercase") {
 result.to_ascii_uppercase()
 } else {
 result.to_ascii_lowercase()
 }
}

fn maybe_shorten_32_hex(result: String, output_length: Option<u32>) -> String {
 if output_length == Some(16) && result.len() >= 24 {
 result[8..24].to_string()
 } else {
 result
 }
}

fn finalize_hmac<M>(mut mac: M, data: &str) -> String
where
 M: Mac,
{
 mac.update(data.as_bytes());
 let result = mac.finalize().into_bytes();
 bytes_to_hex(&result)
}

fn hmac_md5_hex(data: &str, key: &str) -> String {
 let mac = <Hmac<Md5> as Mac>::new_from_slice(key.as_bytes())
 .expect("HMAC-MD5 accepts keys of any size");
 finalize_hmac(mac, data)
}

fn hmac_sha1_hex(data: &str, key: &str) -> String {
 let mac = <Hmac<Sha1> as Mac>::new_from_slice(key.as_bytes())
 .expect("HMAC-SHA1 accepts keys of any size");
 finalize_hmac(mac, data)
}

fn hmac_sha256_hex(data: &str, key: &str) -> String {
 let mac = <Hmac<Sha256> as Mac>::new_from_slice(key.as_bytes())
 .expect("HMAC-SHA256 accepts keys of any size");
 finalize_hmac(mac, data)
}

fn hmac_sha384_hex(data: &str, key: &str) -> String {
 let mac = <Hmac<Sha384> as Mac>::new_from_slice(key.as_bytes())
 .expect("HMAC-SHA384 accepts keys of any size");
 finalize_hmac(mac, data)
}

fn hmac_sha512_hex(data: &str, key: &str) -> String {
 let mac = <Hmac<Sha512> as Mac>::new_from_slice(key.as_bytes())
 .expect("HMAC-SHA512 accepts keys of any size");
 finalize_hmac(mac, data)
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
 let mut hex_string: String = hex.chars().filter(|c| !c.is_whitespace()).collect();

 if hex_string.starts_with("0x") || hex_string.starts_with("0X") {
 hex_string = hex_string[2..].to_string();
 }

 if hex_string.len() % 2 == 1 {
 hex_string.insert(0, '0');
 }

 let mut result = Vec::with_capacity(hex_string.len() / 2);
 let chars: Vec<char> = hex_string.chars().collect();

 for index in (0..chars.len()).step_by(2) {
 let hi = chars[index].to_digit(16).ok_or_else(|| {
 format!(
 "非法HEX字符: '{}{}' at {index}",
 chars[index],
 chars[index + 1]
 )
 })?;
 let lo = chars[index + 1].to_digit(16).ok_or_else(|| {
 format!(
 "非法HEX字符: '{}{}' at {index}",
 chars[index],
 chars[index + 1]
 )
 })?;

 result.push(((hi << 4) | lo) as u8);
 }

 Ok(result)
}

fn normalize_radix(base: u32, label: &str) -> Result<u32, String> {
 if (2..=36).contains(&base) {
 Ok(base)
 } else {
 Err(format!("{label}必须在2到36之间，当前为{base}"))
 }
}

fn normalize_radix_input(data: &str, input_base: u32) -> Result<(bool, String), String> {
 let mut value = data.trim();
 if value.is_empty() {
 return Err("待转换数字不能为空".to_string());
 }

 let negative = if let Some(rest) = value.strip_prefix('-') {
 value = rest;
 true
 } else if let Some(rest) = value.strip_prefix('+') {
 value = rest;
 false
 } else {
 false
 };

 if input_base == 16 {
 value = value
 .strip_prefix("0x")
 .or_else(|| value.strip_prefix("0X"))
 .unwrap_or(value);
 } else if input_base == 2 {
 value = value
 .strip_prefix("0b")
 .or_else(|| value.strip_prefix("0B"))
 .unwrap_or(value);
 } else if input_base == 8 {
 value = value
 .strip_prefix("0o")
 .or_else(|| value.strip_prefix("0O"))
 .unwrap_or(value);
 }

 let digits: String = value
 .chars()
 .filter(|ch| !ch.is_ascii_whitespace() && *ch != '_')
 .collect();

 if digits.is_empty() {
 return Err("待转换数字不能为空".to_string());
 }

 for ch in digits.chars() {
 let Some(digit) = ch.to_digit(36) else {
 return Err(format!("非法进制字符: {ch}"));
 };

 if digit >= input_base {
 return Err(format!("字符 {ch} 不属于{input_base}进制"));
 }
 }

 Ok((negative, digits))
}

fn decode_input_bytes(data: &str, input_format: &str) -> Result<Vec<u8>, String> {
 if input_format.trim().is_empty() {
 if is_auto_hex(data) {
 return hex_to_bytes(data);
 }

 return STANDARD
 .decode(data)
 .map_err(|error| format!("Base64解码失败: {error}"));
 }

 match input_format.to_ascii_lowercase().as_str() {
 "hex" => hex_to_bytes(data),
 "base64" => STANDARD
 .decode(data)
 .map_err(|error| format!("Base64解码失败: {error}")),
 "text" => Ok(data.as_bytes().to_vec()),
 _ => hex_to_bytes(data),
 }
}

fn is_auto_hex(data: &str) -> bool {
 !data.is_empty() && data.len() % 2 == 0 && data.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn decode_key_or_iv(value: &str, format: &str) -> Result<Vec<u8>, String> {
 if format.eq_ignore_ascii_case("hex") {
 hex_to_bytes(value)
 } else {
 Ok(value.as_bytes().to_vec())
 }
}

fn validate_aes_key(original_key: &str, key_bytes: &[u8]) -> Result<(), String> {
 if matches!(key_bytes.len(), 16 | 24 | 32) {
 return Ok(());
 }

 let key_bits = key_bytes.len() * 8;
 if original_key.contains("BEGIN") && original_key.contains("KEY") {
 return Err(format!(
 "AES密钥长度无效（{key_bits}位），当前传入的是RSA/SM2密钥，请提供128/192/256位的AES密钥"
 ));
 }

 Err(format!(
 "AES密钥长度必须为128/192/256位（16/24/32字节），当前长度: {key_bits}位"
 ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AesMode {
 Ecb,
 Cbc,
}

impl AesMode {
 fn as_str(self) -> &'static str {
 match self {
 Self::Ecb => "ECB",
 Self::Cbc => "CBC",
 }
 }
}

fn parse_aes_mode(mode: Option<&str>) -> Result<AesMode, String> {
 let mode = mode
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .unwrap_or("ECB");

 match mode.to_ascii_uppercase().as_str() {
 "ECB" => Ok(AesMode::Ecb),
 "CBC" => Ok(AesMode::Cbc),
 _ => Err(format!("AES模式暂未迁移到 Rust: {mode}")),
 }
}

#[derive(Debug, Clone, Copy)]
enum AesPadding {
 Pkcs7,
 NoPadding,
}

fn parse_aes_padding(padding: Option<&str>) -> Result<AesPadding, String> {
 let padding = padding
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .unwrap_or("PKCS5Padding");

 match padding.to_ascii_uppercase().as_str() {
 "PKCS5PADDING" | "PKCS5" | "PKCS7PADDING" | "PKCS7" => Ok(AesPadding::Pkcs7),
 "NOPADDING" | "NONE" => Ok(AesPadding::NoPadding),
 _ => Err(format!("AES填充暂未迁移到 Rust: {padding}")),
 }
}

fn validate_aes_block_aligned(operation: &str, data: &[u8]) -> Result<(), String> {
 if data.len() % AES_BLOCK_SIZE == 0 {
 Ok(())
 } else {
 Err(format!(
 "AES NoPadding {operation}数据长度必须是16字节的整数倍，当前长度: {}字节",
 data.len()
 ))
 }
}

fn validate_blowfish_key(key_bytes: &[u8]) -> Result<(), String> {
 if (4..=56).contains(&key_bytes.len()) {
 return Ok(());
 }

 Err(format!(
 "Blowfish密钥长度必须为4到56字节，当前长度: {}字节",
 key_bytes.len()
 ))
}

fn validate_blowfish_iv(iv_bytes: &[u8]) -> Result<(), String> {
 if iv_bytes.len() == BLOWFISH_BLOCK_SIZE {
 return Ok(());
 }

 Err(format!(
 "Blowfish CBC模式IV长度必须为8字节，当前长度: {}字节",
 iv_bytes.len()
 ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlowfishMode {
 Ecb,
 Cbc,
}

fn parse_blowfish_mode(mode: Option<&str>) -> Result<BlowfishMode, String> {
 let mode = mode
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .unwrap_or("ECB");

 match mode.to_ascii_uppercase().as_str() {
 "ECB" => Ok(BlowfishMode::Ecb),
 "CBC" => Ok(BlowfishMode::Cbc),
 _ => Err(format!("Blowfish模式暂未迁移到 Rust: {mode}")),
 }
}

#[derive(Debug, Clone, Copy)]
enum BlowfishPadding {
 Pkcs7,
 NoPadding,
}

fn parse_blowfish_padding(padding: Option<&str>) -> Result<BlowfishPadding, String> {
 let padding = padding
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .unwrap_or("PKCS5Padding");

 match padding.to_ascii_uppercase().as_str() {
 "PKCS5PADDING" | "PKCS5" | "PKCS7PADDING" | "PKCS7" => Ok(BlowfishPadding::Pkcs7),
 "NOPADDING" | "NONE" => Ok(BlowfishPadding::NoPadding),
 _ => Err(format!("Blowfish填充暂未迁移到 Rust: {padding}")),
 }
}

fn validate_blowfish_block_aligned(operation: &str, data: &[u8]) -> Result<(), String> {
 if data.len() % BLOWFISH_BLOCK_SIZE == 0 {
 Ok(())
 } else {
 Err(format!(
 "Blowfish NoPadding {operation}数据长度必须是8字节的整数倍，当前长度: {}字节",
 data.len()
 ))
 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sm4Mode {
 Ecb,
 Cbc,
}

fn parse_sm4_mode(mode: Option<&str>) -> Sm4Mode {
 match mode
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .unwrap_or("ECB")
 .to_ascii_uppercase()
 .as_str()
 {
 "CBC" => Sm4Mode::Cbc,
 _ => Sm4Mode::Ecb,
 }
}

#[derive(Debug, Clone, Copy)]
enum Sm4Padding {
 Pkcs7,
 Zero,
 NoPadding,
}

fn parse_sm4_padding(padding: Option<&str>) -> Sm4Padding {
 match padding
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .unwrap_or("PKCS5Padding")
 .to_ascii_uppercase()
 .as_str()
 {
 "ZEROPADDING" | "ZERO" => Sm4Padding::Zero,
 "NOPADDING" | "NONE" => Sm4Padding::NoPadding,
 _ => Sm4Padding::Pkcs7,
 }
}

fn parse_sm2_mode(mode: Option<&str>) -> Result<sm2::pke::Mode, String> {
 let mode = mode
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .unwrap_or("C1C3C2");

 match mode.to_ascii_uppercase().as_str() {
 "C1C3C2" => Ok(sm2::pke::Mode::C1C3C2),
 "C1C2C3" => Ok(sm2::pke::Mode::C1C2C3),
 _ => Err(format!("不支持的SM2密文顺序: {mode}")),
 }
}

fn normalize_sm4_key_or_iv(mut value: Vec<u8>) -> Vec<u8> {
 value.resize(SM4_BLOCK_SIZE, 0);
 value.truncate(SM4_BLOCK_SIZE);
 value
}

fn validate_sm4_block_aligned(operation: &str, data: &[u8]) -> Result<(), String> {
 if data.len() % SM4_BLOCK_SIZE == 0 {
 Ok(())
 } else {
 Err(format!(
 "SM4 NoPadding {operation}数据长度必须是16字节的整数倍，当前长度: {}字节",
 data.len()
 ))
 }
}

fn is_rsa_pkcs1_signature(padding: Option<&str>) -> bool {
 padding
 .map(str::trim)
 .is_some_and(|value| value == "PKCS1签名")
}

fn is_rsa_oaep(padding: Option<&str>) -> bool {
 padding
 .map(str::trim)
 .filter(|value| !value.is_empty())
 .is_some_and(|value| value.eq_ignore_ascii_case("OAEP"))
}

fn rsa_key_der_bytes(key: &str) -> Result<Vec<u8>, String> {
 let trimmed = key.trim();
 if trimmed.contains("-----BEGIN") {
 return pem_body_to_der(trimmed);
 }

 let compact: String = trimmed.chars().filter(|ch| !ch.is_whitespace()).collect();
 if compact.is_empty() {
 return Err("RSA密钥不能为空".to_string());
 }

 STANDARD
 .decode(compact)
 .map_err(|error| format!("RSA密钥Base64解码失败: {error}"))
}

fn pem_body_to_der(pem: &str) -> Result<Vec<u8>, String> {
 let body = pem
 .lines()
 .map(str::trim)
 .filter(|line| !line.starts_with("-----BEGIN") && !line.starts_with("-----END"))
 .collect::<String>();

 if body.is_empty() {
 return Err("RSA PEM密钥内容为空".to_string());
 }

 STANDARD
 .decode(body)
 .map_err(|error| format!("RSA PEM密钥Base64解码失败: {error}"))
}

fn parse_rsa_public_key(key: &str) -> Result<RsaPublicKey, String> {
 let der = rsa_key_der_bytes(key)?;
 RsaPublicKey::from_public_key_der(&der)
 .or_else(|_| RsaPublicKey::from_pkcs1_der(&der))
 .map_err(|error| format!("RSA公钥解析失败: {error}"))
}

fn parse_rsa_private_key(key: &str) -> Result<RsaPrivateKey, String> {
 let der = rsa_key_der_bytes(key)?;
 RsaPrivateKey::from_pkcs8_der(&der)
 .or_else(|_| RsaPrivateKey::from_pkcs1_der(&der))
 .map_err(|error| format!("RSA私钥解析失败: {error}"))
}

fn rsa_pkcs1_encrypt(public_key: &RsaPublicKey, data: &[u8]) -> Result<Vec<u8>, String> {
 let block_size = public_key.size().saturating_sub(11);
 if block_size == 0 {
 return Err("RSA公钥长度无效".to_string());
 }

 let mut rng = OsRng;
 let mut output = Vec::new();
 for chunk in data.chunks(block_size) {
 let encrypted = public_key
 .encrypt(&mut rng, Pkcs1v15Encrypt, chunk)
 .map_err(|error| format!("RSA加密失败: {error}"))?;
 output.extend(encrypted);
 }

 Ok(output)
}

fn rsa_pkcs1_decrypt(private_key: &RsaPrivateKey, data: &[u8]) -> Result<Vec<u8>, String> {
 let block_size = private_key.size();
 if block_size == 0 {
 return Err("RSA私钥长度无效".to_string());
 }
 if data.len() % block_size != 0 {
 return Err(format!(
 "RSA密文长度必须是密钥长度的整数倍，当前密文长度: {}字节，密钥长度: {block_size}字节",
 data.len()
 ));
 }

 let mut output = Vec::new();
 for chunk in data.chunks(block_size) {
 let decrypted = private_key
 .decrypt(Pkcs1v15Encrypt, chunk)
 .map_err(|error| format!("RSA解密失败: {error}"))?;
 output.extend(decrypted);
 }

 Ok(output)
}

fn rsa_oaep_encrypt(public_key: &RsaPublicKey, data: &[u8]) -> Result<Vec<u8>, String> {
 let block_size = public_key.size().saturating_sub(2 * 20 + 2);
 if block_size == 0 {
 return Err("RSA OAEP 公钥长度无效".to_string());
 }

 let mut rng = OsRng;
 let mut output = Vec::new();
 for chunk in data.chunks(block_size) {
 let encrypted = public_key
 .encrypt(&mut rng, Oaep::new::<Sha1>(), chunk)
 .map_err(|error| format!("RSA OAEP加密失败: {error}"))?;
 output.extend(encrypted);
 }

 Ok(output)
}

fn rsa_oaep_decrypt(private_key: &RsaPrivateKey, data: &[u8]) -> Result<Vec<u8>, String> {
 let block_size = private_key.size();
 if block_size == 0 {
 return Err("RSA OAEP私钥长度无效".to_string());
 }
 if data.len() % block_size != 0 {
 return Err(format!(
 "RSA OAEP密文长度必须是密钥长度的整数倍，当前密文长度: {}字节，密钥长度: {block_size}字节",
 data.len()
 ));
 }

 let mut output = Vec::new();
 for chunk in data.chunks(block_size) {
 let decrypted = private_key
 .decrypt(Oaep::new::<Sha1>(), chunk)
 .map_err(|error| format!("RSA OAEP解密失败: {error}"))?;
 output.extend(decrypted);
 }

 Ok(output)
}

fn rsa_pkcs1_sha1_digest_signature(
 private_key: &RsaPrivateKey,
 data: &str,
 input_format: &str,
) -> Result<Vec<u8>, String> {
 let digest = decode_input_bytes(data, input_format)?;
 let key_bytes = private_key.size();
 let digest_info_len = SHA1_DIGEST_INFO_PREFIX.len() + digest.len();
 if digest_info_len > key_bytes.saturating_sub(11) {
 return Err(format!(
 "签名数据过长，最大允许: {}字节",
 key_bytes.saturating_sub(11)
 ));
 }

 let pad_len = key_bytes - 3 - digest_info_len;
 let mut padded = Vec::with_capacity(key_bytes);
 padded.push(0);
 padded.push(1);
 padded.extend(std::iter::repeat(0xff).take(pad_len));
 padded.push(0);
 padded.extend(SHA1_DIGEST_INFO_PREFIX);
 padded.extend(digest);

 let message = BigUint::from_bytes_be(&padded);
 let signature = message.modpow(private_key.d(), private_key.n());
 Ok(biguint_to_unsigned_fixed_bytes(&signature, key_bytes))
}

fn biguint_to_unsigned_fixed_bytes(value: &BigUint, key_bytes: usize) -> Vec<u8> {
 let bytes = value.to_bytes_be();
 if bytes.len() <= key_bytes {
 return bytes;
 }

 bytes[bytes.len() - key_bytes..].to_vec()
}

fn sm2_key_bytes(key: &str) -> Result<Vec<u8>, String> {
 let trimmed = key.trim();
 if trimmed.is_empty() {
 return Err("SM2密钥不能为空".to_string());
 }
 if trimmed.contains("-----BEGIN") {
 return asymmetric_pem_body_to_der(trimmed, "SM2");
 }

 let compact: String = trimmed.chars().filter(|ch| !ch.is_whitespace()).collect();
 if compact.is_empty() {
 return Err("SM2密钥不能为空".to_string());
 }

 if is_auto_hex(&compact) {
 return hex_to_bytes(&compact);
 }

 STANDARD
 .decode(compact)
 .map_err(|error| format!("SM2密钥Base64解码失败: {error}"))
}

fn asymmetric_pem_body_to_der(pem: &str, algorithm: &str) -> Result<Vec<u8>, String> {
 let body = pem
 .lines()
 .map(str::trim)
 .filter(|line| !line.starts_with("-----BEGIN") && !line.starts_with("-----END"))
 .collect::<String>();

 if body.is_empty() {
 return Err(format!("{algorithm} PEM密钥内容为空"));
 }

 STANDARD
 .decode(body)
 .map_err(|error| format!("{algorithm} PEM密钥Base64解码失败: {error}"))
}

fn parse_sm2_public_key(key: &str, mode: sm2::pke::Mode) -> Result<Sm2EncryptingKey, String> {
 let bytes = sm2_key_bytes(key)?;
 let public_key = if bytes.len() == 64 {
 let mut sec1 = Vec::with_capacity(65);
 sec1.push(0x04);
 sec1.extend(bytes);
 Sm2PublicKey::from_sec1_bytes(&sec1).map_err(|error| format!("SM2公钥解析失败: {error}"))?
 } else if matches!(bytes.len(), 33 | 65) && matches!(bytes[0], 0x02 | 0x03 | 0x04) {
 Sm2PublicKey::from_sec1_bytes(&bytes)
 .map_err(|error| format!("SM2公钥解析失败: {error}"))?
 } else {
 Sm2PublicKey::from_public_key_der(&bytes)
 .map_err(|error| format!("SM2公钥解析失败: {error}"))?
 };

 Ok(Sm2EncryptingKey::new_with_mode(public_key, mode))
}

fn parse_sm2_private_key(key: &str, mode: sm2::pke::Mode) -> Result<Sm2DecryptingKey, String> {
 let bytes = sm2_key_bytes(key)?;
 let secret_key = if bytes.len() == 32 {
 Sm2SecretKey::from_slice(&bytes).map_err(|error| format!("SM2私钥解析失败: {error}"))?
 } else {
 Sm2SecretKey::from_pkcs8_der(&bytes).map_err(|error| format!("SM2私钥解析失败: {error}"))?
 };

 Ok(Sm2DecryptingKey::new_with_mode(
 secret_key.to_nonzero_scalar(),
 mode,
 ))
}

fn sm2_encrypt(
 public_key: &Sm2EncryptingKey,
 mode: sm2::pke::Mode,
 data: &[u8],
) -> Result<Vec<u8>, String> {
 let cipher = public_key
 .encrypt_cipher::<_, Sm3>(&mut SysRng, data)
 .map_err(|error| format!("SM2加密失败: {error}"))?;
 cipher
 .to_vec(mode, false)
 .map_err(|error| format!("SM2密文编码失败: {error}"))
}

fn aes_encrypt(
 key: &[u8],
 iv: &[u8],
 mode: AesMode,
 padding: AesPadding,
 data: &[u8],
) -> Result<Vec<u8>, String> {
 if matches!(padding, AesPadding::NoPadding) {
 validate_aes_block_aligned("加密", data)?;
 }

 match (mode, key.len(), padding) {
 (AesMode::Ecb, 16, AesPadding::Pkcs7) => Ok(ecb::Encryptor::<Aes128>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data)),
 (AesMode::Ecb, 24, AesPadding::Pkcs7) => Ok(ecb::Encryptor::<Aes192>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data)),
 (AesMode::Ecb, 32, AesPadding::Pkcs7) => Ok(ecb::Encryptor::<Aes256>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data)),
 (AesMode::Ecb, 16, AesPadding::NoPadding) => {
 Ok(ecb::Encryptor::<Aes128>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data))
 }
 (AesMode::Ecb, 24, AesPadding::NoPadding) => {
 Ok(ecb::Encryptor::<Aes192>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data))
 }
 (AesMode::Ecb, 32, AesPadding::NoPadding) => {
 Ok(ecb::Encryptor::<Aes256>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data))
 }
 (AesMode::Cbc, 16, AesPadding::Pkcs7) => {
 Ok(cbc::Encryptor::<Aes128>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data))
 }
 (AesMode::Cbc, 24, AesPadding::Pkcs7) => {
 Ok(cbc::Encryptor::<Aes192>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data))
 }
 (AesMode::Cbc, 32, AesPadding::Pkcs7) => {
 Ok(cbc::Encryptor::<Aes256>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data))
 }
 (AesMode::Cbc, 16, AesPadding::NoPadding) => {
 Ok(cbc::Encryptor::<Aes128>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data))
 }
 (AesMode::Cbc, 24, AesPadding::NoPadding) => {
 Ok(cbc::Encryptor::<Aes192>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data))
 }
 (AesMode::Cbc, 32, AesPadding::NoPadding) => {
 Ok(cbc::Encryptor::<Aes256>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data))
 }
 _ => Err("AES密钥长度必须为128/192/256位（16/24/32字节）".to_string()),
 }
}

fn aes_decrypt(
 key: &[u8],
 iv: &[u8],
 mode: AesMode,
 padding: AesPadding,
 data: &[u8],
) -> Result<Vec<u8>, String> {
 validate_aes_block_aligned("解密", data)?;

 match (mode, key.len(), padding) {
 (AesMode::Ecb, 16, AesPadding::Pkcs7) => ecb::Decryptor::<Aes128>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("AES解密失败: {error}")),
 (AesMode::Ecb, 24, AesPadding::Pkcs7) => ecb::Decryptor::<Aes192>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("AES解密失败: {error}")),
 (AesMode::Ecb, 32, AesPadding::Pkcs7) => ecb::Decryptor::<Aes256>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("AES解密失败: {error}")),
 (AesMode::Ecb, 16, AesPadding::NoPadding) => ecb::Decryptor::<Aes128>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("AES解密失败: {error}")),
 (AesMode::Ecb, 24, AesPadding::NoPadding) => ecb::Decryptor::<Aes192>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("AES解密失败: {error}")),
 (AesMode::Ecb, 32, AesPadding::NoPadding) => ecb::Decryptor::<Aes256>::new_from_slice(key)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("AES解密失败: {error}")),
 (AesMode::Cbc, 16, AesPadding::Pkcs7) => cbc::Decryptor::<Aes128>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("AES解密失败: {error}")),
 (AesMode::Cbc, 24, AesPadding::Pkcs7) => cbc::Decryptor::<Aes192>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("AES解密失败: {error}")),
 (AesMode::Cbc, 32, AesPadding::Pkcs7) => cbc::Decryptor::<Aes256>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("AES解密失败: {error}")),
 (AesMode::Cbc, 16, AesPadding::NoPadding) => {
 cbc::Decryptor::<Aes128>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("AES解密失败: {error}"))
 }
 (AesMode::Cbc, 24, AesPadding::NoPadding) => {
 cbc::Decryptor::<Aes192>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("AES解密失败: {error}"))
 }
 (AesMode::Cbc, 32, AesPadding::NoPadding) => {
 cbc::Decryptor::<Aes256>::new_from_slices(key, iv)
 .map_err(|error| format!("AES初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("AES解密失败: {error}"))
 }
 _ => Err("AES密钥长度必须为128/192/256位（16/24/32字节）".to_string()),
 }
}

fn blowfish_encrypt(
 key: &[u8],
 iv: &[u8],
 mode: BlowfishMode,
 padding: BlowfishPadding,
 data: &[u8],
) -> Result<Vec<u8>, String> {
 if matches!(padding, BlowfishPadding::NoPadding) {
 validate_blowfish_block_aligned("加密", data)?;
 }

 match (mode, padding) {
 (BlowfishMode::Ecb, BlowfishPadding::Pkcs7) => {
 Ok(ecb::Encryptor::<Blowfish>::new_from_slice(key)
 .map_err(|error| format!("Blowfish初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data))
 }
 (BlowfishMode::Ecb, BlowfishPadding::NoPadding) => {
 Ok(ecb::Encryptor::<Blowfish>::new_from_slice(key)
 .map_err(|error| format!("Blowfish初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data))
 }
 (BlowfishMode::Cbc, BlowfishPadding::Pkcs7) => {
 Ok(cbc::Encryptor::<Blowfish>::new_from_slices(key, iv)
 .map_err(|error| format!("Blowfish初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data))
 }
 (BlowfishMode::Cbc, BlowfishPadding::NoPadding) => {
 Ok(cbc::Encryptor::<Blowfish>::new_from_slices(key, iv)
 .map_err(|error| format!("Blowfish初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data))
 }
 }
}

fn blowfish_decrypt(
 key: &[u8],
 iv: &[u8],
 mode: BlowfishMode,
 padding: BlowfishPadding,
 data: &[u8],
) -> Result<Vec<u8>, String> {
 validate_blowfish_block_aligned("解密", data)?;

 match (mode, padding) {
 (BlowfishMode::Ecb, BlowfishPadding::Pkcs7) => {
 ecb::Decryptor::<Blowfish>::new_from_slice(key)
 .map_err(|error| format!("Blowfish初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("Blowfish解密失败: {error}"))
 }
 (BlowfishMode::Ecb, BlowfishPadding::NoPadding) => {
 ecb::Decryptor::<Blowfish>::new_from_slice(key)
 .map_err(|error| format!("Blowfish初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("Blowfish解密失败: {error}"))
 }
 (BlowfishMode::Cbc, BlowfishPadding::Pkcs7) => {
 cbc::Decryptor::<Blowfish>::new_from_slices(key, iv)
 .map_err(|error| format!("Blowfish初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("Blowfish解密失败: {error}"))
 }
 (BlowfishMode::Cbc, BlowfishPadding::NoPadding) => {
 cbc::Decryptor::<Blowfish>::new_from_slices(key, iv)
 .map_err(|error| format!("Blowfish初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("Blowfish解密失败: {error}"))
 }
 }
}

fn sm4_encrypt(
 key: &[u8],
 iv: &[u8],
 mode: Sm4Mode,
 padding: Sm4Padding,
 data: &[u8],
) -> Result<Vec<u8>, String> {
 if matches!(padding, Sm4Padding::NoPadding) {
 validate_sm4_block_aligned("加密", data)?;
 }

 match (mode, padding) {
 (Sm4Mode::Ecb, Sm4Padding::Pkcs7) => Ok(ecb::Encryptor::<Sm4>::new_from_slice(key)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data)),
 (Sm4Mode::Ecb, Sm4Padding::Zero) => Ok(ecb::Encryptor::<Sm4>::new_from_slice(key)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<ZeroPadding>(data)),
 (Sm4Mode::Ecb, Sm4Padding::NoPadding) => Ok(ecb::Encryptor::<Sm4>::new_from_slice(key)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data)),
 (Sm4Mode::Cbc, Sm4Padding::Pkcs7) => Ok(cbc::Encryptor::<Sm4>::new_from_slices(key, iv)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<Pkcs7>(data)),
 (Sm4Mode::Cbc, Sm4Padding::Zero) => Ok(cbc::Encryptor::<Sm4>::new_from_slices(key, iv)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<ZeroPadding>(data)),
 (Sm4Mode::Cbc, Sm4Padding::NoPadding) => {
 Ok(cbc::Encryptor::<Sm4>::new_from_slices(key, iv)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .encrypt_padded_vec_mut::<NoPadding>(data))
 }
 }
}

fn sm4_decrypt(
 key: &[u8],
 iv: &[u8],
 mode: Sm4Mode,
 padding: Sm4Padding,
 data: &[u8],
) -> Result<Vec<u8>, String> {
 validate_sm4_block_aligned("解密", data)?;

 match (mode, padding) {
 (Sm4Mode::Ecb, Sm4Padding::Pkcs7) => ecb::Decryptor::<Sm4>::new_from_slice(key)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("SM4解密失败: {error}")),
 (Sm4Mode::Ecb, Sm4Padding::Zero) => ecb::Decryptor::<Sm4>::new_from_slice(key)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<ZeroPadding>(data)
 .map_err(|error| format!("SM4解密失败: {error}")),
 (Sm4Mode::Ecb, Sm4Padding::NoPadding) => ecb::Decryptor::<Sm4>::new_from_slice(key)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("SM4解密失败: {error}")),
 (Sm4Mode::Cbc, Sm4Padding::Pkcs7) => cbc::Decryptor::<Sm4>::new_from_slices(key, iv)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<Pkcs7>(data)
 .map_err(|error| format!("SM4解密失败: {error}")),
 (Sm4Mode::Cbc, Sm4Padding::Zero) => cbc::Decryptor::<Sm4>::new_from_slices(key, iv)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<ZeroPadding>(data)
 .map_err(|error| format!("SM4解密失败: {error}")),
 (Sm4Mode::Cbc, Sm4Padding::NoPadding) => cbc::Decryptor::<Sm4>::new_from_slices(key, iv)
 .map_err(|error| format!("SM4初始化失败: {error}"))?
 .decrypt_padded_vec_mut::<NoPadding>(data)
 .map_err(|error| format!("SM4解密失败: {error}")),
 }
}

fn resolve_url_charset(charset: &str) -> Result<UrlCharset, String> {
 let charset = charset.trim();
 if charset.is_empty() {
 return Ok(UrlCharset::Utf8);
 }

 let normalized = charset
 .chars()
 .filter(|c| *c != '-' && *c != '_' && !c.is_whitespace())
 .collect::<String>()
 .to_ascii_uppercase();

 match normalized.as_str() {
 "UTF8" => Ok(UrlCharset::Utf8),
 "GBK" => Ok(UrlCharset::Gbk),
 "ISO88591" => Ok(UrlCharset::Iso88591),
 _ => Err(format!("不支持的字符集: {charset}")),
 }
}

fn url_encode(data: &str, charset: UrlCharset) -> String {
 let mut output = String::new();

 for ch in data.chars() {
 if is_url_safe_ascii(ch) {
 output.push(ch);
 } else if ch == ' ' {
 output.push('+');
 } else {
 for byte in encode_char(ch, charset) {
 output.push_str(&format!("%{byte:02X}"));
 }
 }
 }

 output
}

fn url_decode(data: &str, charset: UrlCharset) -> Result<String, String> {
 let mut output = String::new();
 let mut index = 0;
 let bytes = data.as_bytes();

 while index < bytes.len() {
 match bytes[index] {
 b'+' => {
 output.push(' ');
 index += 1;
 }
 b'%' => {
 let mut encoded_bytes = Vec::new();
 while index < bytes.len() && bytes[index] == b'%' {
 if index + 2 >= bytes.len() {
 return Err("URL解码失败: 不完整的百分号编码".to_string());
 }

 let hi = hex_value(bytes[index + 1]).ok_or_else(|| {
 format!(
 "URL解码失败: 非法百分号编码 %{}{}",
 bytes[index + 1] as char,
 bytes[index + 2] as char
 )
 })?;
 let lo = hex_value(bytes[index + 2]).ok_or_else(|| {
 format!(
 "URL解码失败: 非法百分号编码 %{}{}",
 bytes[index + 1] as char,
 bytes[index + 2] as char
 )
 })?;
 encoded_bytes.push((hi << 4) | lo);
 index += 3;
 }
 output.push_str(&decode_bytes(&encoded_bytes, charset));
 }
 _ => {
 let ch = data[index..]
 .chars()
 .next()
 .expect("index is always at a char boundary");
 output.push(ch);
 index += ch.len_utf8();
 }
 }
 }

 Ok(output)
}

fn is_url_safe_ascii(ch: char) -> bool {
 ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '*')
}

fn encode_char(ch: char, charset: UrlCharset) -> Vec<u8> {
 match charset {
 UrlCharset::Utf8 => {
 let mut buffer = [0; 4];
 ch.encode_utf8(&mut buffer).as_bytes().to_vec()
 }
 UrlCharset::Gbk => {
 let value = ch.to_string();
 let (encoded, _, had_errors) = GBK.encode(&value);
 if had_errors {
 vec![b'?']
 } else {
 encoded.into_owned()
 }
 }
 UrlCharset::Iso88591 => {
 let code = ch as u32;
 if code <= 0xFF {
 vec![code as u8]
 } else {
 vec![b'?']
 }
 }
 }
}

fn decode_bytes(bytes: &[u8], charset: UrlCharset) -> String {
 match charset {
 UrlCharset::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
 UrlCharset::Gbk => {
 let (decoded, _, _) = GBK.decode(bytes);
 decoded.into_owned()
 }
 UrlCharset::Iso88591 => bytes.iter().map(|byte| char::from(*byte)).collect(),
 }
}

fn hex_value(byte: u8) -> Option<u8> {
 match byte {
 b'0'..=b'9' => Some(byte - b'0'),
 b'a'..=b'f' => Some(byte - b'a' + 10),
 b'A'..=b'F' => Some(byte - b'A' + 10),
 _ => None,
 }
}

fn encode_unicode(data: &str, format: &str) -> Result<String, String> {
 let format = if format.trim().is_empty() {
 "standard"
 } else {
 format.trim()
 };
 let format = format.to_ascii_lowercase();
 let mut output = String::new();
 for code_unit in data.encode_utf16() {
 if code_unit < 128 {
 output.push(char::from(code_unit as u8));
 } else {
 match format.as_str() {
 "html" => output.push_str(&format!("&#{code_unit};")),
 "css" => output.push_str(&format!("\\{code_unit:04X}")),
 _ => output.push_str(&format!("\\u{code_unit:04X}")),
 }
 }
 }

 Ok(output)
}

fn decode_unicode(data: &str) -> String {
 let mut code_units: Vec<u16> = data.encode_utf16().collect();
 code_units = decode_standard_unicode_units(&code_units);
 code_units = decode_html_unicode_units(&code_units);
 code_units = decode_css_unicode_units(&code_units);

 std::char::decode_utf16(code_units)
 .map(|item| item.unwrap_or(char::REPLACEMENT_CHARACTER))
 .collect()
}

fn decode_standard_unicode_units(input: &[u16]) -> Vec<u16> {
 let mut output = Vec::with_capacity(input.len());
 let mut index = 0;

 while index < input.len() {
 if index + 5 < input.len()
 && input[index] == b'\\' as u16
 && input[index + 1] == b'u' as u16
 && input[index + 2..index + 6]
 .iter()
 .all(|value| u8::try_from(*value).ok().and_then(hex_value).is_some())
 {
 output.push(parse_hex_units(&input[index + 2..index + 6]));
 index += 6;
 } else {
 output.push(input[index]);
 index += 1;
 }
 }

 output
}

fn decode_html_unicode_units(input: &[u16]) -> Vec<u16> {
 let mut output = Vec::with_capacity(input.len());
 let mut index = 0;

 while index < input.len() {
 if index + 3 < input.len() && input[index] == b'&' as u16 && input[index + 1] == b'#' as u16
 {
 let mut end = index + 2;
 while end < input.len() && is_ascii_digit_unit(input[end]) {
 end += 1;
 }

 if end > index + 2 && end < input.len() && input[end] == b';' as u16 {
 let value = input[index + 2..end]
 .iter()
 .fold(0u32, |acc, digit| acc * 10 + (*digit as u32 - b'0' as u32));
 output.push(value as u16);
 index = end + 1;
 continue;
 }
 }

 output.push(input[index]);
 index += 1;
 }

 output
}

fn decode_css_unicode_units(input: &[u16]) -> Vec<u16> {
 let mut output = Vec::with_capacity(input.len());
 let mut index = 0;

 while index < input.len() {
 if index + 4 < input.len()
 && input[index] == b'\\' as u16
 && input[index + 1..index + 5]
 .iter()
 .all(|value| u8::try_from(*value).ok().and_then(hex_value).is_some())
 {
 output.push(parse_hex_units(&input[index + 1..index + 5]));
 index += 5;
 } else {
 output.push(input[index]);
 index += 1;
 }
 }

 output
}

fn parse_hex_units(units: &[u16]) -> u16 {
 units.iter().fold(0u16, |acc, value| {
 let digit = u8::try_from(*value)
 .ok()
 .and_then(hex_value)
 .expect("caller validated hex units");
 (acc << 4) | digit as u16
 })
}

fn is_ascii_digit_unit(value: u16) -> bool {
 matches!(value, value if value >= b'0' as u16 && value <= b'9' as u16)
}

#[cfg(test)]
mod tests {
 use super::{
 process_aes, process_base64, process_blowfish, process_hex, process_hmac_md5,
 process_hmac_sha, process_md5, process_radix, process_rsa, process_sha, process_sm2,
 process_sm3, process_sm4, process_unicode, process_url,
 };

 const PROJECT_RSA_PRIVATE_KEY: &str = "MIICdgIBADANBgkqhkiG9w0BAQEFAASCAmAwggJcAgEAAoGBAJKf8oCY21bhRnx8nldD2evjW69K4OrbGuG3FUH/b+qrhw95qUBHjaNIH5QB9kO4HHsPAxhV5snGaXHSenuXCxZFBWe5WG9cdW7dI/7YhhPGOY0l0ywjiMe5wHhbycpIRmfbyCfWtpU36KtMv9t75pQTKjbcJjQtPOtQ+v5OejBVAgMBAAECgYAlqia+OAXoHHhh1BVMr2ZUfRP5RI/gZKZUIxa33GkgbC2GoScEFx1gO0+5UoOzQ6E1T1bpMm/Vlz1Q+tNx2gwDqOy1839z65ZAggm9buEN9j9E3NOztia1wRtAgH5C3/Aflo192HZTjYzzVI5ZDLN7A2Y76MmLMzqVxNY7vf+hRQJBAN9nEH9x4ZIncrBhlClObE5aBn00l31LbhoetNFFIIwZ9kAAa/jJAgD2Ddky87YQLYyYiYqQTKUp018dxZN+KbsCQQCoBPC3cEmHLk8D1dAd9VOCUQKDcka0Gfs9eXE/cp9kQ+4AfbfaZcQuYPwqF9ZQqfylLQGARw8xfhX98CEsEaUvAkEAr6puSZh1xCQqxdDk3RoihfW6Noe9OzOt7vIIQqn1rtTXUnpCbI06eyD/wLOU+at89ZoYRRG0gwcBg0B41MKW8wJANurudzbngZzcTMedL72ZHxY1eRtoCsQXP5+rKW7gtFgTuetdpa/vsK0YnvWNom39W0vbmr8fMzEgJRFQ9mOKFwJADY9VWYxOdf/5AjimWA1gt+mtJqZB+2jdJYyoR16cWm5sMVrEz18vVPJeh0qOmXOO7ClFs9UUVtTabiszuO+wRw==";
 const PROJECT_RSA_PUBLIC_KEY_FROM_PRIVATE: &str = "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQCSn/KAmNtW4UZ8fJ5XQ9nr41uvSuDq2xrhtxVB/2/qq4cPealAR42jSB+UAfZDuBx7DwMYVebJxmlx0np7lwsWRQVnuVhvXHVu3SP+2IYTxjmNJdMsI4jHucB4W8nKSEZn28gn1raVN+irTL/be+aUEyo23CY0LTzrUPr+TnowVQIDAQAB";
 const SM2_PRIVATE_D: &str = "9998894D66977D5F2C68B7E0564DFBFB36EE5AFD5520F7FDA1AF6E7D6ACAA874";
 const SM2_PUBLIC_Q: &str = "049031694836FCCD075D20CC284278901F37AF7D1EF8DEA025393C4643CE577C9DB64DF3E331ECC5B105E40E6C65949B6B5F6E8F1D99D28B6E01539DAE723588F0";
 const SM2_C1C3C2_GOLDEN_CIPHER_HEX: &str = "042B2354EAF69D675491C0C0294E06C69D6A68669F1A8B2FC661EACB5B1B16D8E52AFC1F89D3C255094AD8E2ED4C62E6DD47DF4550128B606B93465D2E72EB245F2BD5DA2AB31132EED6AE0C1597747B6098831AB0F269DA7AA4884E2AD8039FD6CB7C40FC5BD131E55C";

 #[test]
 fn base64_encode_decode_matches_expected_vectors() {
 let encoded = process_base64("中文ABC", "encode").unwrap();
 assert_eq!(encoded, "5Lit5paHQUJD");
 assert_eq!(process_base64(&encoded, "decode").unwrap(), "中文ABC");
 assert_eq!(process_base64("abc", "encrypt").unwrap(), "YWJj");
 }

 #[test]
 fn hex_encode_decode_matches_expected_vectors() {
 assert_eq!(
 process_hex("中文ABC", "encode").unwrap(),
 "e4b8ade69687414243"
 );
 assert_eq!(
 process_hex("e4 b8 ad e6 96 87 41 42 43", "decode").unwrap(),
 "中文ABC"
 );
 assert_eq!(process_hex("0x41", "decode").unwrap(), "A");
 assert_eq!(process_hex("1", "decode").unwrap(), "\u{1}");
 }

 #[test]
 fn hex_rejects_invalid_characters() {
 assert!(process_hex("0xGG", "decode")
 .unwrap_err()
 .contains("非法HEX字符"));
 }

 #[test]
 fn radix_converts_between_common_bases() {
 assert_eq!(
 process_radix("255", Some(10), Some(16), "uppercase").unwrap(),
 "FF"
 );
 assert_eq!(
 process_radix("0xff", Some(16), Some(2), "lowercase").unwrap(),
 "11111111"
 );
 assert_eq!(
 process_radix("-1010_1010", Some(2), Some(10), "uppercase").unwrap(),
 "-170"
 );
 assert_eq!(
 process_radix("z", Some(36), Some(10), "uppercase").unwrap(),
 "35"
 );
 }

 #[test]
 fn radix_reports_invalid_base_and_digits() {
 assert!(process_radix("10", Some(1), Some(10), "uppercase")
 .unwrap_err()
 .contains("源进制必须在2到36之间"));
 assert!(process_radix("102", Some(2), Some(10), "uppercase")
 .unwrap_err()
 .contains("不属于2进制"));
 assert!(process_radix("", Some(10), Some(16), "uppercase")
 .unwrap_err()
 .contains("待转换数字不能为空"));
 }

 #[test]
 fn url_encode_decode_utf8_matches_url_encoder_vectors() {
 let encoded = process_url("a b中文", "UTF-8", "encode").unwrap();
 assert_eq!(encoded, "a+b%E4%B8%AD%E6%96%87");
 assert_eq!(process_url(&encoded, "UTF-8", "decode").unwrap(), "a b中文");
 }

 #[test]
 fn url_supports_frontend_charset_options() {
 assert_eq!(
 process_url("中文", "GBK", "encode").unwrap(),
 "%D6%D0%CE%C4"
 );
 assert_eq!(
 process_url("%D6%D0%CE%C4", "GBK", "decode").unwrap(),
 "中文"
 );
 assert_eq!(
 process_url("é中文", "ISO-8859-1", "encode").unwrap(),
 "%E9%3F%3F"
 );
 assert_eq!(
 process_url("%E9%3F%3F", "ISO-8859-1", "decode").unwrap(),
 "é??"
 );
 }

 #[test]
 fn url_rejects_invalid_percent_encoding() {
 assert!(process_url("%G0", "UTF-8", "decode")
 .unwrap_err()
 .contains("非法百分号编码"));
 assert!(process_url("%A", "UTF-8", "decode")
 .unwrap_err()
 .contains("不完整的百分号编码"));
 }

 #[test]
 fn unicode_encode_matches_utf16_code_unit_vectors() {
 assert_eq!(
 process_unicode("ABC中文", "standard", "encode").unwrap(),
 r"ABC\u4E2D\u6587"
 );
 assert_eq!(
 process_unicode("ABC中文", "html", "encode").unwrap(),
 "ABC&#20013;&#25991;"
 );
 assert_eq!(
 process_unicode("ABC中文", "css", "encode").unwrap(),
 r"ABC\4E2D\6587"
 );
 assert_eq!(
 process_unicode("😀", "standard", "encode").unwrap(),
 r"\uD83D\uDE00"
 );
 assert_eq!(
 process_unicode("中文", "unknown", "encode").unwrap(),
 r"\u4E2D\u6587"
 );
 }

 #[test]
 fn unicode_decode_accepts_all_supported_formats() {
 assert_eq!(
 process_unicode(r"\u4E2D&#25991;\0041", "standard", "decode").unwrap(),
 "中文A"
 );
 assert_eq!(
 process_unicode(r"\uD83D\uDE00", "standard", "decode").unwrap(),
 "😀"
 );
 }

 #[test]
 fn md5_matches_expected_vectors() {
 assert_eq!(
 process_md5("abc", Some(32), "lowercase").unwrap(),
 "900150983cd24fb0d6963f7d28e17f72"
 );
 assert_eq!(
 process_md5("abc", Some(16), "lowercase").unwrap(),
 "3cd24fb0d6963f7d"
 );
 assert_eq!(
 process_md5("abc", Some(16), "uppercase").unwrap(),
 "3CD24FB0D6963F7D"
 );
 }

 #[test]
 fn sha_matches_expected_vectors() {
 assert_eq!(
 process_sha("abc", Some("SHA1"), "lowercase").unwrap(),
 "a9993e364706816aba3e25717850c26c9cd0d89d"
 );
 assert_eq!(
 process_sha("abc", Some("SHA256"), "lowercase").unwrap(),
 "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
 );
 assert_eq!(
 process_sha("abc", Some("SHA384"), "lowercase").unwrap(),
 "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
 );
 assert_eq!(
 process_sha("abc", Some("SHA512"), "uppercase").unwrap(),
 "DDAF35A193617ABACC417349AE20413112E6FA4E89A97EA20A9EEEE64B55D39A2192992A274FC1A836BA3C23A3FEEBBD454D4423643CE80E2A9AC94FA54CA49F"
 );
 assert!(process_sha("abc", Some("SHA999"), "lowercase")
 .unwrap_err()
 .contains("不支持的SHA算法类型"));
 }

 #[test]
 fn sm3_matches_expected_vectors() {
 assert_eq!(
 process_sm3("abc", "lowercase").unwrap(),
 "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0"
 );
 assert_eq!(
 process_sm3("abc", "uppercase").unwrap(),
 "66C7F0F462EEEDD9D1F2D46BDC10E4E24167C4875CF2F7A2297DA02B8F4BA8E0"
 );
 assert_eq!(
 process_sm3("中文ABC", "lowercase").unwrap(),
 "71fb681b35dff823ff2cac1ce1a0ed2511cf1e59ce49b36b1fbfec49d17c7691"
 );
 }

 #[test]
 fn sm2_decrypts_reference_c1c3c2_golden_ciphertext() {
 let decrypted = process_sm2(
 None,
 Some(SM2_PRIVATE_D),
 Some("C1C3C2"),
 "hex",
 "utf-8",
 "uppercase",
 SM2_C1C3C2_GOLDEN_CIPHER_HEX,
 "decrypt",
 )
 .unwrap();

 assert_eq!(decrypted, "中文ABC");
 }

 #[test]
 fn sm2_encrypt_decrypt_round_trips_with_raw_keys_and_outputs_uncompressed_c1() {
 let encrypted = process_sm2(
 Some(SM2_PUBLIC_Q),
 None,
 Some("C1C3C2"),
 "text",
 "hex",
 "uppercase",
 "中文ABC",
 "encrypt",
 )
 .unwrap();

 assert_eq!(encrypted.len(), SM2_C1C3C2_GOLDEN_CIPHER_HEX.len());
 assert!(encrypted.starts_with("04"));

 let decrypted = process_sm2(
 None,
 Some(SM2_PRIVATE_D),
 Some("C1C3C2"),
 "hex",
 "utf-8",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap();

 assert_eq!(decrypted, "中文ABC");
 }

 #[test]
 fn sm2_supports_c1c2c3_ciphertext_order() {
 let c1 = &SM2_C1C3C2_GOLDEN_CIPHER_HEX[..130];
 let c3 = &SM2_C1C3C2_GOLDEN_CIPHER_HEX[130..194];
 let c2 = &SM2_C1C3C2_GOLDEN_CIPHER_HEX[194..];
 let c1c2c3 = format!("{c1}{c2}{c3}");

 let decrypted = process_sm2(
 None,
 Some(SM2_PRIVATE_D),
 Some("C1C2C3"),
 "hex",
 "utf-8",
 "uppercase",
 &c1c2c3,
 "decrypt",
 )
 .unwrap();

 assert_eq!(decrypted, "中文ABC");
 }

 #[test]
 fn sm2_accepts_public_key_without_uncompressed_prefix() {
 let public_key_without_prefix = &SM2_PUBLIC_Q[2..];
 let encrypted = process_sm2(
 Some(public_key_without_prefix),
 None,
 Some("C1C3C2"),
 "text",
 "base64",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap();

 let decrypted = process_sm2(
 None,
 Some(SM2_PRIVATE_D),
 Some("C1C3C2"),
 "base64",
 "utf-8",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap();

 assert_eq!(decrypted, "abc");
 }

 #[test]
 fn sm2_reports_required_inputs_and_invalid_mode() {
 assert!(process_sm2(
 None,
 None,
 Some("C1C3C2"),
 "text",
 "hex",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("SM2加密需要公钥"));

 assert!(process_sm2(
 None,
 None,
 Some("C1C3C2"),
 "hex",
 "utf-8",
 "uppercase",
 SM2_C1C3C2_GOLDEN_CIPHER_HEX,
 "decrypt",
 )
 .unwrap_err()
 .contains("SM2解密需要私钥"));

 assert!(process_sm2(
 Some(SM2_PUBLIC_Q),
 None,
 Some("unknown"),
 "text",
 "hex",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("不支持的SM2密文顺序"));
 }

 #[test]
 fn hmac_sha_matches_expected_vectors() {
 assert_eq!(
 process_hmac_sha("abc", Some("key"), Some("HmacSHA1"), "lowercase").unwrap(),
 "4fd0b215276ef12f2b3e4c8ecac2811498b656fc"
 );
 assert_eq!(
 process_hmac_sha("abc", Some("key"), Some("HmacSHA256"), "lowercase").unwrap(),
 "9c196e32dc0175f86f4b1cb89289d6619de6bee699e4c378e68309ed97a1a6ab"
 );
 assert_eq!(
 process_hmac_sha("abc", Some("key"), Some("HmacSHA384"), "lowercase").unwrap(),
 "30ddb9c8f347cffbfb44e519d814f074cf4047a55d6f563324f1c6a33920e5edfb2a34bac60bdc96cd33a95623d7d638"
 );
 assert_eq!(
 process_hmac_sha("abc", Some("key"), Some("HmacSHA512"), "uppercase").unwrap(),
 "3926A207C8C42B0C41792CBD3E1A1AAAF5F7A25704F62DFC939C4987DD7CE060009C5BB1C2447355B3216F10B537E9AFA7B64A4E5391B0D631172D07939E087A"
 );
 assert!(
 process_hmac_sha("abc", None, Some("HmacSHA256"), "lowercase")
 .unwrap_err()
 .contains("HmacSHA密钥不能为空")
 );
 assert!(
 process_hmac_sha("abc", Some("key"), Some("HmacSHA999"), "lowercase")
 .unwrap_err()
 .contains("不支持的HmacSHA算法类型")
 );
 }

 #[test]
 fn hmac_md5_matches_expected_vectors() {
 assert_eq!(
 process_hmac_md5("abc", Some("key"), Some(32), "lowercase").unwrap(),
 "d2fe98063f876b03193afb49b4979591"
 );
 assert_eq!(
 process_hmac_md5("abc", Some("key"), Some(16), "uppercase").unwrap(),
 "3F876B03193AFB49"
 );
 assert!(process_hmac_md5("abc", None, Some(32), "lowercase")
 .unwrap_err()
 .contains("HmacMD5密钥不能为空"));
 }

 #[test]
 fn aes_ecb_pkcs5_text_to_base64_matches_expected_vectors() {
 let encrypted = process_aes(
 Some("d3k7s8c4n2m1s5b9"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "text",
 "base64",
 "text",
 "text",
 "uppercase",
 "中文ABC",
 "encrypt",
 )
 .unwrap();

 assert_eq!(encrypted, "maSZYCez1P5rOLu0Bpr2BA==");
 assert_eq!(
 process_aes(
 Some("d3k7s8c4n2m1s5b9"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "base64",
 "utf-8",
 "text",
 "text",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap(),
 "中文ABC"
 );
 }

 #[test]
 fn aes_ecb_pkcs5_hex_key_outputs_hex_case() {
 let encrypted = process_aes(
 Some("00112233445566778899aabbccddeeff"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "hex",
 "text",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap();

 assert_eq!(encrypted, "9E2E902DDFD264E59397700E9316D13C");
 assert_eq!(
 process_aes(
 Some("00112233445566778899aabbccddeeff"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "hex",
 "text",
 "lowercase",
 "abc",
 "encrypt",
 )
 .unwrap(),
 "9e2e902ddfd264e59397700e9316d13c"
 );
 }

 #[test]
 fn aes_cbc_pkcs5_uses_text_iv_and_round_trips() {
 let encrypted = process_aes(
 Some("0123456789abcdef"),
 Some("abcdef9876543210"),
 Some("CBC"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "hello rust",
 "encrypt",
 )
 .unwrap();

 assert_eq!(encrypted, "0C6B692518F591669AC846FD3E1D5275");
 assert_eq!(
 process_aes(
 Some("0123456789abcdef"),
 Some("abcdef9876543210"),
 Some("CBC"),
 Some("PKCS5Padding"),
 "hex",
 "utf-8",
 "text",
 "text",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap(),
 "hello rust"
 );
 }

 #[test]
 fn aes_no_padding_requires_block_aligned_input() {
 let encrypted = process_aes(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("NoPadding"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "1234567890abcdef",
 "encrypt",
 )
 .unwrap();

 assert_eq!(encrypted, "461A5FFD9DF79171358E9E0177D84EAA");
 assert_eq!(
 process_aes(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("NoPadding"),
 "hex",
 "utf-8",
 "text",
 "text",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap(),
 "1234567890abcdef"
 );
 assert!(process_aes(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("NoPadding"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "not block aligned",
 "encrypt",
 )
 .unwrap_err()
 .contains("16字节的整数倍"));
 }

 #[test]
 fn aes_reports_key_iv_mode_and_data_errors() {
 assert!(process_aes(
 None,
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("AES密钥不能为空"));

 assert!(process_aes(
 Some("short"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("AES密钥长度必须为128/192/256位"));

 assert!(process_aes(
 Some("0123456789abcdef"),
 None,
 Some("CBC"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("CBC模式下IV向量不能为空"));

 assert!(process_aes(
 Some("0123456789abcdef"),
 None,
 Some("CTR"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("AES模式暂未迁移到 Rust"));

 assert!(process_aes(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "hex",
 "utf-8",
 "text",
 "text",
 "uppercase",
 "",
 "decrypt",
 )
 .unwrap_err()
 .contains("解密数据不能为空"));
 }

 #[test]
 fn blowfish_cbc_no_padding_hex_to_base64_matches_golden() {
 let encrypted = process_blowfish(
 Some("FZvIAvgfQRfKKzFjHi_hndypTFCSWVRl"),
 Some("0000000000000000"),
 Some("CBC"),
 Some("None"),
 "hex",
 "base64",
 "text",
 "hex",
 "uppercase",
 "00000000000319cc",
 "encrypt",
 )
 .unwrap();

 assert_eq!(encrypted, "DQ8foMjgxwQ=");
 }

 #[test]
 fn blowfish_ecb_pkcs5_round_trips_text() {
 let encrypted = process_blowfish(
 Some("test-key"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "text",
 "hex",
 "uppercase",
 "hello",
 "encrypt",
 )
 .unwrap();

 assert_eq!(
 process_blowfish(
 Some("test-key"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "hex",
 "utf-8",
 "text",
 "hex",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap(),
 "hello"
 );
 }

 #[test]
 fn blowfish_decrypt_binary_plaintext_supports_hex_and_base64_output() {
 // 明文是非 UTF-8 二进制；输出应按字节编码，而不是先强制 UTF-8
 let plain_hex = "00000000000319CC";
 let encrypted = process_blowfish(
 Some("FZvIAvgfQRfKKzFjHi_hndypTFCSWVRl"),
 Some("0000000000000000"),
 Some("CBC"),
 Some("None"),
 "hex",
 "base64",
 "text",
 "hex",
 "uppercase",
 plain_hex,
 "encrypt",
 )
 .unwrap();

 assert_eq!(encrypted, "DQ8foMjgxwQ=");
 assert_eq!(
 process_blowfish(
 Some("FZvIAvgfQRfKKzFjHi_hndypTFCSWVRl"),
 Some("0000000000000000"),
 Some("CBC"),
 Some("None"),
 "base64",
 "hex",
 "text",
 "hex",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap(),
 plain_hex
 );
 assert_eq!(
 process_blowfish(
 Some("FZvIAvgfQRfKKzFjHi_hndypTFCSWVRl"),
 Some("0000000000000000"),
 Some("CBC"),
 Some("None"),
 "base64",
 "base64",
 "text",
 "hex",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap(),
 "AAAAAAADGcw="
 );
 assert!(
 process_blowfish(
 Some("FZvIAvgfQRfKKzFjHi_hndypTFCSWVRl"),
 Some("0000000000000000"),
 Some("CBC"),
 Some("None"),
 "base64",
 "utf-8",
 "text",
 "hex",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap_err()
 .contains("不是有效UTF-8")
 );
 }

 #[test]
 fn blowfish_decrypt_supports_gbk_output() {
 // "中文" GBK bytes: D6 D0 CE C4
 let encrypted = process_blowfish(
 Some("test-key"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "hex",
 "hex",
 "text",
 "hex",
 "uppercase",
 "D6D0CEC4",
 "encrypt",
 )
 .unwrap();

 assert_eq!(
 process_blowfish(
 Some("test-key"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "hex",
 "gbk",
 "text",
 "hex",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap(),
 "中文"
 );
 }

 #[test]
 fn blowfish_reports_required_inputs_and_alignment() {
 assert!(process_blowfish(
 None,
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "text",
 "hex",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("Blowfish密钥不能为空"));

 assert!(process_blowfish(
 Some("abc"),
 None,
 Some("ECB"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "text",
 "hex",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("Blowfish密钥长度必须为4到56字节"));

 assert!(process_blowfish(
 Some("test-key"),
 None,
 Some("CBC"),
 Some("PKCS5Padding"),
 "text",
 "hex",
 "text",
 "hex",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("CBC模式下IV向量不能为空"));

 assert!(process_blowfish(
 Some("test-key"),
 Some("0000000000000001"),
 Some("CBC"),
 Some("NoPadding"),
 "text",
 "hex",
 "text",
 "hex",
 "uppercase",
 "not aligned",
 "encrypt",
 )
 .unwrap_err()
 .contains("8字节的整数倍"));
 }

 #[test]
 fn sm4_ecb_pkcs7_text_to_base64_matches_expected_vectors() {
 let encrypted = process_sm4(
 Some("Li8CW5HpIDSwMKog"),
 None,
 Some("ECB"),
 Some("pkcs7"),
 "text",
 "base64",
 "text",
 "text",
 "uppercase",
 "中文ABC",
 "encrypt",
 )
 .unwrap();

 assert_eq!(encrypted, "ozajx+RY9q4hdBsTbwlFog==");
 assert_eq!(
 process_sm4(
 Some("Li8CW5HpIDSwMKog"),
 None,
 Some("ECB"),
 Some("pkcs7"),
 "base64",
 "utf-8",
 "text",
 "text",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap(),
 "中文ABC"
 );
 }

 #[test]
 fn sm4_key_and_iv_are_zero_padded_or_truncated_to_block_size() {
 let encrypted_short_key = process_sm4(
 Some("short"),
 None,
 Some("ECB"),
 Some("pkcs7"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap();
 assert_eq!(encrypted_short_key, "AEA2B10BF7BC309E67AFEADD1BE32C71");

 let encrypted_cbc = process_sm4(
 Some("1234567890abcdefZZZ"),
 Some("iv-short"),
 Some("CBC"),
 Some("pkcs7"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "hello rust",
 "encrypt",
 )
 .unwrap();
 assert_eq!(encrypted_cbc, "490EBF82870026FF5EDB2A6A24322F48");
 assert_eq!(
 process_sm4(
 Some("1234567890abcdefZZZ"),
 Some("iv-short"),
 Some("CBC"),
 Some("pkcs7"),
 "hex",
 "utf-8",
 "text",
 "text",
 "uppercase",
 &encrypted_cbc,
 "decrypt",
 )
 .unwrap(),
 "hello rust"
 );
 }

 #[test]
 fn sm4_supports_zero_and_no_padding() {
 let zero_padded = process_sm4(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("zero"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap();
 assert_eq!(zero_padded, "5FFD60168716DF08E9765D5AAE52EC1C");
 assert_eq!(
 process_sm4(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("zero"),
 "hex",
 "utf-8",
 "text",
 "text",
 "uppercase",
 &zero_padded,
 "decrypt",
 )
 .unwrap(),
 "abc"
 );

 let no_padding = process_sm4(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("none"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "1234567890abcdef",
 "encrypt",
 )
 .unwrap();
 assert_eq!(no_padding, "EAF9B4843492C0B14C0E0EBF6E322C8B");
 assert_eq!(
 process_sm4(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("none"),
 "hex",
 "utf-8",
 "text",
 "text",
 "uppercase",
 &no_padding,
 "decrypt",
 )
 .unwrap(),
 "1234567890abcdef"
 );
 }

 #[test]
 fn sm4_reports_required_inputs_and_no_padding_alignment() {
 assert!(process_sm4(
 None,
 None,
 Some("ECB"),
 Some("pkcs7"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("SM4密钥不能为空"));

 assert!(process_sm4(
 Some("0123456789abcdef"),
 None,
 Some("CBC"),
 Some("pkcs7"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("CBC模式下IV向量不能为空"));

 assert!(process_sm4(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("none"),
 "text",
 "hex",
 "text",
 "text",
 "uppercase",
 "not block aligned",
 "encrypt",
 )
 .unwrap_err()
 .contains("16字节的整数倍"));

 assert!(process_sm4(
 Some("0123456789abcdef"),
 None,
 Some("ECB"),
 Some("pkcs7"),
 "hex",
 "utf-8",
 "text",
 "text",
 "uppercase",
 "",
 "decrypt",
 )
 .unwrap_err()
 .contains("解密数据不能为空"));
 }

 #[test]
 fn rsa_pkcs1_signature_matches_expected_vectors_for_project_key() {
 let signed = process_rsa(
 None,
 Some(PROJECT_RSA_PRIVATE_KEY),
 Some("PKCS1签名"),
 "hex",
 "hex",
 "lowercase",
 "a9993e364706816aba3e25717850c26c9cd0d89d",
 "decrypt",
 )
 .unwrap();

 assert_eq!(
 signed,
 "90fe104f9480b49f7b239502d59daa03e828a27fcc9ff2d18ce2fe9088f2f7536b1a10d67863471070923974152d8e3624efff4a577f3f02591dc7e47cac40f94f074cf1e05a22bfbdff4e668c81e70ee83b11ded1379ff9c25b121b58aadc71bb64d3876227c4d168027befa6b7bc87674a48877cf1c850c5f537c3581b4306"
 );
 }

 #[test]
 fn rsa_pkcs1_encrypt_decrypt_round_trips_and_supports_segments() {
 let plain = "中文ABC".repeat(30);
 let encrypted = process_rsa(
 Some(PROJECT_RSA_PUBLIC_KEY_FROM_PRIVATE),
 None,
 Some("PKCS1"),
 "text",
 "base64",
 "uppercase",
 &plain,
 "encrypt",
 )
 .unwrap();

 let decrypted = process_rsa(
 None,
 Some(PROJECT_RSA_PRIVATE_KEY),
 Some("PKCS1"),
 "base64",
 "utf-8",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap();

 assert_eq!(decrypted, plain);
 }

 #[test]
 fn rsa_parses_pkcs1_der_private_key_and_subject_public_key() {
 let encrypted = process_rsa(
 Some(PROJECT_RSA_PUBLIC_KEY_FROM_PRIVATE),
 None,
 Some("PKCS1"),
 "text",
 "hex",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap();

 let decrypted = process_rsa(
 None,
 Some(PROJECT_RSA_PRIVATE_KEY),
 Some("PKCS1"),
 "hex",
 "utf-8",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap();

 assert_eq!(decrypted, "abc");
 }

 #[test]
 fn rsa_oaep_encrypt_decrypt_round_trips_and_supports_segments() {
 let plain = "OAEP中文ABC".repeat(12);
 let encrypted = process_rsa(
 Some(PROJECT_RSA_PUBLIC_KEY_FROM_PRIVATE),
 None,
 Some("OAEP"),
 "text",
 "base64",
 "uppercase",
 &plain,
 "encrypt",
 )
 .unwrap();

 let decrypted = process_rsa(
 None,
 Some(PROJECT_RSA_PRIVATE_KEY),
 Some("OAEP"),
 "base64",
 "utf-8",
 "uppercase",
 &encrypted,
 "decrypt",
 )
 .unwrap();

 assert_eq!(decrypted, plain);
 }

 #[test]
 fn rsa_reports_required_inputs_and_bad_ciphertext_lengths() {
 assert!(process_rsa(
 None,
 None,
 Some("PKCS1"),
 "text",
 "hex",
 "uppercase",
 "abc",
 "encrypt",
 )
 .unwrap_err()
 .contains("RSA加密需要公钥"));

 assert!(process_rsa(
 None,
 None,
 Some("PKCS1"),
 "hex",
 "utf-8",
 "uppercase",
 "abc",
 "decrypt",
 )
 .unwrap_err()
 .contains("RSA解密需要私钥"));

 assert!(process_rsa(
 None,
 Some(PROJECT_RSA_PRIVATE_KEY),
 Some("PKCS1"),
 "hex",
 "utf-8",
 "uppercase",
 "abc",
 "decrypt",
 )
 .unwrap_err()
 .contains("RSA密文长度必须是密钥长度的整数倍"));

 assert!(process_rsa(
 None,
 Some(PROJECT_RSA_PRIVATE_KEY),
 Some("OAEP"),
 "hex",
 "utf-8",
 "uppercase",
 "abc",
 "decrypt",
 )
 .unwrap_err()
 .contains("RSA OAEP密文长度必须是密钥长度的整数倍"));
 }
}
