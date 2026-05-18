use std::collections::HashMap;

const ESC_DOLLAR: &str = "\0ESC_DOLLAR\0";
const ESC_LBRACE: &str = "\0ESC_LBRACE\0";
const ESC_RBRACE: &str = "\0ESC_RBRACE\0";
const ESC_SLASH: &str = "\0ESC_SLASH\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpressionError {
 pub error_type: ExpressionErrorType,
 pub message: String,
 pub position: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionErrorType {
 Syntax,
 Order,
 Reference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseExpressionResult {
 pub is_valid: bool,
 pub placeholders: Vec<String>,
 pub errors: Vec<ExpressionError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateExpressionResult {
 pub is_valid: bool,
 pub errors: Vec<ExpressionError>,
}

pub fn parse_expression(expression: &str) -> ParseExpressionResult {
 let mut result = ParseExpressionResult {
 is_valid: true,
 placeholders: Vec::new(),
 errors: Vec::new(),
 };

 if expression.is_empty() {
 return result;
 }

 let preprocessed = preprocess_expression(expression);
 if !preprocessed.errors.is_empty() {
 result.is_valid = false;
 result.errors = preprocessed.errors;
 return result;
 }

 if let Some(error) = validate_placeholder_boundaries(&preprocessed.processed) {
 result.is_valid = false;
 result.errors.push(error);
 return result;
 }

 let mut rest = preprocessed.processed.as_str();
 let mut base_offset = 0usize;

 while let Some(start) = rest.find("${") {
 let absolute_start = base_offset + start;
 let after_start = &rest[start + 2..];

 let Some(end) = after_start.find('}') else {
 break;
 };

 let placeholder = &after_start[..end];
 if placeholder.trim().is_empty() {
 result.is_valid = false;
 result.errors.push(ExpressionError {
 error_type: ExpressionErrorType::Syntax,
 message: "表达式语法错误：占位符内容为空".to_string(),
 position: Some(absolute_start),
 });
 } else if !result
 .placeholders
 .iter()
 .any(|existing| existing == placeholder)
 {
 result.placeholders.push(placeholder.to_string());
 }

 let consumed = start + 2 + end + 1;
 base_offset += consumed;
 rest = &rest[consumed..];
 }

 result
}

pub fn resolve_expression(expression: &str, values: &HashMap<String, String>) -> String {
 if expression.is_empty() {
 return String::new();
 }

 let preprocessed = preprocess_expression(expression);
 let mut result = String::with_capacity(preprocessed.processed.len());
 let mut rest = preprocessed.processed.as_str();

 while let Some(start) = rest.find("${") {
 result.push_str(&rest[..start]);
 let after_start = &rest[start + 2..];

 let Some(end) = after_start.find('}') else {
 result.push_str(&rest[start..]);
 return postprocess_expression(&result);
 };

 let placeholder = &after_start[..end];
 if let Some(value) = values.get(placeholder) {
 result.push_str(value);
 }

 rest = &after_start[end + 1..];
 }

 result.push_str(rest);
 postprocess_expression(&result)
}

pub fn validate_expression_refs(
 expression: &str,
 available_refs: &[String],
 subsequent_refs: &[String],
) -> ValidateExpressionResult {
 let parse_result = parse_expression(expression);
 let mut errors = parse_result.errors;

 if parse_result.is_valid {
 for placeholder in parse_result.placeholders {
 if subsequent_refs.iter().any(|value| value == &placeholder) {
 errors.push(ExpressionError {
 error_type: ExpressionErrorType::Order,
 message: format!("不能引用后续组件的输出：\"{placeholder}\""),
 position: None,
 });
 continue;
 }

 if !available_refs.iter().any(|value| value == &placeholder) {
 errors.push(ExpressionError {
 error_type: ExpressionErrorType::Reference,
 message: format!(
 "引用的变量 \"{placeholder}\" 不存在（请检查输入映射或组件输出）"
 ),
 position: None,
 });
 }
 }
 }

 ValidateExpressionResult {
 is_valid: errors.is_empty(),
 errors,
 }
}

pub fn is_pure_text(expression: &str) -> bool {
 if expression.is_empty() {
 return true;
 }

 let preprocessed = preprocess_expression(expression);
 !preprocessed.processed.contains("${")
}

pub fn escape_expression(text: &str) -> String {
 if text.is_empty() {
 return String::new();
 }

 text.replace('/', "//")
 .replace('$', "/$")
 .replace('{', "/{")
 .replace('}', "/}")
}

struct PreprocessResult {
 processed: String,
 errors: Vec<ExpressionError>,
}

fn preprocess_expression(expression: &str) -> PreprocessResult {
 let mut processed = expression.replace("//", ESC_SLASH);
 processed = processed.replace("/$", ESC_DOLLAR);
 processed = processed.replace("/{", ESC_LBRACE);
 processed = processed.replace("/}", ESC_RBRACE);

 let mut errors = Vec::new();
 for (position, character) in processed.char_indices() {
 if character == '/' {
 errors.push(ExpressionError {
 error_type: ExpressionErrorType::Syntax,
 message: "表达式语法错误：无效的转义字符 \"/\"，只能转义 $ { } /".to_string(),
 position: Some(position),
 });
 }
 }

 PreprocessResult { processed, errors }
}

fn postprocess_expression(expression: &str) -> String {
 expression
 .replace(ESC_DOLLAR, "$")
 .replace(ESC_LBRACE, "{")
 .replace(ESC_RBRACE, "}")
 .replace(ESC_SLASH, "/")
}

fn validate_placeholder_boundaries(processed: &str) -> Option<ExpressionError> {
 let mut in_placeholder = false;
 let mut iter = processed.char_indices().peekable();

 while let Some((position, character)) = iter.next() {
 if character == '$' && iter.peek().is_some_and(|(_, next)| *next == '{') {
 if in_placeholder {
 return Some(ExpressionError {
 error_type: ExpressionErrorType::Syntax,
 message: "表达式语法错误：不支持嵌套占位符".to_string(),
 position: Some(position),
 });
 }

 in_placeholder = true;
 iter.next();
 } else if character == '}' && in_placeholder {
 in_placeholder = false;
 }
 }

 in_placeholder.then_some(ExpressionError {
 error_type: ExpressionErrorType::Syntax,
 message: "表达式语法错误：占位符未闭合".to_string(),
 position: None,
 })
}

#[cfg(test)]
mod tests {
 use std::collections::HashMap;

 use super::{
 escape_expression, is_pure_text, parse_expression, resolve_expression,
 validate_expression_refs, ExpressionErrorType,
 };

 #[test]
 fn parse_expression_extracts_unique_placeholders() {
 let result = parse_expression("a ${one} b ${two} c ${one}");

 assert!(result.is_valid);
 assert_eq!(result.placeholders, vec!["one", "two"]);
 assert!(result.errors.is_empty());
 }

 #[test]
 fn parse_expression_reports_frontend_syntax_errors() {
 let invalid_escape = parse_expression("a /x");
 assert!(!invalid_escape.is_valid);
 assert_eq!(
 invalid_escape.errors[0].error_type,
 ExpressionErrorType::Syntax
 );
 assert!(invalid_escape.errors[0].message.contains("无效的转义字符"));

 let unclosed = parse_expression("${name");
 assert!(!unclosed.is_valid);
 assert!(unclosed.errors[0].message.contains("占位符未闭合"));

 let empty = parse_expression("${ }");
 assert!(!empty.is_valid);
 assert!(empty.errors[0].message.contains("占位符内容为空"));

 let nested = parse_expression("${a${b}}");
 assert!(!nested.is_valid);
 assert!(nested.errors[0].message.contains("不支持嵌套占位符"));
 }

 #[test]
 fn resolve_expression_matches_frontend_escape_and_missing_value_behavior() {
 let values = HashMap::from([
 ("name".to_string(), "Rust".to_string()),
 ("slash".to_string(), "/".to_string()),
 ]);

 assert_eq!(
 resolve_expression("hello ${name} ${missing}", &values),
 "hello Rust "
 );
 assert_eq!(
 resolve_expression("/$ /{ /} // ${slash}", &values),
 "$ { } / /"
 );
 }

 #[test]
 fn validate_expression_refs_checks_order_and_existence() {
 let available = vec!["input".to_string(), "prev".to_string()];
 let subsequent = vec!["next".to_string()];
 let result =
 validate_expression_refs("${input}-${next}-${missing}", &available, &subsequent);

 assert!(!result.is_valid);
 assert_eq!(result.errors.len(), 2);
 assert_eq!(result.errors[0].error_type, ExpressionErrorType::Order);
 assert_eq!(result.errors[1].error_type, ExpressionErrorType::Reference);
 }

 #[test]
 fn is_pure_text_ignores_escaped_placeholder_start() {
 assert!(is_pure_text("/${not_ref}"));
 assert!(!is_pure_text("${ref}"));
 }

 #[test]
 fn escape_expression_escapes_frontend_special_chars() {
 assert_eq!(escape_expression("$/{}"), "/$///{/}");
 }
}
