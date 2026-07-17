<script setup>
import { ref, computed, watch } from 'vue'
import {
 NCard,
 NButton,
 NIcon,
 NSpace,
 NText,
 NBadge,
 NTooltip,
 NCollapse,
 NCollapseItem,
 NForm,
 NFormItem,
 NInput,
 NSelect,
 NRadioGroup,
 NRadio,
 NAlert,
 NTag
} from 'naive-ui'
import {
 TrashOutline,
 ChevronDownOutline,
 ChevronUpOutline,
 ReorderFourOutline,
 WarningOutline,
 DocumentTextOutline,
 CodeSlashOutline,
 LockClosedOutline,
 ShieldOutline,
 KeyOutline,
 ShieldCheckmarkOutline,
 FingerPrintOutline,
 CodeOutline,
 LinkOutline,
 TextOutline,
 CalculatorOutline
} from '@vicons/ionicons5'
import { useConfigStore } from '@/store/index.js'
// 导入参数引用选择器组件
import ParamRefSelector from './ParamRefSelector.vue'

// Props
const props = defineProps({
 component: {
 type: Object,
 required: true
 },
 index: {
 type: Number,
 required: true
 },
 isSelected: {
 type: Boolean,
 default: false
 },
 isHighlighted: {
 type: Boolean,
 default: false
 },
 availableInputSources: {
 type: Array,
 default: () => []
 }
})

// Emits
const emit = defineEmits(['select', 'hover', 'leave'])

// Store
const configStore = useConfigStore()

// 展开/折叠状态
const isExpanded = ref(false)

// 表达式输入框引用
const expressionInputRef = ref(null)

// 本地配置副本
const localConfig = ref({ ...props.component.config })

// 标记是否正在从外部同步
let isSyncingFromProps = false

// 监听组件配置变化，更新本地配置
watch(() => props.component.config, (newConfig) => {
 isSyncingFromProps = true
 localConfig.value = { ...newConfig }
 setTimeout(() => {
 isSyncingFromProps = false
 }, 0)
}, { deep: true })

// 自动保存配置
watch(localConfig, (newConfig) => {
 if (isSyncingFromProps) return
 configStore.updateComponentConfig(props.component.id, newConfig)
}, { deep: true })

// 图标映射
const iconMap = {
 DocumentTextOutline,
 CodeSlashOutline,
 LockClosedOutline,
 ShieldOutline,
 KeyOutline,
 ShieldCheckmarkOutline,
 FingerPrintOutline,
 CodeOutline,
 LinkOutline,
 TextOutline,
 CalculatorOutline
}

// 获取图标组件
const getIcon = (iconName) => {
 return iconMap[iconName] || CodeOutline
}

// 校验错误
const validationErrors = computed(() => {
 return configStore.validateComponent({
 ...props.component,
 config: localConfig.value
 })
})

// 是否有错误
const hasErrors = computed(() => validationErrors.value.length > 0)

// 检查特定字段是否有错误
const hasFieldError = (fieldName) => {
 return validationErrors.value.some(error => error.field === fieldName)
}

// 获取特定字段的错误信息
const getFieldError = (fieldName) => {
 const error = validationErrors.value.find(e => e.field === fieldName)
 return error ? error.message : ''
}

// 获取特定字段的所有错误信息
const getFieldErrors = (fieldName) => {
 return validationErrors.value
 .filter(e => e.field === fieldName)
 .map(e => e.message)
}

// 表达式校验错误列表
const expressionErrors = computed(() => {
 return getFieldErrors('inputExpression')
})

const radixBaseOptions = Array.from({ length: 35 }, (_, index) => {
 const value = index + 2
 const commonLabels = {
 2: '二进制 (2)',
 8: '八进制 (8)',
 10: '十进制 (10)',
 16: '十六进制 (16)',
 36: '三十六进制 (36)'
 }

 return {
 label: commonLabels[value] || `${value} 进制`,
 value
 }
})

