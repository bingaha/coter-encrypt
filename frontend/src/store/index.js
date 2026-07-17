import { defineStore } from 'pinia'
import { validateExpressionRefs } from '@/utils/expressionParser'

// 组件分类定义
const componentCategories = [
 {
 id: 'encoding',
 name: '编码类',
 icon: 'CodeOutline',
 collapsed: false,
 components: [
 { id: 'base64', name: 'Base64', icon: 'DocumentTextOutline', description: 'Base64 编码/解码' },
 { id: 'hex', name: 'Hex', icon: 'CodeSlashOutline', description: '十六进制编码/解码' },
 { id: 'radix', name: '进制转换', icon: 'CalculatorOutline', description: '2 到 36 进制数字转换' },
 { id: 'url', name: 'URL', icon: 'LinkOutline', description: 'URL 编码/解码' },
 { id: 'unicode', name: 'Unicode', icon: 'TextOutline', description: 'Unicode 编码/解码' }
 ]
 },
 {
 id: 'hash',
 name: '哈希类',
 icon: 'FingerPrintOutline',
 collapsed: false,
 components: [
 { id: 'md5', name: 'MD5', icon: 'LockClosedOutline', description: 'MD5 哈希算法' },
 { id: 'sha', name: 'SHA', icon: 'LockClosedOutline', description: 'SHA 系列哈希算法' },
 { id: 'hmacmd5', name: 'HmacMD5', icon: 'KeyOutline', description: 'HmacMD5 消息认证码' },
 { id: 'hmacsha', name: 'HmacSHA', icon: 'KeyOutline', description: 'HmacSHA 系列消息认证码' },
 { id: 'sm3', name: 'SM3', icon: 'ShieldOutline', description: '国密 SM3 哈希算法' }
 ]
 },
 {
 id: 'symmetric',
 name: '对称加密',
 icon: 'KeyOutline',
 collapsed: false,
 components: [
 { id: 'aes', name: 'AES', icon: 'KeyOutline', description: 'AES 对称加密算法' },
 { id: 'blowfish', name: 'Blowfish', icon: 'KeyOutline', description: 'Blowfish 对称加密算法' },
 { id: 'sm4', name: 'SM4', icon: 'ShieldCheckmarkOutline', description: '国密 SM4 对称加密算法' }
 ]
 },
 {
 id: 'asymmetric',
 name: '非对称加密',
 icon: 'ShieldCheckmarkOutline',
 collapsed: false,
 components: [
 { id: 'sm2', name: 'SM2', icon: 'ShieldOutline', description: '国密 SM2 椭圆曲线加密' },
 { id: 'rsa', name: 'RSA', icon: 'KeyOutline', description: 'RSA 非对称加密算法' }
 ]
 }
]

