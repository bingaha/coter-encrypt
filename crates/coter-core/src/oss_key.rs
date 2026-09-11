//! OSS key extraction from free-form text.
//!
//! A pasted payload (for example a JSON document produced by a business
//! system) may embed one or more OSS keys among arbitrary text. OSS keys are
//! Base64 strings, so every maximal run of Base64 alphabet characters is a
//! candidate token. Anything else — symbols, whitespace, CJK characters —
//! acts as a separator and is never part of a match. This keeps the Base64
//! payload intact while still splitting the surrounding document.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;

/// Minimum length for a token to be considered an OSS key.
///
/// Derived from the reference key
/// `cCtOcjUxblV1QnduZ1hxZjVFZkk2bndRRVNaTTZ4QzVXRC9JNWU2SGhaUT0=` (60
/// characters) minus 10.
pub const MIN_OSS_KEY_LENGTH: usize = 50;

/// A Base64-looking token located inside a larger text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OssKeyMatch {
 pub start: usize,
 pub end: usize,
 pub value: String,
}

fn is_base64_alphabet(c: char) -> bool {
 c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')
}

fn is_valid_oss_key(token: &str) -> bool {
 // Base64 with padding is always a multiple of four characters, and the
 // canonical decoder rejects misplaced padding.
 token.len() % 4 == 0 && matches!(STANDARD.decode(token), Ok(bytes) if !bytes.is_empty())
}

fn push_match(text: &str, start: usize, end: usize, matches: &mut Vec<OssKeyMatch>) {
 let token = &text[start..end];
 if token.len() >= MIN_OSS_KEY_LENGTH && is_valid_oss_key(token) {
 matches.push(OssKeyMatch {
 start,
 end,
 value: token.to_string(),
 });
 }
}

/// Find every Base64 OSS key candidate in `text`, in source order.
pub fn find_oss_keys(text: &str) -> Vec<OssKeyMatch> {
 let mut matches = Vec::new();
 let mut run_start: Option<usize> = None;
 let mut run_end = 0usize;

 for (index, ch) in text.char_indices() {
 if is_base64_alphabet(ch) {
 if run_start.is_none() {
 run_start = Some(index);
 }
 run_end = index + ch.len_utf8();
 } else if let Some(start) = run_start.take() {
 push_match(text, start, run_end, &mut matches);
 }
 }

 if let Some(start) = run_start {
 push_match(text, start, run_end, &mut matches);
 }

 matches
}

/// How a slice of the rebuilt text relates to the original document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OssTextSegmentKind {
 /// Text that was not a recognised key.
 Plain,
 /// A key that was replaced by its converted value.
 Converted,
 /// A key that was recognised but could not be converted.
 Failed,
}

/// One contiguous slice of the rebuilt text, tagged for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OssTextSegment {
 pub text: String,
 pub kind: OssTextSegmentKind,
}

/// Rebuild `text`, replacing every matched key through `resolve`.
///
/// `resolve` returns `Some(new_key)` for a successful conversion and `None`
/// when the key could not be converted. Successful keys are substituted into
/// the output and tagged `Converted`; everything else (including unconverted
/// keys) is preserved verbatim. The returned segments let the UI colour the
/// result without recomputing offsets.
pub fn replace_oss_keys<F>(
 text: &str,
 matches: &[OssKeyMatch],
 resolve: F,
) -> (String, Vec<OssTextSegment>)
where
 F: Fn(&str) -> Option<String>,
{
 let mut segments = Vec::new();
 let mut converted_text = String::with_capacity(text.len());
 let mut cursor = 0usize;

 for matched in matches {
 if matched.start > cursor {
 let plain = &text[cursor..matched.start];
 converted_text.push_str(plain);
 segments.push(OssTextSegment {
 text: plain.to_string(),
 kind: OssTextSegmentKind::Plain,
 });
 }

 match resolve(&matched.value) {
 Some(new_key) => {
 converted_text.push_str(&new_key);
 segments.push(OssTextSegment {
 text: new_key,
 kind: OssTextSegmentKind::Converted,
 });
 }
 None => {
 converted_text.push_str(&matched.value);
 segments.push(OssTextSegment {
 text: matched.value.clone(),
 kind: OssTextSegmentKind::Failed,
 });
 }
 }

 cursor = matched.end;
 }

 if cursor < text.len() {
 let tail = &text[cursor..];
 converted_text.push_str(tail);
 segments.push(OssTextSegment {
 text: tail.to_string(),
 kind: OssTextSegmentKind::Plain,
 });
 }

 (converted_text, segments)
}

#[cfg(test)]
mod tests {
 use super::*;

 const KEY_A: &str = "aGd6T0NpWDBHQ2wySStsVlBpS3FES0paenVtVE92bFZGUGpOUFgxdGFuVT0=";
 const KEY_B: &str = "MENZMThkM1RDc0tzU0RKdW1NWkhIdXhuaDZUQnlHd2p4YW5YcnhUb1ZEbz0=";
 const REFERENCE_KEY: &str = "cCtOcjUxblV1QnduZ1hxZjVFZkk2bndRRVNaTTZ4QzVXRC9JNWU2SGhaUT0=";

 #[test]
 fn reference_key_meets_minimum_length() {
 assert_eq!(REFERENCE_KEY.len(), MIN_OSS_KEY_LENGTH + 10);
 assert_eq!(find_oss_keys(REFERENCE_KEY).len(), 1);
 }