// 输入源选项
const inputSourceOptions = computed(() => {
 return props.availableInputSources.map(source => ({
 label: `${source.name} (${source.outputRef})`,
 value: source.outputRef
 }))
})

// 输入映射选项
const inputMappingOptions = computed(() => {
 return configStore.getAvailableInputMappings()
})

// 合并的输入选项（输入映射 + 组件输出）
const combinedInputOptions = computed(() => {
 const options = []
 
 // 添加输入映射选项
 const mappings = configStore.inputMappings
 if (mappings.length > 0) {
 options.push({
 type: 'group',
 label: '输入映射',
 key: 'input-mappings',
 children: mappings.map(m => ({
 label: m.name || m.inputRef,
 value: m.inputRef
 }))
 })
 }
 
 // 添加组件输出选项
 if (props.availableInputSources.length > 0) {
 options.push({
 type: 'group',
 label: '组件输出',
 key: 'component-outputs',
 children: props.availableInputSources.map(source => ({
 label: `${source.name} (${source.outputRef})`,
 value: source.outputRef
 }))
 })
 }
 
 return options
})

// 切换展开状态
const toggleExpand = () => {
 isExpanded.value = !isExpanded.value
 emit('select')
}

// 处理鼠标悬停 - Requirements: 5.5
const handleMouseEnter = () => {
 emit('hover')
}

// 处理鼠标离开
const handleMouseLeave = () => {
 emit('leave')
}

// 删除组件
const handleRemove = () => {
 configStore.removeComponent(props.component.id)
}

// 插入变量到表达式输入框
const insertVariable = (outputRef) => {
 const variableText = '${' + outputRef + '}'
 const currentValue = localConfig.value.inputExpression || ''
 
 // 尝试获取输入框的DOM元素并在光标位置插入
 const inputEl = expressionInputRef.value?.$el?.querySelector('textarea')
 if (inputEl) {
 const start = inputEl.selectionStart
 const end = inputEl.selectionEnd
 const newValue = currentValue.substring(0, start) + variableText + currentValue.substring(end)
 localConfig.value.inputExpression = newValue
 
 // 设置光标位置到插入内容之后
 setTimeout(() => {
 inputEl.focus()
 const newCursorPos = start + variableText.length
 inputEl.setSelectionRange(newCursorPos, newCursorPos)
 }, 0)
 } else {
 // 如果无法获取光标位置，直接追加到末尾
 localConfig.value.inputExpression = currentValue + variableText
 }
}
</script>

