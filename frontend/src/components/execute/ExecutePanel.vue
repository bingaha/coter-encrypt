<script setup>
import { ref, computed, watch } from 'vue'
import {
 NCard,
 NButton,
 NIcon,
 NSpace,
 NText,
 NInput,
 NForm,
 NFormItem,
 NAlert,
 NCollapse,
 NCollapseItem,
 NTimeline,
 NTimelineItem,
 NScrollbar,
 NSpin,
 NEmpty,
 NTooltip,
 NDivider,
 NModal,
 NSelect,
 useMessage
} from 'naive-ui'
import {
 PlayOutline,
 CopyOutline,
 CheckmarkCircleOutline,
 CloseCircleOutline,
 TimeOutline,
 AlertCircleOutline,
 ChevronDownOutline,
 ChevronUpOutline,
 DocumentTextOutline,
 FlashOutline
} from '@vicons/ionicons5'
import { useConfigStore, useExecutionStore } from '@/store/index.js'
import { useKeyboardShortcuts } from '@/composables/index.js'
import { isPureText, resolveExpression } from '@/utils/expressionParser.js'
import { useProjectManager } from '@/composables/useProjectManager'
import { executeBatch } from '@/api/encrypt'
import { processHarWithProject } from '@/api/har'
import { open, save } from '@tauri-apps/plugin-dialog'

// Store
const configStore = useConfigStore()
const executionStore = useExecutionStore()
const message = useMessage()

// 本地状态
const inputValues = ref({})
const executionResults = ref({})
const isExecuting = ref(false)
const executionError = ref(null)
const executionLog = ref([])
const showLog = ref(false)
const localValidationErrors = ref([])

// HAR 弹窗相关状态
const harModalVisible = ref(false)
const harInputPath = ref('')
const harOutputPath = ref('')
const harRegexPreset = ref('BASE64')
const harInputOriginalRef = ref('')
const harFinalOutputMappingId = ref('')
const harInputValues = ref({})
const harStats = ref(null)
const isProcessingHar = ref(false)

// 项目保存（用于先保存再处理HAR）
const { handleSave } = useProjectManager()

// 计算属性
const addedComponents = computed(() => configStore.addedComponents)
const hasComponents = computed(() => addedComponents.value.length > 0)
const outputMappings = computed(() => configStore.outputMappings)
const inputMappings = computed(() => configStore.inputMappings)

// 根据输入映射生成输入字段
const inputFields = computed(() => {
 return inputMappings.value.map(mapping => ({
 key: mapping.inputRef,
 label: mapping.name || mapping.inputRef,
 placeholder: mapping.defaultValue || `请输入 ${mapping.name || mapping.inputRef}`,
 defaultValue: mapping.defaultValue || ''
 }))
})

// 需要用户输入的组件（inputSourceType为'param'的组件）- 保留兼容旧配置
const inputComponents = computed(() => {
 return addedComponents.value.filter(component => {
 return component.config.inputSourceType === 'param'
 })
})

// 使用纯文本表达式的组件（无占位符，作为默认值显示）
const pureTextExpressionComponents = computed(() => {
 return addedComponents.value.filter(component => {
 if (component.config.inputSourceType !== 'expression') {
 return false
 }
 const expression = component.config.inputExpression
 return expression && isPureText(expression)
 })
})

// 获取输入框标签
const getInputLabel = (component) => {
 if (component.config.inputParamName && component.config.inputParamName.trim()) {
 return component.config.inputParamName
 }
 return `${component.name} 输入`
}

// 获取组件的校验错误
const getComponentErrors = (componentId) => {
 return localValidationErrors.value.filter(error => error.componentId === componentId)
}

// 检查是否有校验错误（仅配置错误，不包含输入值错误）
const hasConfigErrors = computed(() => {
 return localValidationErrors.value.filter(e => e.field !== 'inputValue').length > 0
})

// 检查所有必填输入是否已填写（有默认值的字段不算空）
const hasEmptyInputs = computed(() => {
 // 只检查输入映射的值
 for (const field of inputFields.value) {
 const inputValue = inputValues.value[field.key]
 // 如果有默认值，则不算空
 if (field.defaultValue) {
 continue
 }
 if (!inputValue || inputValue.trim() === '') {
 return true
 }
 }
 return false
})

