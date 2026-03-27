/**
 * 表达式解析工具
 * 支持 ${outputRef} 语法引用组件输出
 * 支持转义：/$ /{ /} // 分别表示字面量 $ { } /
 */

// 占位符正则表达式
const PLACEHOLDER_REGEX = /\$\{([^}]*)\}/g

// 转义序列占位符（使用不可见字符避免冲突）
const ESC_DOLLAR = '\x00ESC_DOLLAR\x00'
const ESC_LBRACE = '\x00ESC_LBRACE\x00'
const ESC_RBRACE = '\x00ESC_RBRACE\x00'
const ESC_SLASH = '\x00ESC_SLASH\x00'

/**
 * 预处理表达式，将转义序列替换为占位符
 * @param {string} expression - 原始表达式
 * @returns {{ processed: string, errors: Array }} 预处理结果
 */
function preprocessExpression(expression) {
 const errors = []
 let processed = expression

 // 按顺序替换转义序列（先处理 // 避免干扰其他转义）
 processed = processed.split('//').join(ESC_SLASH)
 processed = processed.split('/$').join(ESC_DOLLAR)
 processed = processed.split('/{').join(ESC_LBRACE)
 processed = processed.split('/}').join(ESC_RBRACE)

 // 检查是否有单独的 /（无效转义）
 for (let i = 0; i < processed.length; i++) {
 if (processed[i] === '/') {
 errors.push({
 type: 'syntax',
 message: '表达式语法错误：无效的转义字符 "/"，只能转义 $ { } /',
 position: i
 })
 }
 }

 return { processed, errors }
}

/**
 * 后处理表达式，将占位符还原为字面量
 * @param {string} expression - 预处理后的表达式
 * @returns {string} 还原后的表达式
 */
function postprocessExpression(expression) {
 let result = expression
 result = result.split(ESC_DOLLAR).join('$')
 result = result.split(ESC_LBRACE).join('{')
 result = result.split(ESC_RBRACE).join('}')
 result = result.split(ESC_SLASH).join('/')
 return result
}

/**
 * 解析表达式，提取占位符并校验语法
 * @param {string} expression - 表达式字符串
 * @returns {Object} 解析结果 { isValid, placeholders, errors }
 */
export function parseExpression(expression) {
 const result = {
 isValid: true,
 placeholders: [],
 errors: []
 }

 if (!expression || typeof expression !== 'string') {
 return result
 }

 // 预处理：替换转义序列
 const { processed, errors: preprocessErrors } = preprocessExpression(expression)

 if (preprocessErrors.length > 0) {
 result.isValid = false
 result.errors.push(...preprocessErrors)
 return result
 }

 // 检查未闭合的占位符
 let inPlaceholder = false

 for (let i = 0; i < processed.length; i++) {
 if (processed[i] === '$' && i + 1 < processed.length && processed[i + 1] === '{') {
 if (inPlaceholder) {
 result.isValid = false
 result.errors.push({
 type: 'syntax',
 message: '表达式语法错误：不支持嵌套占位符',
 position: i
 })
 return result
 }
 inPlaceholder = true
 i++ // 跳过 {
 } else if (processed[i] === '}' && inPlaceholder) {
 inPlaceholder = false
 }
 }

 if (inPlaceholder) {
 result.isValid = false
 result.errors.push({
 type: 'syntax',
 message: '表达式语法错误：占位符未闭合'
 })
 return result
 }

 // 提取所有占位符
 const regex = new RegExp(PLACEHOLDER_REGEX.source, 'g')
 let match

 while ((match = regex.exec(processed)) !== null) {
 const placeholder = match[1]

 // 检查空占位符
 if (!placeholder || placeholder.trim() === '') {
 result.isValid = false
 result.errors.push({
 type: 'syntax',
 message: '表达式语法错误：占位符内容为空',
 position: match.index
 })
 continue
 }

 // 添加到占位符列表（去重）
 if (!result.placeholders.includes(placeholder)) {
 result.placeholders.push(placeholder)
 }
 }

 return result
}

/**
 * 解析表达式，替换占位符为实际值
 * @param {string} expression - 表达式字符串
 * @param {Object} outputValues - 组件输出值映射 { outputRef: value }
 * @returns {string} 替换后的字符串
 */
export function resolveExpression(expression, outputValues = {}) {
 if (!expression || typeof expression !== 'string') {
 return ''
 }

 // 预处理：替换转义序列
 const { processed } = preprocessExpression(expression)

 // 替换占位符
 let result = processed.replace(PLACEHOLDER_REGEX, (_, placeholder) => {
 const value = outputValues[placeholder]

 // 如果值未定义或为空，使用空字符串替换
 if (value === undefined || value === null) {
 console.warn('表达式解析警告：引用的组件输出 "' + placeholder + '" 为空或未定义')
 return ''
 }

 return String(value)
 })

 // 后处理：还原转义字符为字面量
 return postprocessExpression(result)
}

/**
 * 校验表达式中的引用是否有效
 * @param {string} expression - 表达式字符串
 * @param {string[]} availableRefs - 可用的输出引用列表
 * @param {string[]} subsequentRefs - 后续组件的输出引用列表（不可引用）
 * @returns {Object} 校验结果 { isValid, errors }
 */
export function validateExpressionRefs(expression, availableRefs = [], subsequentRefs = []) {
 const parseResult = parseExpression(expression)
 const errors = [...parseResult.errors]

 if (!parseResult.isValid) {
 return {
 isValid: false,
 errors
 }
 }

 // 校验每个占位符引用
 for (const placeholder of parseResult.placeholders) {
 // 检查是否引用了后续组件
 if (subsequentRefs.includes(placeholder)) {
 errors.push({
 type: 'order',
 message: '不能引用后续组件的输出："' + placeholder + '"'
 })
 continue
 }

 // 检查引用是否存在
 if (!availableRefs.includes(placeholder)) {
 errors.push({
 type: 'reference',
 message: '引用的变量 "' + placeholder + '" 不存在（请检查输入映射或组件输出）'
 })
 }
 }

 return {
 isValid: errors.length === 0,
 errors
 }
}

/**
 * 检查表达式是否为纯文本（不包含占位符）
 * @param {string} expression - 表达式字符串
 * @returns {boolean} 是否为纯文本
 */
export function isPureText(expression) {
 if (!expression || typeof expression !== 'string') {
 return true
 }
 // 预处理后检查是否还有占位符
 const { processed } = preprocessExpression(expression)
 return !PLACEHOLDER_REGEX.test(processed)
}

/**
 * 转义表达式中的特殊字符
 * 将 $ { } / 转换为 /$ /{ /} //
 * @param {string} text - 需要转义的文本
 * @returns {string} 转义后的文本
 */
export function escapeExpression(text) {
 if (!text || typeof text !== 'string') {
 return ''
 }
 return text
 .replace(/\//g, '//')
 .replace(/\$/g, '/$')
 .replace(/\{/g, '/{')
 .replace(/\}/g, '/}')
}
