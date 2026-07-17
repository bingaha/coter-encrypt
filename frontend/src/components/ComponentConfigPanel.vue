<script setup>
import { ref, watch, computed } from 'vue'
import {
 NForm,
 NFormItem,
 NInput,
 NSelect,
 NRadioGroup,
 NRadio,
 NSpace,
 NAlert,
 NText
} from 'naive-ui'
import { useConfigStore } from '../store'
// 导入参数引用选择器组件
import ParamRefSelector from './config/ParamRefSelector.vue'

const props = defineProps({
 component: {
 type: Object,
 required: true
 }
})

const configStore = useConfigStore()
const localConfig = ref({ ...props.component.config })

// 标记是否正在从外部同步，避免循环更新
let isSyncingFromProps = false

// 监听组件变化，更新本地配置
watch(() => props.component.config, (newConfig) => {
 isSyncingFromProps = true
 localConfig.value = { ...newConfig }
 setTimeout(() => {
 isSyncingFromProps = false
 }, 0)
}, { deep: true })

// 获取当前组件的校验错误
const validationErrors = computed(() => {
 return configStore.validateComponent({
 ...props.component,
 config: localConfig.value
 })
})

// 检查是否有校验错误
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

// 获取可用的输入源组件列表
const availableInputSources = computed(() => {
 return configStore.getAvailableInputSources(props.component.id)
})