<template>
 <div 
 class="component-card" 
 :class="{ 'is-expanded': isExpanded, 'has-errors': hasErrors, 'is-selected': isSelected, 'is-highlighted': isHighlighted }"
 @mouseenter="handleMouseEnter"
 @mouseleave="handleMouseLeave"
 >
 <!-- 卡片头部 -->
 <div class="card-header" @click="toggleExpand">
 <!-- 拖拽手柄 -->
 <div class="drag-handle">
 <n-icon :size="18" color="#999">
 <ReorderFourOutline />
 </n-icon>
 </div>

 <!-- 组件图标和名称 -->
 <div class="component-info">
 <n-icon :size="20" :component="getIcon(component.icon)" color="#18a058" />
 <span class="component-name">{{ component.name }}</span>
 <n-text depth="3" class="component-index">#{{ index + 1 }}</n-text>
 </div>

 <!-- 错误徽章 -->
 <n-tooltip v-if="hasErrors" trigger="hover">
 <template #trigger>
 <n-badge :value="validationErrors.length" type="warning" :max="9">
 <n-icon :size="18" color="#f0a020">
 <WarningOutline />
 </n-icon>
 </n-badge>
 </template>
 <div class="error-tooltip">
 <div v-for="(error, idx) in validationErrors" :key="idx" class="error-item">
 {{ error.message }}
 </div>
 </div>
 </n-tooltip>

 <!-- 展开/折叠图标 -->
 <n-icon :size="18" class="expand-icon">
 <ChevronUpOutline v-if="isExpanded" />
 <ChevronDownOutline v-else />
 </n-icon>

 <!-- 删除按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 size="tiny"
 type="error"
 @click.stop="handleRemove"
 >
 <template #icon>
 <n-icon><TrashOutline /></n-icon>
 </template>
 </n-button>
 </template>
 删除组件
 </n-tooltip>
 </div>

 <!-- 配置面板 -->
 <div v-show="isExpanded" class="card-content">
 <!-- 校验警告 -->
 <n-alert
 v-if="hasErrors"
 type="warning"
 :show-icon="true"
 class="validation-alert"
 >
 <template #header>配置不完整</template>
 <ul class="error-list">
 <li v-for="(error, idx) in validationErrors" :key="idx">
 {{ error.message }}
 </li>
 </ul>
 </n-alert>

 <n-form :model="localConfig" label-placement="left" label-width="100" size="small">
 <!-- 输入来源配置 -->
 <n-form-item label="输入来源" :validation-status="hasFieldError('inputSourceType') ? 'error' : undefined">
 <n-radio-group v-model:value="localConfig.inputSourceType">
 <n-space>
 <n-radio value="reference">引用输入/组件</n-radio>
 <n-radio value="expression">表达式输入</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>

 <!-- 引用选择（输入映射 + 组件输出） -->
 <n-form-item
 v-if="localConfig.inputSourceType === 'reference' || localConfig.inputSourceType === 'inputMapping' || localConfig.inputSourceType === 'component'"
 label="选择来源"
 :validation-status="hasFieldError('inputMappingRef') || hasFieldError('inputComponentRef') ? 'error' : undefined"
 :feedback="getFieldError('inputMappingRef') || getFieldError('inputComponentRef')"
 >
 <n-select
 v-model:value="localConfig.inputMappingRef"
 :options="combinedInputOptions"
 placeholder="选择输入映射或组件输出"
 :disabled="combinedInputOptions.length === 0"
 />
 <template v-if="combinedInputOptions.length === 0">
 <n-text type="warning" style="font-size: 12px; margin-top: 4px;">
 请先添加输入映射或上游组件
 </n-text>
 </template>
 </n-form-item>

 <!-- 表达式输入 -->
 <n-form-item
 v-if="localConfig.inputSourceType === 'expression'"
 label="表达式"
 :validation-status="hasFieldError('inputExpression') ? 'error' : undefined"
 >
 <n-input
 ref="expressionInputRef"
 v-model:value="localConfig.inputExpression"
 type="textarea"
 :rows="3"
 placeholder="输入表达式，使用 ${ref} 引用输入映射或组件输出&#10;例如：固定前缀${input-xxx} 或 ${base64-output-xxx}"
 />
 </n-form-item>

 <!-- 表达式校验错误显示 -->
 <n-form-item
 v-if="localConfig.inputSourceType === 'expression' && expressionErrors.length > 0"
 :show-label="false"
 >
 <div class="expression-errors">
 <n-alert type="error" :show-icon="true" size="small">
 <template #header>表达式校验错误</template>
 <ul class="expression-error-list">
 <li v-for="(error, idx) in expressionErrors" :key="idx">
 {{ error }}
 </li>
 </ul>
 </n-alert>
 </div>
 </n-form-item>

 <!-- 可用变量列表 -->
 <n-form-item
 v-if="localConfig.inputSourceType === 'expression'"
 label="可用变量"
 >
 <div class="available-variables">
 <!-- 输入映射变量 -->
 <template v-if="inputMappingOptions.length > 0">
 <n-tag
 v-for="mapping in inputMappingOptions"
 :key="mapping.value"
 type="success"
 size="small"
 class="variable-tag"
 @click="insertVariable(mapping.value)"
 >
 {{ mapping.label }} ({{ mapping.value }})
 </n-tag>
 </template>
 <!-- 组件输出变量 -->
 <template v-if="availableInputSources.length > 0">
 <n-tag
 v-for="source in availableInputSources"
 :key="source.id"
 type="info"
 size="small"
 class="variable-tag"
 @click="insertVariable(source.outputRef)"
 >
 {{ source.name }} ({{ source.outputRef }})
 </n-tag>
 </template>
 <n-text v-if="inputMappingOptions.length === 0 && availableInputSources.length === 0" depth="3" style="font-size: 12px;">
 没有可用的变量，请先添加输入映射或上游组件
 </n-text>
 </div>
 </n-form-item>

 <!-- 输出参数标识 -->
 <n-form-item label="输出标识">
 <n-input
 :value="component.outputRef"
 placeholder="输出参数标识符"
 @update:value="(val) => configStore.updateComponentOutputRef(component.id, val)"
 />
 </n-form-item>

 <!-- Base64 配置 -->
 <template v-if="component.type === 'base64'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encode">编码</n-radio>
 <n-radio value="decode">解码</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出格式">
 <n-radio-group v-model:value="localConfig.outputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="utf-8">UTF-8</n-radio>
 <n-radio value="gbk">GBK</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item v-if="localConfig.outputFormat === 'hex'" label="HEX大小写">
 <n-radio-group v-model:value="localConfig.hexCase">
 <n-space>
 <n-radio value="uppercase">大写</n-radio>
 <n-radio value="lowercase">小写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
