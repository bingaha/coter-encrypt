//! 在线凭证查询专用的 `assistant_cert.cert_info_enc` 解密模块。
//!
//! 口径对齐网厅后端
//! `com.shebao.platform.crawler.utils.CertInfoEncryptUtil`：
//! `AES-128-ECB-PKCS5Padding`，密钥 `7fK2pQ9mLx4Nw8Rc`（16 字节），
//! 明文 UTF-8，密文 Base64。
//!
//! 注意：本模块是独立实现，故意不调用“加解密工具”
//!（`coter-core::crypto` / `src-tauri::crypto`）的任何函数，
//! 只依赖第三方 `aes` / `ecb` / `base64` 基础库，
//! 防止后续加解密工具改版影响在线凭证查询。

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyInit};
use aes::Aes128;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ecb::Decryptor;

/// 网厅写入 `assistant_cert.cert_info_enc` 时使用的 AES-128 密钥（写死，不可更改）。
const CERT_INFO_KEY: &[u8] = b"7fK2pQ9mLx4Nw8Rc";

/// 解密 `cert_info_enc` 密文，返回凭证 JSON 明文。
pub fn decrypt_cert_info_enc(cipher_text: &str) -> Result<String, String> {
  let cipher_text = cipher_text.trim();
  if cipher_text.is_empty() {
    return Err("cert_info_enc 为空，无法解密".to_string());
  }
  let raw = STANDARD
    .decode(cipher_text)
    .map_err(|error| format!("cert_info_enc 不是合法 Base64: {error}"))?;
  if raw.is_empty() || raw.len() % 16 != 0 {
    return Err(format!(
      "cert_info_enc 长度不合法（{} 字节，需为 16 的整数倍）",
      raw.len()
    ));
  }
  let plain = Decryptor::<Aes128>::new_from_slice(CERT_INFO_KEY)
    .map_err(|error| format!("cert_info 解密初始化失败: {error}"))?
    .decrypt_padded_vec_mut::<Pkcs7>(&raw)
    .map_err(|error| format!("cert_info_enc 解密失败: {error}"))?;
  String::from_utf8(plain).map_err(|error| format!("cert_info 明文不是合法 UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
  use super::decrypt_cert_info_enc;

  /// 与 Java 侧 `CertInfoEncryptUtil.encrypt("{\"hello\":\"world\"}")` 等价的加密过程，
  /// 用于验证本模块解密口径（这里用加密只为构造测试向量，不作为业务能力）。
  fn encrypt_for_test(plain: &str) -> String {
    use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyInit};
    use aes::Aes128;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use ecb::Encryptor;

    let cipher = Encryptor::<Aes128>::new_from_slice(super::CERT_INFO_KEY).unwrap();
    let encrypted = cipher.encrypt_padded_vec_mut::<Pkcs7>(plain.as_bytes());
    STANDARD.encode(encrypted)
  }

  #[test]
  fn decrypts_java_compatible_ciphertext() {
    let plain = r#"{"cookies":"[{\"name\":\"SESSION\",\"value\":\"abc\"}]"}"#;
    let cipher_text = encrypt_for_test(plain);
    assert_eq!(decrypt_cert_info_enc(&cipher_text).unwrap(), plain);
  }

  #[test]
  fn decrypts_real_nanjing_gjj_sample_prefix() {
    // 南京久伴房屋租赁有限公司（公积金）2026-10-16 最新有效凭证的真实密文，
    // 明文应为 www.njgjj.com 网厅 cookies JSON。
    let cipher_text = "Xca22apxIr4I+wjtLmpOO0JLAm0Nki+/fs8ar66GYsz0t4OCvh9q8swhzRDvG7rqxrKQjklevX0nOFkHqjucrC6CkDv9266z97BI/BH4NAXkmByfEmcXerBQc+RXnoxqY6Lj8UHIqMv8n3uq6v1VymhomE/lnHscwHexncOfC7sVkRBKTo4sijbK1UN30i5rQ5UZliaFMDiberd8Yfpft4CIOGddgvwm18zWHaamCx+RoGcTpd4rFAPqT6GXuFzDMP4t9YTgyQ/49sQquzpaN1acPW5Nvefm+LOLzHwHzpHmuDlAFcjK3bcHKrbJIHOmVfVPF0wQcrmP5ZIvhSmQzKcrTqu3GY6FgdGCfhnljmoP6g/MqoLpd9oG7m4wJ60QnTXzL5GyIE6xOLlDizoKcwGxUp6FHP8d05pxfVnZGoDD92j/NB9XbxVkElsu71tX491qS4Ze1quQnPnTpXAbwQ==";
    let plain = decrypt_cert_info_enc(cipher_text).unwrap();
    assert!(plain.contains("www.njgjj.com"), "unexpected plain: {plain:.120}");
    assert!(plain.contains("cookies"), "unexpected plain: {plain:.120}");
    // 明文必须是合法 JSON，否则下游桥接无法提取 Cookie。
    let _: serde_json::Value = serde_json::from_str(&plain).unwrap();
  }

  #[test]
  fn rejects_invalid_input() {
    assert!(decrypt_cert_info_enc("").is_err());
    assert!(decrypt_cert_info_enc("not-base64!!!").is_err());
  }
}
