<script setup>
/**
 * 参数引用选择器组件
 * 用于选择输入映射或上游组件输出作为参数来源
 * 
 * 支持功能：
 * - 分组显示输入映射和组件输出
 * - v-model 双向绑定
 * - 无可用来源时显示禁用状态和提示
 * 
 * Requirements: 2.1, 2.2, 2.3, 2.5
 */
import { computed } from 'vue'
import { NSelect, NText } from 'naive-ui'
import { useConfigStore } from '@/store/index.js'

// Props 定义
const props = defineProps({
 // v-model 绑定值，格式: "input:xxx" 或 "output:xxx"
 modelValue: {
 type: String,
 default: ''
 },
 // 当前组件ID，用于获取可用的上游组件输出
 componentId: {
 type: String,
 required: true
 },
 // 占位符文本
 placeholder: {
 type: String,
 default: '选择参数来源'
 },
 // 是否禁用（外部控制）
 disabled: {
 type: Boolean,
 default: false
 }
})

// 事件定义
const emit = defineEmits(['update:modelValue'])

// 获取配置 Store
const configStore = useConfigStore()

/**
 * 合并输入映射和上游组件输出选项
 * 分组显示，便于用户识别来源类型
 * 
 * Requirements: 2.2, 2.5
 */
const combinedOptions = computed(() => {
 const options = []
 
 // 输入映射选项组
 // 显示所有已定义的输入映射
 if (configStore.inputMappings.length > 0) {
 options.push({
 type: 'group',
 label: '输入映射',
 key: 'input-mappings',
 children: configStore.inputMappings.map(m => ({
 // 显示名称，如果没有名称则显示引用标识
 label: m.name || m.inputRef,
 // 值格式: "input:{inputRef}"
 value: `input:${m.inputRef}`
 }))
 })
 }
 
 // 上游组件输出选项组
 // 只显示当前组件之前的组件输出
 const availableSources = configStore.getAvailableInputSources(props.componentId)
 if (availableSources.length > 0) {
 options.push({
 type: 'group',
 label: '组件输出',
 key: 'component-outputs',
 children: availableSources.map(s => ({
 // 显示组件名称和输出标识
 label: `${s.name} (${s.outputRef})`,
 // 值格式: "output:{outputRef}"
 value: `output:${s.outputRef}`
 }))
 })
 }
 
 return options
})

/**
 * 是否有可用选项
 * 用于判断是否应该禁用选择器
 * 
 * Requirements: 2.3
 */
const hasOptions = computed(() => {
 return combinedOptions.value.length > 0
})

/**
 * 选择器是否禁用
 * 当没有可用选项或外部禁用时禁用
 */
const isDisabled = computed(() => {
 return props.disabled || !hasOptions.value
})

/**
 * 处理值变化，触发 v-model 更新
 */
const handleUpdate = (value) => {
 emit('update:modelValue', value)
}
</script>

<template>
 <div class="param-ref-selector">
 <n-select
 :value="modelValue"
 :options="combinedOptions"
 :placeholder="placeholder"
 :disabled="isDisabled"
 clearable
 @update:value="handleUpdate"
 />
 <!-- 无可用来源时显示提示信息 -->
 <!-- Requirements: 2.3 -->
 <n-text 
 v-if="!hasOptions" 
 type="warning" 
 class="no-options-hint"
 >
 请先添加输入映射或上游组件
 </n-text>
 </div>
</template>

<style scoped>
.param-ref-selector {
 width: 100%;
}

.no-options-hint {
 display: block;
 font-size: 12px;
 margin-top: 4px;
}
</style>