</template>

 <!-- Hex 配置 -->
 <template v-else-if="component.type === 'hex'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encode">编码</n-radio>
 <n-radio value="decode">解码</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- 进制转换配置 -->
 <template v-else-if="component.type === 'radix'">
 <n-form-item label="源进制">
 <n-select
 v-model:value="localConfig.inputBase"
 :options="radixBaseOptions"
 placeholder="选择源进制"
 />
 </n-form-item>
 <n-form-item label="目标进制">
 <n-select
 v-model:value="localConfig.outputBase"
 :options="radixBaseOptions"
 placeholder="选择目标进制"
 />
 </n-form-item>
 <n-form-item label="字母大小写">
 <n-radio-group v-model:value="localConfig.hexCase">
 <n-space>
 <n-radio value="uppercase">大写</n-radio>
 <n-radio value="lowercase">小写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- MD5 配置 -->
 <template v-else-if="component.type === 'md5'">
 <n-form-item label="结果格式">
 <n-radio-group v-model:value="localConfig.resultFormat">
 <n-space>
 <n-radio value="lowercase">小写</n-radio>
 <n-radio value="uppercase">大写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出位数">
 <n-radio-group v-model:value="localConfig.outputLength">
 <n-space>
 <n-radio :value="16">16位</n-radio>
 <n-radio :value="32">32位</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- SHA 配置 -->
 <template v-else-if="component.type === 'sha'">
 <n-form-item label="算法类型">
 <n-radio-group v-model:value="localConfig.shaType">
 <n-space>
 <n-radio value="SHA1">SHA1</n-radio>
 <n-radio value="SHA256">SHA256</n-radio>
 <n-radio value="SHA384">SHA384</n-radio>
 <n-radio value="SHA512">SHA512</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="结果格式">
 <n-radio-group v-model:value="localConfig.resultFormat">
 <n-space>
 <n-radio value="lowercase">小写</n-radio>
 <n-radio value="uppercase">大写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- HmacMD5 配置 -->
 <template v-else-if="component.type === 'hmacmd5'">
 <n-form-item
 label="密钥来源"
 :validation-status="hasFieldError('keyRef') ? 'error' : undefined"
 :feedback="getFieldError('keyRef')"
 >
 <ParamRefSelector
 v-model="localConfig.keyRef"
 :component-id="component.id"
 placeholder="选择密钥来源"
 />
 </n-form-item>
 <n-form-item label="结果格式">
 <n-radio-group v-model:value="localConfig.resultFormat">
 <n-space>
 <n-radio value="lowercase">小写</n-radio>
 <n-radio value="uppercase">大写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出位数">
 <n-radio-group v-model:value="localConfig.outputLength">
 <n-space>
 <n-radio :value="16">16位</n-radio>
 <n-radio :value="32">32位</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- HmacSHA 配置 -->
 <template v-else-if="component.type === 'hmacsha'">
 <n-form-item
 label="密钥来源"
 :validation-status="hasFieldError('keyRef') ? 'error' : undefined"
 :feedback="getFieldError('keyRef')"
 >
 <ParamRefSelector
 v-model="localConfig.keyRef"
 :component-id="component.id"
 placeholder="选择密钥来源"
 />
 </n-form-item>
 <n-form-item label="算法类型">
 <n-radio-group v-model:value="localConfig.hmacShaType">
 <n-space>
 <n-radio value="HmacSHA1">HmacSHA1</n-radio>
 <n-radio value="HmacSHA256">HmacSHA256</n-radio>
 <n-radio value="HmacSHA384">HmacSHA384</n-radio>
 <n-radio value="HmacSHA512">HmacSHA512</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="结果格式">
 <n-radio-group v-model:value="localConfig.resultFormat">
 <n-space>
 <n-radio value="lowercase">小写</n-radio>
 <n-radio value="uppercase">大写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- SM3 配置 -->
 <template v-else-if="component.type === 'sm3'">
 <n-form-item label="结果格式">
 <n-radio-group v-model:value="localConfig.resultFormat">
 <n-space>
 <n-radio value="lowercase">小写</n-radio>
 <n-radio value="uppercase">大写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- AES 配置 -->
 <template v-else-if="component.type === 'aes'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encrypt">加密</n-radio>
 <n-radio value="decrypt">解密</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="加密模式">
 <n-select
 v-model:value="localConfig.mode"
 :options="[
 { label: 'ECB (电子密码本)', value: 'ECB' },
 { label: 'CBC (密码块链)', value: 'CBC' },
 { label: 'CTR (计算器模式)', value: 'CTR' },
 { label: 'OFB (输出反馈)', value: 'OFB' },
 { label: 'CFB (加密反馈)', value: 'CFB' },
 { label: 'CTS (密文窃取)', value: 'CTS' },
 { label: 'GCM (Galois/Counter)', value: 'GCM' }
 ]"
 placeholder="选择加密模式"
 />
 </n-form-item>
 <n-form-item label="填充模式">
 <n-radio-group v-model:value="localConfig.padding">
 <n-space>
 <n-radio value="PKCS5Padding">PKCS5Padding</n-radio>
 <n-radio value="NoPadding">NoPadding</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item
 label="密钥来源"
 :validation-status="hasFieldError('keyRef') ? 'error' : undefined"
 :feedback="getFieldError('keyRef')"
 >
 <ParamRefSelector
 v-model="localConfig.keyRef"
 :component-id="component.id"
 placeholder="选择密钥来源"
 />
 </n-form-item>
 <n-form-item label="密钥格式">
 <n-radio-group v-model:value="localConfig.keyFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item
 v-if="localConfig.mode !== 'ECB'"
 label="IV向量来源"
 :validation-status="hasFieldError('ivRef') ? 'error' : undefined"
 :feedback="getFieldError('ivRef')"
 >
 <ParamRefSelector
 v-model="localConfig.ivRef"
 :component-id="component.id"
 placeholder="选择IV向量来源"
 />
 </n-form-item>
 <n-form-item v-if="localConfig.mode !== 'ECB'" label="IV格式">
 <n-radio-group v-model:value="localConfig.ivFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输入格式">
 <n-radio-group v-model:value="localConfig.inputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出格式">
 <n-radio-group v-model:value="localConfig.outputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="utf-8">UTF-8</n-radio>
 <n-radio value="gbk">GBK</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item v-if="localConfig.outputFormat === 'hex'" label="HEX大小写">
 <n-radio-group v-model:value="localConfig.hexCase">
 <n-space>
 <n-radio value="uppercase">大写</n-radio>
 <n-radio value="lowercase">小写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- Blowfish 配置 -->
 <template v-else-if="component.type === 'blowfish'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encrypt">加密</n-radio>
 <n-radio value="decrypt">解密</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="加密模式">
 <n-radio-group v-model:value="localConfig.mode">
 <n-space>
 <n-radio value="ECB">ECB</n-radio>
 <n-radio value="CBC">CBC</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="填充模式">
 <n-radio-group v-model:value="localConfig.padding">
 <n-space>
 <n-radio value="PKCS5Padding">PKCS5Padding</n-radio>
 <n-radio value="None">None</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item
 label="密钥来源"
 :validation-status="hasFieldError('keyRef') ? 'error' : undefined"
 :feedback="getFieldError('keyRef')"
 >
 <ParamRefSelector
 v-model="localConfig.keyRef"
 :component-id="component.id"
 placeholder="选择密钥来源"
 />
 </n-form-item>
 <n-form-item label="密钥格式">
 <n-radio-group v-model:value="localConfig.keyFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item
 v-if="localConfig.mode === 'CBC'"
 label="IV向量来源"
 :validation-status="hasFieldError('ivRef') ? 'error' : undefined"
 :feedback="getFieldError('ivRef')"
 >
 <ParamRefSelector
 v-model="localConfig.ivRef"
 :component-id="component.id"
 placeholder="选择IV向量来源"
 />
 </n-form-item>
 <n-form-item v-if="localConfig.mode === 'CBC'" label="IV格式">
 <n-radio-group v-model:value="localConfig.ivFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输入格式">
 <n-radio-group v-model:value="localConfig.inputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出格式">
 <n-radio-group v-model:value="localConfig.outputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="utf-8">UTF-8</n-radio>
 <n-radio value="gbk">GBK</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item v-if="localConfig.outputFormat === 'hex'" label="HEX大小写">
 <n-radio-group v-model:value="localConfig.hexCase">
 <n-space>
 <n-radio value="uppercase">大写</n-radio>
 <n-radio value="lowercase">小写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- SM4 配置 -->
 <template v-else-if="component.type === 'sm4'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encrypt">加密</n-radio>
 <n-radio value="decrypt">解密</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="加密模式">
 <n-radio-group v-model:value="localConfig.mode">
 <n-space>
 <n-radio value="ECB">ECB</n-radio>
 <n-radio value="CBC">CBC</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="填充模式">
 <n-radio-group v-model:value="localConfig.padding">
 <n-space>
 <n-radio value="pkcs7">PKCS7 (PKCS5)</n-radio>
 <n-radio value="zero">Zero Padding</n-radio>
 <n-radio value="none">No Padding</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item
 label="密钥来源"
 :validation-status="hasFieldError('keyRef') ? 'error' : undefined"
 :feedback="getFieldError('keyRef')"
 >
 <ParamRefSelector
 v-model="localConfig.keyRef"
 :component-id="component.id"
 placeholder="选择密钥来源"
 />
 </n-form-item>
 <n-form-item label="密钥格式">
 <n-radio-group v-model:value="localConfig.keyFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item
 v-if="localConfig.mode === 'CBC'"
 label="IV向量来源"
 :validation-status="hasFieldError('ivRef') ? 'error' : undefined"
 :feedback="getFieldError('ivRef')"
 >
 <ParamRefSelector
 v-model="localConfig.ivRef"
 :component-id="component.id"
 placeholder="选择IV向量来源"
 />
 </n-form-item>
 <n-form-item v-if="localConfig.mode === 'CBC'" label="IV格式">
 <n-radio-group v-model:value="localConfig.ivFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输入格式">
 <n-radio-group v-model:value="localConfig.inputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出格式">
 <n-radio-group v-model:value="localConfig.outputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="utf-8">UTF-8</n-radio>
 <n-radio value="gbk">GBK</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item v-if="localConfig.outputFormat === 'hex'" label="HEX大小写">
 <n-radio-group v-model:value="localConfig.hexCase">
 <n-space>
 <n-radio value="uppercase">大写</n-radio>
 <n-radio value="lowercase">小写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- SM2 配置 -->
 <template v-else-if="component.type === 'sm2'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encrypt">加密</n-radio>
 <n-radio value="decrypt">解密</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="数据顺序">
 <n-radio-group v-model:value="localConfig.mode">
 <n-space>
 <n-radio value="C1C3C2">C1C3C2</n-radio>
 <n-radio value="C1C2C3">C1C2C3</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item
 v-if="localConfig.operation === 'encrypt'"
 label="公钥来源"
 :validation-status="hasFieldError('publicKeyRef') ? 'error' : undefined"
 :feedback="getFieldError('publicKeyRef')"
 >
 <ParamRefSelector
 v-model="localConfig.publicKeyRef"
 :component-id="component.id"
 placeholder="选择公钥来源"
 />
 </n-form-item>
 <n-form-item
 v-if="localConfig.operation === 'decrypt'"
 label="私钥来源"
 :validation-status="hasFieldError('privateKeyRef') ? 'error' : undefined"
 :feedback="getFieldError('privateKeyRef')"
 >
 <ParamRefSelector
 v-model="localConfig.privateKeyRef"
 :component-id="component.id"
 placeholder="选择私钥来源"
 />
 </n-form-item>
 <n-form-item label="输入格式">
 <n-radio-group v-model:value="localConfig.inputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出格式">
 <n-radio-group v-model:value="localConfig.outputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="utf-8">UTF-8</n-radio>
 <n-radio value="gbk">GBK</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item v-if="localConfig.outputFormat === 'hex'" label="HEX大小写">
 <n-radio-group v-model:value="localConfig.hexCase">
 <n-space>
 <n-radio value="uppercase">大写</n-radio>
 <n-radio value="lowercase">小写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- RSA 配置 -->
 <template v-else-if="component.type === 'rsa'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encrypt">加密</n-radio>
 <n-radio value="decrypt">解密</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="填充模式">
 <n-radio-group v-model:value="localConfig.padding">
 <n-space>
 <n-radio value="PKCS1">PKCS1</n-radio>
 <n-radio value="OAEP">OAEP</n-radio>
 <n-radio value="PKCS1签名">PKCS1签名</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item
 v-if="localConfig.operation === 'encrypt'"
 label="公钥来源"
 :validation-status="hasFieldError('publicKeyRef') ? 'error' : undefined"
 :feedback="getFieldError('publicKeyRef')"
 >
 <ParamRefSelector
 v-model="localConfig.publicKeyRef"
 :component-id="component.id"
 placeholder="选择公钥来源"
 />
 </n-form-item>
 <n-form-item
 v-if="localConfig.operation === 'decrypt'"
 label="私钥来源"
 :validation-status="hasFieldError('privateKeyRef') ? 'error' : undefined"
 :feedback="getFieldError('privateKeyRef')"
 >
 <ParamRefSelector
 v-model="localConfig.privateKeyRef"
 :component-id="component.id"
 placeholder="选择私钥来源"
 />
 </n-form-item>
 <n-form-item label="输入格式">
 <n-radio-group v-model:value="localConfig.inputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出格式">
 <n-radio-group v-model:value="localConfig.outputFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="base64">BASE64</n-radio>
 <n-radio value="utf-8">UTF-8</n-radio>
 <n-radio value="gbk">GBK</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item v-if="localConfig.outputFormat === 'hex'" label="HEX大小写">
 <n-radio-group v-model:value="localConfig.hexCase">
 <n-space>
 <n-radio value="uppercase">大写</n-radio>
 <n-radio value="lowercase">小写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- URL 编码配置 -->
 <template v-else-if="component.type === 'url'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encode">编码</n-radio>
 <n-radio value="decode">解码</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="字符集">
 <n-radio-group v-model:value="localConfig.charset">
 <n-space>
 <n-radio value="UTF-8">UTF-8</n-radio>
 <n-radio value="GBK">GBK</n-radio>
 <n-radio value="ISO-8859-1">ISO-8859-1</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>

 <!-- Unicode 编码配置 -->
 <template v-else-if="component.type === 'unicode'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encode">编码</n-radio>
 <n-radio value="decode">解码</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="编码格式">
 <n-radio-group v-model:value="localConfig.format">
 <n-space>
 <n-radio value="standard">标准格式 (\u0041)</n-radio>
 <n-radio value="html">HTML实体 (&amp;#65;)</n-radio>
 <n-radio value="css">CSS格式 (\0041)</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 </template>
 </n-form>
 </div>
 </div>
</template>


<style scoped>
.component-card {
 background-color: var(--n-card-color, #fff);
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 overflow: hidden;
 transition: all 0.2s ease;
}

.component-card:hover {
 box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.component-card.has-errors {
 border-color: #f0a020;
}

.component-card.is-selected {
 border-color: #18a058;
 box-shadow: 0 0 0 2px rgba(24, 160, 88, 0.2);
}

.component-card.is-highlighted {
 border-color: #2080f0;
 box-shadow: 0 0 0 2px rgba(32, 128, 240, 0.2);
 background-color: rgba(32, 128, 240, 0.05);
}

.component-card.is-expanded {
 box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
}

/* 卡片头部 */
.card-header {
 display: flex;
 align-items: center;
 gap: 8px;
 padding: 12px;
 cursor: pointer;
 transition: background-color 0.2s ease;
}

.card-header:hover {
 background-color: var(--n-hover-color, #f5f5f5);
}

/* 拖拽手柄 */
.drag-handle {
 cursor: grab;
 padding: 4px;
 border-radius: 4px;
 transition: background-color 0.2s ease;
}

.drag-handle:hover {
 background-color: var(--n-hover-color, #e8e8e8);
}

.drag-handle:active {
 cursor: grabbing;
}

/* 组件信息 */
.component-info {
 display: flex;
 align-items: center;
 gap: 8px;
 flex: 1;
 min-width: 0;
}

.component-name {
 font-weight: 500;
 font-size: 14px;
 color: var(--n-text-color-1, #333);
}

.component-index {
 font-size: 12px;
 padding: 2px 6px;
 background-color: var(--n-tag-color, #f0f0f0);
 border-radius: 4px;
}

/* 展开图标 */
.expand-icon {
 color: var(--n-text-color-3, #999);
 transition: transform 0.2s ease;
}

/* 错误提示 */
.error-tooltip {
 max-width: 250px;
}

.error-item {
 padding: 2px 0;
 font-size: 12px;
}

/* 配置面板 */
.card-content {
 padding: 16px;
 border-top: 1px solid var(--n-border-color, #e0e0e6);
 background-color: var(--n-body-color, #fafafa);
}

.validation-alert {
 margin-bottom: 16px;
}

.error-list {
 margin: 4px 0 0 0;
 padding-left: 16px;
 font-size: 12px;
}

.error-list li {
 margin-bottom: 2px;
}

/* 表单样式调整 */
:deep(.n-form-item) {
 margin-bottom: 12px;
}

:deep(.n-form-item:last-child) {
 margin-bottom: 0;
}

:deep(.n-form-item-label) {
 font-size: 13px;
}

/* 可用变量列表样式 */
.available-variables {
 display: flex;
 flex-wrap: wrap;
 gap: 6px;
}

.variable-tag {
 cursor: pointer;
 transition: all 0.2s ease;
}

.variable-tag:hover {
 transform: scale(1.05);
 box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

/* 表达式错误样式 */
.expression-errors {
 width: 100%;
}

.expression-error-list {
 margin: 4px 0 0 0;
 padding-left: 16px;
 font-size: 12px;
}

.expression-error-list li {
 margin-bottom: 2px;
}
</style>