// 执行按钮是否禁用
const isExecuteDisabled = computed(() => {
 return hasConfigErrors.value || hasEmptyInputs.value
})

// 检查是否有校验错误（用于显示警告）
const hasValidationErrors = computed(() => localValidationErrors.value.length > 0)

// 下拉选项
const inputMappingOptions = computed(() => inputMappings.value.map(m => ({ label: m.name || m.inputRef, value: m.inputRef })))
const outputMappingOptions = computed(() => outputMappings.value.map(m => ({ label: m.name || m.id, value: m.id })))
const regexOptions = [
 { label: 'BASE64', value: 'BASE64' },
 { label: 'HEX', value: 'HEX' }
]

// 生成输出展示项
const generateOutputs = computed(() => {
 // 如果配置了outputMappings，使用配置的映射
 if (outputMappings.value && outputMappings.value.length > 0) {
 return outputMappings.value.map(mapping => {
 const component = addedComponents.value.find(c => c.outputRef === mapping.componentRef)
 return {
 id: mapping.id,
 name: mapping.name || (component ? `${component.name} 输出` : '未知输出'),
 componentRef: mapping.componentRef,
 componentId: component ? component.id : null
 }
 })
 }
 
 // 如果没有配置outputMappings，显示最后一个组件的输出
 if (addedComponents.value.length > 0) {
 const lastComponent = addedComponents.value[addedComponents.value.length - 1]
 return [{
 id: 'default-output',
 name: `${lastComponent.name} 输出`,
 componentRef: lastComponent.outputRef,
 componentId: lastComponent.id
 }]
 }
 
 return []
})

// 更新校验状态
const updateValidation = () => {
 const result = configStore.validateConfig()
 localValidationErrors.value = result.errors
}

// 监听组件变化，更新校验和输入表单
watch(() => configStore.addedComponents, (newComponents, oldComponents) => {
 updateValidation()
 
 // 清理执行结果（配置变化后需要重新执行）
 if (JSON.stringify(newComponents) !== JSON.stringify(oldComponents)) {
 executionResults.value = {}
 }
}, { deep: true, immediate: true })

// 监听输出映射变化
watch(() => configStore.outputMappings, () => {
 // 输出映射变化时，清空执行结果以便重新计算
 executionResults.value = {}
}, { deep: true })

// 监听输入映射变化，清理已删除的输入映射对应的输入值
watch(() => configStore.inputMappings, (newMappings) => {
 const currentInputRefs = new Set(newMappings.map(m => m.inputRef))
 Object.keys(inputValues.value).forEach(key => {
 if (!currentInputRefs.has(key)) {
 delete inputValues.value[key]
 }
 })
}, { deep: true })

// 监听输入值变化，清除相关的输入值校验错误
watch(inputValues, () => {
 // 清除输入值相关的校验错误
 localValidationErrors.value = localValidationErrors.value.filter(e => e.field !== 'inputValue')
 // 清除执行错误
 executionError.value = null
}, { deep: true })

// 根据inputSourceType获取组件输入数据
const getComponentInput = (component) => {
 const config = component.config
 
 if (config.inputSourceType === 'reference' || config.inputSourceType === 'inputMapping' || config.inputSourceType === 'component') {
 // 引用类型：从输入映射值或组件输出中获取
 const ref = config.inputMappingRef
 if (ref) {
 // 先检查是否是输入映射
 if (inputValues.value[ref] !== undefined) {
 return inputValues.value[ref]
 }
 // 再检查是否是组件输出
 if (executionResults.value[ref] !== undefined) {
 return executionResults.value[ref]
 }
 }
 return ''
 } else if (config.inputSourceType === 'expression') {
 // 表达式输入：解析表达式并替换占位符
 const expression = config.inputExpression || ''
 
 // 构建输出值映射（使用已执行组件的结果 + 输入映射值）
 const outputValues = { ...inputValues.value }
 for (const comp of addedComponents.value) {
 if (executionResults.value[comp.outputRef] !== undefined) {
 outputValues[comp.outputRef] = executionResults.value[comp.outputRef]
 }
 }
 
 // 解析表达式
 return resolveExpression(expression, outputValues)
 }
 
 return ''
}