// 组件配置store
export const useConfigStore = defineStore('config', {
 state: () => ({
 // 当前项目信息
 currentProject: null,
 
 // 组件分类列表
 componentCategories: JSON.parse(JSON.stringify(componentCategories)),
 
 // 扁平化的可用组件列表（兼容旧代码）
 availableComponents: componentCategories.flatMap(cat => cat.components),
 
 // 右侧已添加组件列表
 addedComponents: [],
 
 // 当前选中的组件
 selectedComponent: null,
 
 // 输入映射配置
 inputMappings: [],
 
 // 输出映射配置
 outputMappings: [],
 
 // 执行配置
 executionConfig: {
 inputs: [],
 outputs: []
 },
 
 // 主题设置
 isDarkMode: localStorage.getItem('theme') === 'dark'
 }),
 
 getters: {
 // 获取当前项目名称
 currentProjectName: (state) => state.currentProject?.name || '',
 
 // 获取当前项目ID
 currentProjectId: (state) => state.currentProject?.id || null,
 
 // 检查是否有未保存的更改
 hasUnsavedChanges: (state) => {
 // 简单实现：如果有已添加的组件就认为有更改
 return state.addedComponents.length > 0
 }
 },
 
 actions: {
 // 设置当前项目
 setCurrentProject(project) {
 this.currentProject = project ? { ...project } : null
 },
 
 // 清除当前项目
 clearCurrentProject() {
 this.currentProject = null
 },
 
 // 切换分类折叠状态
 toggleCategoryCollapse(categoryId) {
 const category = this.componentCategories.find(c => c.id === categoryId)
 if (category) {
 category.collapsed = !category.collapsed
 }
 },
 
 // 切换主题
 toggleTheme() {
 this.isDarkMode = !this.isDarkMode
 localStorage.setItem('theme', this.isDarkMode ? 'dark' : 'light')
 },
 
 // 添加组件
 addComponent(componentType) {
 const component = this.availableComponents.find(c => c.id === componentType)
 if (component) {
 const newComponent = {
 id: `${componentType}-${Date.now()}`,
 type: componentType,
 name: component.name,
 icon: component.icon,
 config: this.getDefaultConfig(componentType),
 inputRef: '',
 outputRef: `${componentType}-output-${Date.now()}`
 }
 this.addedComponents.push(newComponent)
 return newComponent
 }
 },

 // 删除组件
 removeComponent(componentId) {
 const index = this.addedComponents.findIndex(c => c.id === componentId)
 if (index !== -1) {
 const removedComponent = this.addedComponents[index]
 this.addedComponents.splice(index, 1)
 
 // 清除其他组件中对被删除组件的引用
 this.addedComponents.forEach(component => {
 if (component.config.inputComponentRef === componentId) {
 component.config.inputComponentRef = ''
 component.config.inputSourceType = 'param'
 }
 })
 
 // 清除outputMappings中引用被删除组件的映射
 this.outputMappings = this.outputMappings.filter(
 mapping => mapping.componentRef !== removedComponent.outputRef
 )
 
 // 清除引用该组件输出的下游组件参数引用配置
 const outputRefToRemove = `output:${removedComponent.outputRef}`
 this.addedComponents.forEach(comp => {
 const config = comp.config
 if (config.keyRef === outputRefToRemove) config.keyRef = ''
 if (config.ivRef === outputRefToRemove) config.ivRef = ''
 if (config.publicKeyRef === outputRefToRemove) config.publicKeyRef = ''
 if (config.privateKeyRef === outputRefToRemove) config.privateKeyRef = ''
 })
 }
 // 如果删除的是当前选中的组件，清空选中状态
 if (this.selectedComponent && this.selectedComponent.id === componentId) {
 this.selectedComponent = null
 }
 },
 
 // 更新组件配置
 updateComponentConfig(componentId, config) {
 const component = this.addedComponents.find(c => c.id === componentId)
 if (component) {
 component.config = { ...component.config, ...config }
 }
 },
 
 // 更新组件输出标识
 updateComponentOutputRef(componentId, newOutputRef) {
 const component = this.addedComponents.find(c => c.id === componentId)
 if (component) {
 const oldOutputRef = component.outputRef
 component.outputRef = newOutputRef
 
 // 更新其他组件中对该输出标识的引用
 this.outputMappings.forEach(mapping => {
 if (mapping.componentRef === oldOutputRef) {
 mapping.componentRef = newOutputRef
 }
 })
 }
 },
 
 // 选择组件
 selectComponent(componentId) {
 this.selectedComponent = this.addedComponents.find(c => c.id === componentId)
 },
 
 // 获取可用输入源（位于当前组件之前的所有组件）
 getAvailableInputSources(componentId) {
 const currentIndex = this.addedComponents.findIndex(c => c.id === componentId)
 if (currentIndex <= 0) {
 return []
 }
 return this.addedComponents.slice(0, currentIndex).map(c => ({
 id: c.id,
 name: c.name,
 outputRef: c.outputRef
 }))
 },
 
 // 添加输出映射
 addOutputMapping(name, componentRef) {
 const mapping = {
 id: `output-mapping-${Date.now()}`,
 name: name || '',
 componentRef: componentRef || ''
 }
 this.outputMappings.push(mapping)
 return mapping
 },
 
 // 删除输出映射
 removeOutputMapping(id) {
 const index = this.outputMappings.findIndex(m => m.id === id)
 if (index !== -1) {
 this.outputMappings.splice(index, 1)
 }
 },
 
 // 更新输出映射
 updateOutputMapping(id, data) {
 const mapping = this.outputMappings.find(m => m.id === id)
 if (mapping) {
 Object.assign(mapping, data)
 }
 },
 
 // 添加输入映射
 addInputMapping(name = '') {
 const timestamp = Date.now()
 const mapping = {
 id: `input-mapping-${timestamp}`,
 name: name,
 inputRef: `input-${timestamp}`,
 defaultValue: '' // 默认值字段
 }
 this.inputMappings.push(mapping)
 return mapping
 },
 
 // 删除输入映射
 removeInputMapping(id) {
 const index = this.inputMappings.findIndex(m => m.id === id)
 if (index !== -1) {
 const removedMapping = this.inputMappings[index]
 this.inputMappings.splice(index, 1)
 
 // 更新引用该映射的组件
 this.addedComponents.forEach(component => {
 if (component.config.inputMappingRef === removedMapping.inputRef) {
 component.config.inputMappingRef = ''
 // 如果没有其他输入映射可用，切换到表达式模式
 if (this.inputMappings.length === 0) {
 component.config.inputSourceType = 'expression'
 }
 }
 })
 
 // 清除引用该输入映射的组件参数引用配置
 const inputRefToRemove = `input:${removedMapping.inputRef}`
 this.addedComponents.forEach(comp => {
 const config = comp.config
 if (config.keyRef === inputRefToRemove) config.keyRef = ''
 if (config.ivRef === inputRefToRemove) config.ivRef = ''
 if (config.publicKeyRef === inputRefToRemove) config.publicKeyRef = ''
 if (config.privateKeyRef === inputRefToRemove) config.privateKeyRef = ''
 })
 }
 },
 
 // 更新输入映射
 updateInputMapping(id, data) {
 const mapping = this.inputMappings.find(m => m.id === id)
 if (mapping) {
 Object.assign(mapping, data)
 }
 },
 
 // 获取可用的输入映射列表
 getAvailableInputMappings() {
 return this.inputMappings.map(m => ({
 label: m.name || m.inputRef,
 value: m.inputRef
 }))
 },
 
 // 校验单个组件配置完整性
 validateComponent(component) {
 const errors = []
 const config = component.config
 
 // 校验输入来源配置
 if (!config.inputSourceType) {
 errors.push({
 field: 'inputSourceType',
 message: '请选择输入来源类型'
 })
 } else if (config.inputSourceType === 'reference' || config.inputSourceType === 'inputMapping' || config.inputSourceType === 'component') {
 // 引用类型校验（输入映射或组件输出）
 if (!config.inputMappingRef) {
 errors.push({
 field: 'inputMappingRef',
 message: '请选择输入来源'
 })
 }
 } else if (config.inputSourceType === 'expression') {
 // 表达式输入校验
 if (config.inputExpression) {
 // 获取当前组件索引
 const currentIndex = this.addedComponents.findIndex(c => c.id === component.id)
 
 // 获取可用的输出引用（当前组件之前的组件）
 const availableRefs = this.addedComponents
 .slice(0, currentIndex)
 .map(c => c.outputRef)
 
 // 添加输入映射引用到可用引用列表
 const inputMappingRefs = this.inputMappings.map(m => m.inputRef)
 const allAvailableRefs = [...availableRefs, ...inputMappingRefs]
 
 // 获取后续组件的输出引用
 const subsequentRefs = this.addedComponents
 .slice(currentIndex + 1)
 .map(c => c.outputRef)
 
 // 校验表达式
 const validationResult = validateExpressionRefs(
 config.inputExpression,
 allAvailableRefs,
 subsequentRefs
 )
 
 if (!validationResult.isValid) {
 validationResult.errors.forEach(err => {
 errors.push({
 field: 'inputExpression',
 message: err.message
 })
 })
 }
 }
 }
 
 // 算法特定必填项校验（保留旧的直接值校验以保持向后兼容）
 // 参数引用校验（新增）
 switch (component.type) {
 case 'aes':
 case 'blowfish':
 case 'sm4':
 // 密钥校验：优先检查引用，如果没有引用则检查直接值
 if (!config.keyRef) {
 // 没有配置引用，检查是否有直接值（向后兼容）
 if (!config.key) {
 errors.push({ field: 'keyRef', message: '请选择密钥来源' })
 }
 } else if (!this.isValidRef(config.keyRef, component.id)) {
 errors.push({ field: 'keyRef', message: '密钥引用无效' })
 }
 // IV向量校验（非ECB模式下必填）
 if (config.mode && config.mode !== 'ECB') {
 if (!config.ivRef) {
 // 没有配置引用，检查是否有直接值（向后兼容）
 if (!config.iv) {
 errors.push({ field: 'ivRef', message: `${config.mode}模式下请选择IV向量来源` })
 }
 } else if (!this.isValidRef(config.ivRef, component.id)) {
 errors.push({ field: 'ivRef', message: 'IV向量引用无效' })
 }
 }
 break
 case 'sm2':
 case 'rsa':
 // 公钥校验（加密操作时必填）
 if (config.operation === 'encrypt') {
 if (!config.publicKeyRef) {
 // 没有配置引用，检查是否有直接值（向后兼容）
 if (!config.publicKey) {
 errors.push({ field: 'publicKeyRef', message: '加密操作请选择公钥来源' })
 }
 } else if (!this.isValidRef(config.publicKeyRef, component.id)) {
 errors.push({ field: 'publicKeyRef', message: '公钥引用无效' })
 }
 }
 // 私钥校验（解密操作时必填）
 if (config.operation === 'decrypt') {
 if (!config.privateKeyRef) {
 // 没有配置引用，检查是否有直接值（向后兼容）
 if (!config.privateKey) {
 errors.push({ field: 'privateKeyRef', message: '解密操作请选择私钥来源' })
 }
 } else if (!this.isValidRef(config.privateKeyRef, component.id)) {
 errors.push({ field: 'privateKeyRef', message: '私钥引用无效' })
 }
 }
 break
 case 'hmacmd5':
 case 'hmacsha':
 // 密钥校验
 if (!config.keyRef) {
 // 没有配置引用，检查是否有直接值（向后兼容）
 if (!config.key) {
 errors.push({ field: 'keyRef', message: '请选择密钥来源' })
 }
 } else if (!this.isValidRef(config.keyRef, component.id)) {
 errors.push({ field: 'keyRef', message: '密钥引用无效' })
 }
 break
 }
 
 return errors
 },
 
 // 校验整体配置完整性
 validateConfig() {
 const errors = []
 
 this.addedComponents.forEach(component => {
 const componentErrors = this.validateComponent(component)
 componentErrors.forEach(error => {
 errors.push({
 componentId: component.id,
 componentName: component.name,
 field: error.field,
 message: error.message
 })
 })
 })
 
 return {
 isValid: errors.length === 0,
 errors
 }
 },
 
 // 校验引用有效性
 // 解析引用格式 `{type}:{identifier}`
 // 验证 input 类型引用在 inputMappings 中存在
 // 验证 output 类型引用在可用组件输出中存在
 isValidRef(ref, componentId) {
 if (!ref) return false
 
 const [type, id] = ref.split(':')
 if (type === 'input') {
 // 验证输入映射引用是否存在
 return this.inputMappings.some(m => m.inputRef === id)
 } else if (type === 'output') {
 // 验证组件输出引用是否存在于可用的上游组件中
 const sources = this.getAvailableInputSources(componentId)
 return sources.some(s => s.outputRef === id)
 }
 return false
 },

 // 获取组件默认配置
 getDefaultConfig(componentType) {
 const commonInputConfig = {
 inputSourceType: 'reference',
 inputMappingRef: '',
 inputComponentRef: '',
 inputExpression: ''
 }
 
 switch (componentType) {
 case 'base64':
 return {
 ...commonInputConfig,
 operation: 'encode',
 outputFormat: 'base64',
 hexCase: 'uppercase',
 inputParam: '',
 outputParam: ''
 }
 case 'md5':
 return {
 ...commonInputConfig,
 resultFormat: 'lowercase',
 outputLength: 32, // 输出位数：16/32
 inputParam: '',
 outputParam: ''
 }
 case 'sha':
 return {
 ...commonInputConfig,
 shaType: 'SHA256', // SHA算法类型：SHA1/SHA256/SHA384/SHA512
 resultFormat: 'lowercase',
 inputParam: '',
 outputParam: ''
 }
 case 'hmacmd5':
 return {
 ...commonInputConfig,
 key: '',
 keyRef: '', // 密钥引用，格式 "input:xxx" 或 "output:xxx"
 resultFormat: 'lowercase',
 outputLength: 32, // 输出位数：16/32
 inputParam: '',
 outputParam: ''
 }
 case 'hmacsha':
 return {
 ...commonInputConfig,
 key: '',
 keyRef: '', // 密钥引用，格式 "input:xxx" 或 "output:xxx"
 hmacShaType: 'HmacSHA256', // HmacSHA算法类型：HmacSHA1/HmacSHA256/HmacSHA384/HmacSHA512
 resultFormat: 'lowercase',
 inputParam: '',
 outputParam: ''
 }
 case 'aes':
 return {
 ...commonInputConfig,
 operation: 'encrypt',
 key: '',
 keyRef: '', // 密钥引用，格式 "input:xxx" 或 "output:xxx"
 mode: 'ECB',
 iv: '',
 ivRef: '', // IV向量引用
 padding: 'PKCS5Padding',
 keyFormat: 'hex',
 ivFormat: 'hex', // IV向量格式：hex/text
 inputFormat: 'text',
 outputFormat: 'base64',
 hexCase: 'uppercase',
 inputParam: '',
 outputParam: ''
 }
 case 'blowfish':
 return {
 ...commonInputConfig,
 operation: 'encrypt',
 key: '',
 keyRef: '',
 mode: 'CBC',
 iv: '',
 ivRef: '',
 padding: 'None',
 keyFormat: 'text',
 ivFormat: 'hex',
 inputFormat: 'hex',
 outputFormat: 'base64',
 hexCase: 'uppercase',
 inputParam: '',
 outputParam: ''
 }
 case 'hex':
 return {
 ...commonInputConfig,
 operation: 'encode',
 inputParam: '',
 outputParam: ''
 }
 case 'radix':
 return {
 ...commonInputConfig,
 inputBase: 10,
 outputBase: 16,
 hexCase: 'uppercase',
 inputParam: '',
 outputParam: ''
 }
 case 'sm2':
 return {
 ...commonInputConfig,
 operation: 'encrypt',
 publicKey: '',
 publicKeyRef: '', // 公钥引用，格式 "input:xxx" 或 "output:xxx"
 privateKey: '',
 privateKeyRef: '', // 私钥引用
 mode: 'C1C3C2',
 inputFormat: 'text',
 outputFormat: 'base64',
 hexCase: 'uppercase',
 inputParam: '',
 outputParam: ''
 }
 case 'sm3':
 return {
 ...commonInputConfig,
 resultFormat: 'lowercase',
 inputParam: '',
 outputParam: ''
 }
 case 'sm4':
 return {
 ...commonInputConfig,
 operation: 'encrypt',
 key: '',
 keyRef: '', // 密钥引用，格式 "input:xxx" 或 "output:xxx"
 mode: 'ECB',
 iv: '',
 ivRef: '', // IV向量引用
 padding: 'pkcs7',
 keyFormat: 'hex',
 ivFormat: 'hex', // IV向量格式：hex/text
 inputFormat: 'text',
 outputFormat: 'base64',
 hexCase: 'uppercase',
 inputParam: '',
 outputParam: ''
 }
 case 'rsa':
 return {
 ...commonInputConfig,
 operation: 'encrypt',
 publicKey: '',
 publicKeyRef: '', // 公钥引用，格式 "input:xxx" 或 "output:xxx"
 privateKey: '',
 privateKeyRef: '', // 私钥引用
 padding: 'PKCS1',
 inputFormat: 'text',
 outputFormat: 'base64',
 hexCase: 'uppercase',
 inputParam: '',
 outputParam: ''
 }
 case 'url':
 return {
 ...commonInputConfig,
 operation: 'encode',
 charset: 'UTF-8',
 inputParam: '',
 outputParam: ''
 }
 case 'unicode':
 return {
 ...commonInputConfig,
 operation: 'encode',
 format: 'standard',
 inputParam: '',
 outputParam: ''
 }
 default:
 return {
 ...commonInputConfig,
 inputParam: '',
 outputParam: ''
 }
 }
 },
 
 // 保存配置到本地存储
 saveConfigToLocal() {
 const config = {
 components: this.addedComponents,
 inputMappings: this.inputMappings,
 outputMappings: this.outputMappings,
 executionConfig: this.executionConfig,
 currentProject: this.currentProject
 }
 localStorage.setItem('encryption-config', JSON.stringify(config))
 return config
 },
 
 // 从本地存储加载配置
 loadConfigFromLocal() {
 const config = localStorage.getItem('encryption-config')
 if (config && config.trim()) {
 try {
 const parsedConfig = JSON.parse(config)
 if (parsedConfig && typeof parsedConfig === 'object') {
 this.addedComponents = parsedConfig.components || []
 this.inputMappings = parsedConfig.inputMappings || []
 this.outputMappings = parsedConfig.outputMappings || []
 this.executionConfig = parsedConfig.executionConfig || { inputs: [], outputs: [] }
 this.currentProject = parsedConfig.currentProject || null
 return parsedConfig
 }
 } catch (error) {
 console.error('加载配置失败:', error)
 }
 // 解析失败时清除损坏的数据
 localStorage.removeItem('encryption-config')
 this.resetConfig()
 }
 return null
 },
 
 // 从项目数据加载配置
 loadConfigFromProject(projectData) {
 if (projectData) {
 this.addedComponents = projectData.components || []
 this.inputMappings = projectData.inputMappings || []
 this.outputMappings = projectData.outputMappings || []
 this.executionConfig = projectData.executionConfig || { inputs: [], outputs: [] }
 }
 },
 
 // 重置配置
 resetConfig() {
 this.addedComponents = []
 this.inputMappings = []
 this.outputMappings = []
 this.executionConfig = { inputs: [], outputs: [] }
 this.selectedComponent = null
 this.currentProject = null
 },
 
 // 获取当前配置数据（用于保存到服务器）
 getConfigData() {
 return {
 components: this.addedComponents,
 inputMappings: this.inputMappings,
 outputMappings: this.outputMappings,
 executionConfig: this.executionConfig
 }
 },
 
 // 兼容旧的 saveConfig 方法
 saveConfig() {
 return this.saveConfigToLocal()
 },
 
 // 兼容旧的 loadConfig 方法
 loadConfig() {
 return this.loadConfigFromLocal()
 }
 }
})