// 输入源选项
const inputSourceOptions = computed(() => {
 return availableInputSources.value.map(source => ({
 label: `${source.name} (${source.outputRef})`,
 value: source.id
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
 if (availableInputSources.value.length > 0) {
 options.push({
 type: 'group',
 label: '组件输出',
 key: 'component-outputs',
 children: availableInputSources.value.map(source => ({
 label: `${source.name} (${source.outputRef})`,
 value: source.outputRef
 }))
 })
 }
 
 return options
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

// 自动保存配置
watch(localConfig, (newConfig) => {
 if (isSyncingFromProps) return
 configStore.updateComponentConfig(props.component.id, newConfig)
}, { deep: true })
</script>

<template>
 <div class="component-config-panel">
 <h3>{{ component.name }} 配置</h3>
 
 <n-alert v-if="hasErrors" type="warning" :show-icon="true" class="validation-warning">
 <template #header>配置不完整</template>
 <ul class="error-list">
 <li v-for="(error, index) in validationErrors" :key="index">{{ error.message }}</li>
 </ul>
 </n-alert>
 
 <n-form :model="localConfig" label-placement="left" label-width="120px" size="small">
 <!-- 输入来源类型选择 -->
 <n-form-item label="输入来源" 
 :validation-status="hasFieldError('inputSourceType') ? 'error' : undefined"
 :feedback="getFieldError('inputSourceType')">
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
 :validation-status="hasFieldError('inputMappingRef') ? 'error' : undefined"
 :feedback="getFieldError('inputMappingRef')"
 >
 <n-select 
 v-model:value="localConfig.inputMappingRef" 
 :options="combinedInputOptions"
 placeholder="选择输入映射或组件输出"
 :disabled="combinedInputOptions.length === 0"
 />
 <template v-if="combinedInputOptions.length === 0" #feedback>
 <n-text type="warning" style="font-size: 12px;">
 请先添加输入映射或上游组件
 </n-text>
 </template>
 </n-form-item>
 
 <!-- 表达式输入 -->
 <n-form-item 
 v-if="localConfig.inputSourceType === 'expression'"
 label="输入表达式"
 :validation-status="hasFieldError('inputExpression') ? 'error' : undefined"
 :feedback="getFieldError('inputExpression')"
 >
 <n-input 
 v-model:value="localConfig.inputExpression" 
 type="textarea"
 :rows="3"
 placeholder="输入表达式，使用 ${ref} 引用输入映射或组件输出&#10;例如：固定前缀${input-xxx} 或 ${base64-output-xxx}"
 />
 </n-form-item>
 
 <!-- Base64组件配置 -->
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
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>
 
 <!-- MD5组件配置 -->
 <template v-else-if="component.type === 'md5'">
 <n-form-item label="结果格式">
 <n-radio-group v-model:value="localConfig.resultFormat">
 <n-space>
 <n-radio value="lowercase">小写</n-radio>
 <n-radio value="uppercase">大写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>

 <!-- SHA256组件配置 -->
 <template v-else-if="component.type === 'sha256'">
 <n-form-item label="结果格式">
 <n-radio-group v-model:value="localConfig.resultFormat">
 <n-space>
 <n-radio value="lowercase">小写</n-radio>
 <n-radio value="uppercase">大写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>

 <!-- HmacMD5组件配置 -->
 <template v-else-if="component.type === 'hmacmd5'">
 <n-form-item label="密钥来源" 
 :validation-status="hasFieldError('keyRef') ? 'error' : undefined"
 :feedback="getFieldError('keyRef')">
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
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>

 <!-- HmacSHA256组件配置 -->
 <template v-else-if="component.type === 'hmacsha256'">
 <n-form-item label="密钥来源" 
 :validation-status="hasFieldError('keyRef') ? 'error' : undefined"
 :feedback="getFieldError('keyRef')">
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
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>

 <!-- AES组件配置 -->
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
 <n-radio value="NoPadding">NoPadding</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="密钥" 
 :validation-status="hasFieldError('key') ? 'error' : undefined"
 :feedback="getFieldError('key')">
 <n-input v-model:value="localConfig.key" placeholder="请输入密钥" />
 </n-form-item>
 <n-form-item label="密钥格式">
 <n-radio-group v-model:value="localConfig.keyFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item v-if="localConfig.mode === 'CBC'" label="加密向量(IV)" 
 :validation-status="hasFieldError('iv') ? 'error' : undefined"
 :feedback="getFieldError('iv')">
 <n-input v-model:value="localConfig.iv" placeholder="CBC模式下为必填项" />
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
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>
 
 <!-- SM4组件配置 -->
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
 <n-form-item label="密钥" 
 :validation-status="hasFieldError('key') ? 'error' : undefined"
 :feedback="getFieldError('key')">
 <n-input v-model:value="localConfig.key" placeholder="请输入密钥" />
 </n-form-item>
 <n-form-item label="密钥格式">
 <n-radio-group v-model:value="localConfig.keyFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item v-if="localConfig.mode === 'CBC'" label="加密向量(IV)" 
 :validation-status="hasFieldError('iv') ? 'error' : undefined"
 :feedback="getFieldError('iv')">
 <n-input v-model:value="localConfig.iv" placeholder="CBC模式下为必填项" />
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
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>
 
 <!-- Blowfish组件配置 -->
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
 <n-form-item label="密钥" 
 :validation-status="hasFieldError('key') ? 'error' : undefined"
 :feedback="getFieldError('key')">
 <n-input v-model:value="localConfig.key" placeholder="请输入密钥" />
 </n-form-item>
 <n-form-item label="密钥格式">
 <n-radio-group v-model:value="localConfig.keyFormat">
 <n-space>
 <n-radio value="hex">HEX</n-radio>
 <n-radio value="text">TEXT</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item v-if="localConfig.mode === 'CBC'" label="加密向量(IV)" 
 :validation-status="hasFieldError('iv') ? 'error' : undefined"
 :feedback="getFieldError('iv')">
 <n-input v-model:value="localConfig.iv" placeholder="CBC模式下为8字节" />
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
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>
 
 <!-- Hex组件配置 -->
 <template v-else-if="component.type === 'hex'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encode">编码</n-radio>
 <n-radio value="decode">解码</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>

 <!-- 进制转换组件配置 -->
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
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>

 <!-- URL组件配置 -->
 <template v-else-if="component.type === 'url'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encode">编码</n-radio>
 <n-radio value="decode">解码</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>
 
 <!-- Unicode组件配置 -->
 <template v-else-if="component.type === 'unicode'">
 <n-form-item label="操作类型">
 <n-radio-group v-model:value="localConfig.operation">
 <n-space>
 <n-radio value="encode">编码</n-radio>
 <n-radio value="decode">解码</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>
 
 <!-- SM2组件配置 -->
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
 <n-form-item v-if="localConfig.operation === 'encrypt'" label="公钥"
 :validation-status="hasFieldError('publicKey') ? 'error' : undefined"
 :feedback="getFieldError('publicKey')">
 <n-input v-model:value="localConfig.publicKey" type="textarea" :rows="3" placeholder="请输入SM2公钥" />
 </n-form-item>
 <n-form-item v-if="localConfig.operation === 'decrypt'" label="私钥"
 :validation-status="hasFieldError('privateKey') ? 'error' : undefined"
 :feedback="getFieldError('privateKey')">
 <n-input v-model:value="localConfig.privateKey" type="textarea" :rows="3" placeholder="请输入SM2私钥" />
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
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>
 
 <!-- RSA组件配置 -->
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
 <n-form-item v-if="localConfig.operation === 'encrypt'" label="公钥"
 :validation-status="hasFieldError('publicKey') ? 'error' : undefined"
 :feedback="getFieldError('publicKey')">
 <n-input v-model:value="localConfig.publicKey" type="textarea" :rows="3" placeholder="请输入RSA公钥" />
 </n-form-item>
 <n-form-item v-if="localConfig.operation === 'decrypt'" label="私钥"
 :validation-status="hasFieldError('privateKey') ? 'error' : undefined"
 :feedback="getFieldError('privateKey')">
 <n-input v-model:value="localConfig.privateKey" type="textarea" :rows="3" placeholder="请输入RSA私钥" />
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
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>
 
 <!-- SM3组件配置 -->
 <template v-else-if="component.type === 'sm3'">
 <n-form-item label="结果格式">
 <n-radio-group v-model:value="localConfig.resultFormat">
 <n-space>
 <n-radio value="lowercase">小写</n-radio>
 <n-radio value="uppercase">大写</n-radio>
 </n-space>
 </n-radio-group>
 </n-form-item>
 <n-form-item label="输出标识">
 <n-input :value="component.outputRef" placeholder="设置输出参数标识符" disabled />
 </n-form-item>
 </template>
 </n-form>
 </div>
</template>

<style scoped>
.component-config-panel {
 padding: 1rem;
 width: 100%;
 max-width: 100%;
 overflow-x: hidden;
 box-sizing: border-box;
}

.component-config-panel h3 {
 font-size: 1.1rem;
 font-weight: 600;
 color: var(--n-text-color-1, #333);
 margin-bottom: 1rem;
 padding-bottom: 0.5rem;
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
}

.validation-warning {
 margin-bottom: 1rem;
}

.error-list {
 margin: 0.5rem 0 0 0;
 padding-left: 1.2rem;
 font-size: 12px;
}

.error-list li {
 margin-bottom: 0.25rem;
}

.component-config-panel :deep(.n-form) {
 width: 100%;
 max-width: 100%;
}

.component-config-panel :deep(.n-form-item) {
 width: 100%;
 max-width: 100%;
}
</style>