// 截断长文本
const truncateText = (text, maxLength = 100) => {
 if (!text) return '(空)'
 if (text.length <= maxLength) return text
 return text.substring(0, maxLength) + '...'
}

// 格式化时间
const formatTime = (timestamp) => {
 return new Date(timestamp).toLocaleTimeString()
}

// 获取日志类型
const getLogType = (type) => {
 switch (type) {
 case 'success': return 'success'
 case 'error': return 'error'
 case 'complete': return 'info'
 default: return 'default'
 }
}

// 执行前校验
const validateBeforeExecution = () => {
 const result = configStore.validateConfig()
 localValidationErrors.value = [...result.errors]
 
 // 检查所有输入映射是否都有值（有默认值的字段不校验）
 for (const field of inputFields.value) {
 const inputValue = inputValues.value[field.key]
 // 如果有默认值，则不校验
 if (field.defaultValue) {
 continue
 }
 if (!inputValue || inputValue.trim() === '') {
 localValidationErrors.value.push({
 componentId: field.key,
 componentName: '输入映射',
 field: 'inputValue',
 message: `${field.label} 不能为空`
 })
 }
 }
 
 return localValidationErrors.value.length === 0
}

// 构建输入映射值对象
// 从输入映射配置中收集所有输入值，如果没有输入则使用默认值
const buildInputValues = () => {
 const values = {}
 configStore.inputMappings.forEach(mapping => {
 const userInput = inputValues.value[mapping.inputRef]
 // 如果用户有输入则使用用户输入，否则使用默认值
 values[mapping.inputRef] = (userInput && userInput.trim() !== '') 
 ? userInput 
 : (mapping.defaultValue || '')
 })
 return values
}

// 构建 BatchExecutionRequest 格式的请求
// Requirements: 5.2, 6.2 - 构建包含参数引用和输入映射值的批量执行请求
const buildBatchRequest = () => {
 // 先构建包含默认值的输入映射值
 const resolvedInputValues = buildInputValues()
 
 const components = addedComponents.value.map(comp => {
 const config = comp.config
 
 // 解析输入数据
 let resolvedData = ''
 if (config.inputSourceType === 'reference') {
 // 引用类型：可能是输入映射或组件输出
 const ref = config.inputMappingRef
 if (ref) {
 // 检查是否是输入映射
 if (resolvedInputValues[ref] !== undefined) {
 // 输入映射：直接取值
 resolvedData = resolvedInputValues[ref]
 } else {
 // 组件输出：显式标注为 output: 引用，便于后端解析
 resolvedData = `output:${ref}`
 }
 }
 } else if (config.inputSourceType === 'inputMapping') {
 // 输入映射引用：使用已解析的输入值（包含默认值）
 const ref = config.inputMappingRef
 if (ref && resolvedInputValues[ref] !== undefined) {
 resolvedData = resolvedInputValues[ref]
 }
 } else if (config.inputSourceType === 'component') {
 // 组件输出引用：直接使用引用值，后端会解析
 const ref = config.inputMappingRef
 if (ref) {
 resolvedData = `output:${ref}`
 }
 } else if (config.inputSourceType === 'expression') {
 // 表达式输入：直接传给后端解析，不在此处解析
 // 后端会在执行过程中根据组件输出值来解析表达式
 resolvedData = config.inputExpression || ''
 }
 
 return {
 // 基本信息
 algorithm: comp.type.toUpperCase(),
 operation: config.operation || 'encrypt',
 data: resolvedData,
 outputRef: comp.outputRef,
 
 // 参数引用字段（新增）
 keyRef: config.keyRef || null,
 ivRef: config.ivRef || null,
 publicKeyRef: config.publicKeyRef || null,
 privateKeyRef: config.privateKeyRef || null,
 
 // 直接值字段（向后兼容）
 key: config.key || null,
 iv: config.iv || null,
 publicKey: config.publicKey || null,
 privateKey: config.privateKey || null,
 
 // 其他配置
 mode: config.mode || null,
 padding: config.padding || null,
 inputFormat: config.inputFormat || null,
 outputFormat: config.outputFormat || null,
 keyFormat: config.keyFormat || null,
 ivFormat: config.ivFormat || null,
 hexCase: config.hexCase || null,
 resultFormat: config.resultFormat || null,
 charset: config.charset || null,
 format: config.format || null,
 inputBase: config.inputBase || null,
 outputBase: config.outputBase || null,
 shaType: config.shaType || null,
 hmacShaType: config.hmacShaType || null,
 outputLength: config.outputLength || null
 }
 })
 
 return {
 components,
 inputValues: buildInputValues()
 }
}