// 执行状态store
export const useExecutionStore = defineStore('execution', {
 state: () => ({
 isExecuting: false,
 inputValues: {},
 executionResults: {},
 executionError: null,
 executionLog: [],
 validationErrors: []
 }),
 
 actions: {
 // 设置输入值
 setInputValue(key, value) {
 this.inputValues[key] = value
 },
 
 // 开始执行
 startExecution() {
 this.isExecuting = true
 this.executionError = null
 this.executionResults = {}
 this.executionLog = []
 },
 
 // 结束执行
 endExecution() {
 this.isExecuting = false
 },
 
 // 设置组件执行结果
 setResult(componentId, result) {
 this.executionResults[componentId] = result
 },
 
 // 设置执行错误
 setError(error) {
 this.executionError = error
 this.isExecuting = false
 },
 
 // 添加执行日志
 addLog(entry) {
 if (typeof entry === 'string') {
 this.executionLog.push({
 timestamp: new Date().toISOString(),
 type: 'info',
 message: entry
 })
 } else {
 this.executionLog.push({
 timestamp: new Date().toISOString(),
 ...entry
 })
 }
 },
 
 // 设置验证错误
 setValidationErrors(errors) {
 this.validationErrors = errors
 },
 
 // 清除验证错误
 clearValidationErrors() {
 this.validationErrors = []
 },
 
 // 重置执行状态
 resetExecution() {
 this.isExecuting = false
 this.inputValues = {}
 this.executionResults = {}
 this.executionError = null
 this.executionLog = []
 this.validationErrors = []
 }
 }
})