 #[test]
 fn extracts_keys_embedded_in_json() {
 let text = format!(r#"{{"fileOssKey":"{KEY_A}","files":[{{"ossKey":"{KEY_B}"}}]}}"#);

 let found = find_oss_keys(&text);

 assert_eq!(found.len(), 2);
 assert_eq!(found[0].value, KEY_A);
 assert_eq!(found[1].value, KEY_B);
 assert_eq!(&text[found[0].start..found[0].end], KEY_A);
 assert_eq!(&text[found[1].start..found[1].end], KEY_B);
 }

 #[test]
 fn symbols_whitespace_and_cjk_are_separators() {
 let text = format!("身份证正面 {KEY_A}，文件：{KEY_B}");

 let found = find_oss_keys(&text);

 assert_eq!(found.len(), 2);
 assert_eq!(found[0].value, KEY_A);
 assert_eq!(found[1].value, KEY_B);
 }

 #[test]
 fn ignores_tokens_shorter_than_minimum() {
 // 48 Base64 characters decode cleanly but are below the threshold.
 let short = STANDARD.encode(vec![b'a'; 36]);
 assert_eq!(short.len(), 48);
 assert!(find_oss_keys(&short).is_empty());

 // 52 Base64 characters decode cleanly and clear the threshold.
 let long = STANDARD.encode(vec![b'a'; 39]);
 assert_eq!(long.len(), 52);
 assert_eq!(find_oss_keys(&long).len(), 1);
 }

 #[test]
 fn ignores_long_but_invalid_base64_runs() {
 // 51 alphabet characters: long enough, but not valid Base64.
 let invalid = "A".repeat(51);
 assert!(find_oss_keys(&invalid).is_empty());
 }

 #[test]
 fn keeps_multiple_occurrences_separate() {
 let text = format!("{KEY_A}\n{KEY_B}");

 let found = find_oss_keys(&text);

 assert_eq!(found.len(), 2);
 assert_eq!(found[0].value, KEY_A);
 assert_eq!(found[1].value, KEY_B);
 }

 #[test]
 fn replaces_only_resolved_keys_and_preserves_the_rest() {
 let text = format!("前 {KEY_A} 中 {KEY_B} 后");
 let found = find_oss_keys(&text);
 let replacement = "TkVXX0tFWQ==";

 let (converted, segments) =
 replace_oss_keys(&text, &found, |key| (key == KEY_A).then(|| replacement.to_string()));

 assert_eq!(
 converted,
 format!("前 {replacement} 中 {KEY_B} 后")
 );
 assert_eq!(segments.len(), 5);
 assert_eq!(segments[0].kind, OssTextSegmentKind::Plain);
 assert_eq!(segments[0].text, "前 ");
 assert_eq!(segments[1].kind, OssTextSegmentKind::Converted);
 assert_eq!(segments[1].text, replacement);
 assert_eq!(segments[2].kind, OssTextSegmentKind::Plain);
 assert_eq!(segments[2].text, " 中 ");
 assert_eq!(segments[3].kind, OssTextSegmentKind::Failed);
 assert_eq!(segments[3].text, KEY_B);
 assert_eq!(segments[4].kind, OssTextSegmentKind::Plain);
 assert_eq!(segments[4].text, " 后");
 }

 #[test]
 fn repeated_key_is_replaced_everywhere() {
 let text = format!(r#"{{"a":"{KEY_A}","b":"{KEY_A}"}}"#);
 let found = find_oss_keys(&text);

 let (converted, segments) =
 replace_oss_keys(&text, &found, |_| Some("bmV3".to_string()));

 assert_eq!(converted, r#"{"a":"bmV3","b":"bmV3"}"#);
 assert_eq!(
 segments
 .iter()
 .filter(|segment| segment.kind == OssTextSegmentKind::Converted)
 .count(),
 2
 );
 }

 #[test]
 fn empty_text_produces_no_segments() {
 let (converted, segments) = replace_oss_keys("", &[], |_| None);

 assert_eq!(converted, "");
 assert!(segments.is_empty());
 }

 #[test]
 fn segment_serialization_matches_ui_contract() {
 let text = format!("keep {KEY_A}");
 let found = find_oss_keys(&text);
 let (_, segments) = replace_oss_keys(&text, &found, |_| Some("bmV3".to_string()));

 let json = serde_json::to_value(&segments).unwrap();

 assert_eq!(json[0]["text"], "keep ");
 assert_eq!(json[0]["kind"], "plain");
 assert_eq!(json[1]["text"], "bmV3");
 assert_eq!(json[1]["kind"], "converted");
 }

 #[test]
 fn finds_repeated_keys_in_a_business_document() {
 // `extendFields.files` holds JSON as an escaped string, while `files` is a
 // real array; both repeat the same two keys.
 let embedded = format!(r#"[{{"ossKey":"{KEY_A}"}},{{"ossKey":"{KEY_B}"}}]"#);
 let escaped = embedded.replace('"', "\\\"");
 let text = format!(
 r#"{{"extFiles":[{{"fileOssKey":"{KEY_A}"}},{{"fileOssKey":"{KEY_B}"}}],"extendFields":{{"files":"{escaped}"}},"files":[{{"ossKey":"{KEY_A}"}},{{"ossKey":"{KEY_B}"}}],"idcardNo":"420684199201137513"}}"#
 );

 let found = find_oss_keys(&text);
 let mut distinct: Vec<&str> = found.iter().map(|m| m.value.as_str()).collect();
 distinct.sort_unstable();
 distinct.dedup();

 let mut expected = vec![KEY_A, KEY_B];
 expected.sort_unstable();

 assert_eq!(found.len(), 6);
 assert_eq!(distinct, expected);
 }
}