// 执行工作流（使用批量执行 API）
const executeWorkflow = async () => {
 if (!validateBeforeExecution()) {
 message.warning('请先完善配置')
 return
 }
 
 isExecuting.value = true
 executionError.value = null
 executionResults.value = {}
 executionLog.value = []
 showLog.value = true
 
 // 同步到store
 executionStore.startExecution()
 
 try {
 // 构建批量执行请求
 const batchRequest = buildBatchRequest()
 
 // 记录开始执行日志
 executionLog.value.push({
 timestamp: new Date().toISOString(),
 type: 'start',
 message: `开始批量执行 ${batchRequest.components.length} 个组件`
 })
 
 // 调用批量执行 API
 const response = await executeBatch(batchRequest)
 const results = response.data
 
 // 处理每个组件的执行结果
 for (let i = 0; i < results.length; i++) {
 const result = results[i]
 const component = addedComponents.value[i]
 
 if (result.status === 'success') {
 executionResults.value[component.id] = result.result
 executionResults.value[component.outputRef] = result.result
 
 executionStore.setResult(component.id, result.result)
 
 executionLog.value.push({
 timestamp: new Date().toISOString(),
 type: 'success',
 componentName: component.name,
 message: `${component.name} 执行成功`,
 output: result.result
 })
 executionStore.addLog({
 type: 'success',
 componentName: component.name,
 message: `${component.name} 执行成功`,
 output: result.result
 })
 } else {
 executionLog.value.push({
 timestamp: new Date().toISOString(),
 type: 'error',
 componentName: component.name,
 message: `${component.name} 执行失败: ${result.message}`
 })
 throw new Error(`${component.name} 执行失败: ${result.message}`)
 }
 }
 
 executionLog.value.push({
 timestamp: new Date().toISOString(),
 type: 'complete',
 message: '所有组件执行完成'
 })
 executionStore.addLog({
 type: 'complete',
 message: '所有组件执行完成'
 })
 
 message.success('执行完成')
 } catch (error) {
 executionError.value = error.message
 executionStore.setError(error.message)
 executionLog.value.push({
 timestamp: new Date().toISOString(),
 type: 'error',
 message: `执行失败: ${error.message}`
 })
 message.error('执行失败')
 } finally {
 isExecuting.value = false
 executionStore.endExecution()
 }
}

// 获取输出结果值
const getOutputResult = (output) => {
 if (output.componentId) {
 return executionResults.value[output.componentId] || executionResults.value[output.componentRef] || ''
 }
 return executionResults.value[output.componentRef] || ''
}

// 复制结果到剪贴板
const copyResult = async (result) => {
 try {
 await navigator.clipboard.writeText(result)
 message.success('已复制到剪贴板')
 } catch (error) {
 message.error('复制失败')
 }
}

// 使用键盘快捷键 composable
// Requirements: 6.6 - Ctrl+Enter 执行工作流
useKeyboardShortcuts({
 onExecute: () => {
 if (!isExecuting.value && !isExecuteDisabled.value && hasComponents.value) {
 executeWorkflow()
 }
 }
})

// 打开HAR处理弹窗：先保存当前配置，再打开
const openHarModal = async () => {
 try {
 await handleSave()
 } catch (e) {
 return
 }
 // 预填
 harRegexPreset.value = 'BASE64'
 harInputOriginalRef.value = inputMappings.value[0]?.inputRef || ''
 harFinalOutputMappingId.value = outputMappings.value[0]?.id || ''
 // 载入现有输入值并用默认值补齐
 const preset = { ...inputValues.value }
 for (const m of inputMappings.value) {
 if (preset[m.inputRef] === undefined && m.defaultValue) preset[m.inputRef] = m.defaultValue
 }
 harInputValues.value = preset
 harStats.value = null
 harInputPath.value = ''
 harOutputPath.value = ''
 harModalVisible.value = true
}

const buildDefaultHarOutputPath = (inputPath) => {
 if (!inputPath) return 'processed.har'

 const separatorIndex = Math.max(inputPath.lastIndexOf('\\'), inputPath.lastIndexOf('/'))
 const directory = separatorIndex >= 0 ? inputPath.slice(0, separatorIndex + 1) : ''
 const fileName = separatorIndex >= 0 ? inputPath.slice(separatorIndex + 1) : inputPath
 const dotIndex = fileName.lastIndexOf('.')
 const baseName = dotIndex > 0 ? fileName.slice(0, dotIndex) : fileName
 return `${directory}${baseName}_解密.har`
}

const selectHarInputFile = async () => {
 try {
 const selected = await open({
 multiple: false,
 filters: [
 { name: 'HAR / JSON', extensions: ['har', 'json'] }
 ]
 })
 if (!selected) return

 harInputPath.value = Array.isArray(selected) ? selected[0] : selected
 if (!harOutputPath.value) {
 harOutputPath.value = buildDefaultHarOutputPath(harInputPath.value)
 }
 } catch (e) {
 message.error(e?.message || '选择 HAR 文件失败')
 }
}

const selectHarOutputFile = async () => {
 try {
 const selected = await save({
 defaultPath: harOutputPath.value || buildDefaultHarOutputPath(harInputPath.value),
 filters: [
 { name: 'HAR', extensions: ['har'] },
 { name: 'JSON', extensions: ['json'] }
 ]
 })
 if (selected) {
 harOutputPath.value = selected
 }
 } catch (e) {
 message.error(e?.message || '选择保存位置失败')
 }
}

// 提交处理
const submitHarProcessing = async () => {
 if (isProcessingHar.value) return

 if (!harInputPath.value) {
 message.warning('请先选择 HAR 文件')
 return
 }
 if (!harOutputPath.value) {
 message.warning('请选择输出保存位置')
 return
 }
 if (!harInputOriginalRef.value) {
 message.warning('请选择“输入原文”的输入映射')
 return
 }
 if (!harFinalOutputMappingId.value) {
 message.warning('请选择“最终输出”的输出映射')
 return
 }
 // 校验除“输入原文”以外的必填输入
 for (const m of inputMappings.value) {
 if (m.inputRef === harInputOriginalRef.value) continue
 const v = (harInputValues.value[m.inputRef] ?? '').trim()
 if (!m.defaultValue && !v) {
 message.warning(`请输入 ${m.name || m.inputRef}`)
 return
 }
 }

 try {
 isProcessingHar.value = true
 const { data } = await processHarWithProject({
 inputPath: harInputPath.value,
 outputPath: harOutputPath.value,
 projectId: configStore.currentProjectId || null,
 projectName: configStore.currentProjectId ? null : configStore.currentProjectName,
 inputOriginalRef: harInputOriginalRef.value,
 finalOutputMappingId: harFinalOutputMappingId.value,
 regexPreset: harRegexPreset.value,
 inputValues: harInputValues.value || {}
 })
 harStats.value = data.stats
 message.success(`处理完成：匹配 ${data.stats.matched} 条，成功 ${data.stats.success} 条，失败 ${data.stats.failed} 条`)
 } catch (e) {
 message.error(e?.message || '处理失败')
 } finally {
 isProcessingHar.value = false
 }
}
</script>

<template>
 <div class="execute-panel">
 <!-- 头部 -->
 <div class="panel-header">
 <h3 class="panel-title">
 <n-icon :size="18" color="#18a058"><FlashOutline /></n-icon>
 执行面板
 </h3>
 <div class="panel-actions">
 <n-button size="small" tertiary @click="openHarModal">
 使用该配置处理HAR文件
 </n-button>
 </div>
 </div>

 <n-scrollbar class="panel-content">
 <!-- 无组件状态 -->
 <n-empty
 v-if="!hasComponents"
 description="请先在工作流配置区添加组件"
 class="empty-state"
 >
 <template #icon>
 <n-icon :size="48" color="#ccc">
 <DocumentTextOutline />
 </n-icon>
 </template>
 </n-empty>

 <template v-else>
 <!-- 输入映射表单区域 -->
 <div class="input-section" v-if="inputFields.length > 0">
 <div class="section-header">
 <n-text strong>输入参数</n-text>
 <n-text depth="3" class="input-count">
 {{ inputFields.length }} 个参数
 </n-text>
 </div>
 
 <n-form label-placement="top" size="small" class="input-form">
 <n-form-item
 v-for="field in inputFields"
 :key="field.key"
 :label="field.label"
 >
 <n-input
 v-model:value="inputValues[field.key]"
 type="textarea"
 :rows="3"
 :placeholder="field.placeholder"
 />
 </n-form-item>
 </n-form>
 </div>

 <!-- 纯文本表达式默认值显示 -->
 <div class="default-values-section" v-if="pureTextExpressionComponents.length > 0">
 <div class="section-header">
 <n-text strong>默认值</n-text>
 <n-text depth="3" class="input-count">
 {{ pureTextExpressionComponents.length }} 个固定值
 </n-text>
 </div>
 
 <div class="default-value-list">
 <div
 v-for="component in pureTextExpressionComponents"
 :key="component.id"
 class="default-value-item"
 >
 <n-text depth="2" class="default-value-label">
 {{ component.name }} 输入
 </n-text>
 <n-input
 :value="component.config.inputExpression"
 type="textarea"
 :rows="2"
 readonly
 disabled
 class="default-value-input"
 />
 </div>
 </div>
 </div>

 <!-- 无输入参数提示 -->
 <div class="no-input-hint" v-if="inputFields.length === 0 && pureTextExpressionComponents.length === 0">
 <n-icon :size="24" color="#f0a020"><AlertCircleOutline /></n-icon>
 <n-text depth="3">请先在工作流配置中添加输入映射</n-text>
 </div>

 <!-- 校验错误显示 -->
 <n-alert
 v-if="hasValidationErrors"
 type="warning"
 :show-icon="true"
 class="validation-alert"
 >
 <template #header>配置不完整</template>
 <ul class="error-list">
 <li v-for="(error, idx) in localValidationErrors" :key="idx">
 <strong>{{ error.componentName }}</strong>: {{ error.message }}
 </li>
 </ul>
 </n-alert>

 <!-- 执行按钮 -->
 <div class="execute-action">
 <n-button
 type="primary"
 size="large"
 block
 :loading="isExecuting"
 :disabled="isExecuteDisabled"
 @click="executeWorkflow"
 >
 <template #icon>
 <n-icon><PlayOutline /></n-icon>
 </template>
 {{ isExecuting ? '执行中...' : '执行工作流' }}
 </n-button>
 <n-text depth="3" class="shortcut-hint">
 快捷键: Ctrl + Enter
 </n-text>
 </div>

 <!-- 执行错误 -->
 <n-alert
 v-if="executionError"
 type="error"
 :show-icon="true"
 closable
 class="execution-error"
 @close="executionError = null"
 >
 {{ executionError }}
 </n-alert>

 <!-- 结果展示区域 -->
 <div class="results-section" v-if="Object.keys(executionResults).length > 0">
 <n-divider>
 <n-space align="center" :size="4">
 <n-icon :size="16"><CheckmarkCircleOutline /></n-icon>
 <span>执行结果</span>
 </n-space>
 </n-divider>

 <div class="result-list">
 <n-card
 v-for="output in generateOutputs"
 :key="output.id"
 size="small"
 class="result-card"
 >
 <template #header>
 <div class="result-header">
 <n-text strong>{{ output.name }}</n-text>
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 size="tiny"
 @click="copyResult(getOutputResult(output))"
 >
 <template #icon>
 <n-icon><CopyOutline /></n-icon>
 </template>
 </n-button>
 </template>
 复制结果
 </n-tooltip>
 </div>
 </template>
 <n-input
 :value="getOutputResult(output)"
 type="textarea"
 :rows="4"
 readonly
 placeholder="暂无结果"
 />
 <n-text depth="3" class="result-ref">
 输出标识: {{ output.componentRef }}
 </n-text>
 </n-card>
 </div>
 </div>

 <!-- 执行日志（可折叠） -->
 <div class="log-section" v-if="executionLog.length > 0">
 <n-divider>
 <n-space
 align="center"
 :size="4"
 class="log-toggle"
 @click="showLog = !showLog"
 >
 <n-icon :size="16"><TimeOutline /></n-icon>
 <span>执行日志 ({{ executionLog.length }})</span>
 <n-icon :size="14">
 <ChevronUpOutline v-if="showLog" />
 <ChevronDownOutline v-else />
 </n-icon>
 </n-space>
 </n-divider>

 <div v-show="showLog" class="log-content">
 <n-timeline>
 <n-timeline-item
 v-for="(log, index) in executionLog"
 :key="index"
 :type="getLogType(log.type)"
 :title="log.message"
 :time="formatTime(log.timestamp)"
 >
 <template v-if="log.input !== undefined || log.output !== undefined">
 <div class="log-detail">
 <div v-if="log.input !== undefined" class="log-io">
 <n-text depth="3">输入:</n-text>
 <n-tooltip :disabled="log.input.length <= 100">
 <template #trigger>
 <code class="io-value">{{ truncateText(log.input) }}</code>
 </template>
 {{ log.input }}
 </n-tooltip>
 </div>
 <div v-if="log.output !== undefined" class="log-io">
 <n-text depth="3">输出:</n-text>
 <n-tooltip :disabled="log.output.length <= 100">
 <template #trigger>
 <code class="io-value">{{ truncateText(log.output) }}</code>
 </template>
 {{ log.output }}
 </n-tooltip>
 </div>
 </div>
 </template>
 </n-timeline-item>
 </n-timeline>
 </div>
 </div>
 </template>
 </n-scrollbar>

 <!-- HAR 处理弹窗 -->
 <n-modal v-model:show="harModalVisible" preset="card" title="使用当前配置处理 HAR 文件" style="width: 720px">
 <div class="har-form">
 <n-form label-placement="left" label-width="120">
 <n-form-item label="HAR 文件">
 <n-space vertical style="width: 100%">
 <n-input
 v-model:value="harInputPath"
 readonly
 placeholder="请选择本地 HAR/JSON 文件"
 />
 <n-button secondary :disabled="isProcessingHar" @click="selectHarInputFile">
 <template #icon>
 <n-icon><DocumentTextOutline /></n-icon>
 </template>
 选择文件
 </n-button>
 </n-space>
 </n-form-item>
 <n-form-item label="保存到">
 <n-space vertical style="width: 100%">
 <n-input
 v-model:value="harOutputPath"
 readonly
 placeholder="请选择处理后 HAR 的保存位置"
 />
 <n-button secondary :disabled="isProcessingHar" @click="selectHarOutputFile">
 <template #icon>
 <n-icon><DocumentTextOutline /></n-icon>
 </template>
 选择保存位置
 </n-button>
 </n-space>
 </n-form-item>
 <n-form-item label="匹配预设">
 <n-select v-model:value="harRegexPreset" :options="regexOptions" :disabled="isProcessingHar" />
 </n-form-item>
 <n-form-item label="输入原文">
 <n-select v-model:value="harInputOriginalRef" :options="inputMappingOptions" :disabled="isProcessingHar" />
 </n-form-item>
 <n-form-item label="最终输出">
 <n-select v-model:value="harFinalOutputMappingId" :options="outputMappingOptions" :disabled="isProcessingHar" />
 </n-form-item>

 <n-divider>输入映射值</n-divider>
 <div class="har-inputs">
 <n-form-item v-for="m in inputMappings" :key="m.inputRef" :label="m.name || m.inputRef">
 <n-input
 v-model:value="harInputValues[m.inputRef]"
 type="textarea"
 :rows="2"
 :disabled="isProcessingHar || m.inputRef === harInputOriginalRef"
 :placeholder="m.inputRef === harInputOriginalRef ? '该项由匹配文本填充' : (m.defaultValue || '请输入')"
 />
 </n-form-item>
 </div>

 <n-alert v-if="harStats" type="info" :show-icon="true" class="har-stats">
 匹配 {{ harStats.matched }} 条，成功 {{ harStats.success }} 条，失败 {{ harStats.failed }} 条
 </n-alert>

 <div style="text-align:right; margin-top: 8px;">
 <n-button
 type="primary"
 :loading="isProcessingHar"
 :disabled="isProcessingHar"
 @click="submitHarProcessing"
 >
 {{ isProcessingHar ? '处理中...' : '开始处理并保存' }}
 </n-button>
 </div>
 </n-form>
 </div>
 </n-modal>
 </div>
</template>


<style scoped>
.execute-panel {
 display: flex;
 flex-direction: column;
 height: 100%;
 overflow: hidden;
 background-color: var(--n-card-color, #fff);
}

.panel-header {
 display: flex;
 align-items: center;
 justify-content: space-between;
 padding: 16px;
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
 flex-shrink: 0;
}

.panel-title {
 display: flex;
 align-items: center;
 gap: 8px;
 margin: 0;
 font-size: 16px;
 font-weight: 600;
 color: var(--n-text-color-1, #333);
}

.panel-content {
 flex: 1;
 overflow: hidden;
}

.empty-state {
 padding: 60px 20px;
}

/* 输入区域 */
.input-section {
 padding: 16px;
}

.section-header {
 display: flex;
 align-items: center;
 justify-content: space-between;
 margin-bottom: 12px;
}

.input-count {
 font-size: 12px;
}

.input-form {
 margin-top: 8px;
}

.no-input-hint {
 display: flex;
 align-items: center;
 justify-content: center;
 gap: 8px;
 padding: 16px;
 margin: 16px;
 background-color: var(--n-body-color, #f5f5f5);
 border-radius: 8px;
}

/* 默认值区域 */
.default-values-section {
 padding: 16px;
 padding-top: 0;
}

.default-value-list {
 display: flex;
 flex-direction: column;
 gap: 12px;
}

.default-value-item {
 display: flex;
 flex-direction: column;
 gap: 4px;
}

.default-value-label {
 font-size: 13px;
 font-weight: 500;
}

.default-value-input {
 opacity: 0.7;
}

/* 校验错误 */
.validation-alert {
 margin: 0 16px 16px;
}

.error-list {
 margin: 4px 0 0 0;
 padding-left: 16px;
 font-size: 12px;
}

.error-list li {
 margin-bottom: 2px;
}

.error-list li strong {
 color: var(--n-warning-color, #f0a020);
}

/* 执行按钮 */
.execute-action {
 padding: 0 16px 16px;
 text-align: center;
}

.shortcut-hint {
 display: block;
 margin-top: 8px;
 font-size: 12px;
}

/* 执行错误 */
.execution-error {
 margin: 0 16px 16px;
}

/* 结果区域 */
.results-section {
 padding: 0 16px;
}

.result-list {
 display: flex;
 flex-direction: column;
 gap: 12px;
}

.result-card {
 background-color: var(--n-body-color, #fafafa);
}

.result-header {
 display: flex;
 align-items: center;
 justify-content: space-between;
}

.result-ref {
 display: block;
 margin-top: 8px;
 font-size: 11px;
}

.panel-actions {
 display: flex;
 align-items: center;
}

.har-form {
 padding-top: 8px;
}

.har-stats {
 margin-top: 8px;
}

/* 日志区域 */
.log-section {
 padding: 0 16px 16px;
}

.log-toggle {
 cursor: pointer;
 user-select: none;
}

.log-toggle:hover {
 color: var(--n-primary-color, #18a058);
}

.log-content {
 padding: 12px;
 background-color: var(--n-body-color, #fafafa);
 border-radius: 8px;
 margin-top: 8px;
}

.log-detail {
 margin-top: 8px;
}

.log-io {
 display: flex;
 align-items: flex-start;
 gap: 8px;
 padding: 6px 8px;
 background-color: var(--n-card-color, #fff);
 border-radius: 4px;
 margin-bottom: 4px;
 font-size: 12px;
}

.io-value {
 font-family: 'Consolas', 'Monaco', monospace;
 word-break: break-all;
 color: var(--n-text-color-2, #666);
}

/* 滚动条样式 */
:deep(.n-scrollbar-content) {
 min-height: 100%;
}

/* 表单样式调整 */
:deep(.n-form-item) {
 margin-bottom: 12px;
}

:deep(.n-form-item:last-child) {
 margin-bottom: 0;
}

/* 时间线样式调整 */
:deep(.n-timeline-item-content__title) {
 font-size: 13px;
}

:deep(.n-timeline-item-content__meta) {
 font-size: 11px;
}
</style>
