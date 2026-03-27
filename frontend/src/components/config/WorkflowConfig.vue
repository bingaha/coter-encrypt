<script setup>
import { ref, computed, watch } from 'vue'
import {
 NCard,
 NButton,
 NIcon,
 NEmpty,
 NSpace,
 NText,
 NSelect,
 NInput,
 NTooltip,
 NScrollbar,
 NDivider
} from 'naive-ui'
import {
 AddOutline,
 TrashOutline,
 ArrowDownOutline,
 LinkOutline,
 EnterOutline
} from '@vicons/ionicons5'
import draggable from 'vuedraggable'
import { useConfigStore } from '@/store/index.js'
import ComponentCard from './ComponentCard.vue'

// Store
const configStore = useConfigStore()

// 拖拽状态
const isDragging = ref(false)
const dragOverIndex = ref(null)

// 当前选中的组件ID
const selectedComponentId = computed(() => configStore.selectedComponent?.id || null)

// 当前悬停高亮的输入源组件ID
const highlightedSourceId = ref(null)

// 已添加的组件列表
const addedComponents = computed({
 get: () => configStore.addedComponents,
 set: (value) => {
 configStore.addedComponents = value
 }
})

// 选择组件
const handleSelectComponent = (componentId) => {
 configStore.selectComponent(componentId)
}

// 处理组件悬停 - 高亮输入来源组件
// Requirements: 5.5
const handleComponentHover = (componentId) => {
 if (!componentId) {
 highlightedSourceId.value = null
 return
 }
 
 const component = addedComponents.value.find(c => c.id === componentId)
 if (component && component.config.inputSourceType === 'component' && component.config.inputComponentRef) {
 highlightedSourceId.value = component.config.inputComponentRef
 } else {
 highlightedSourceId.value = null
 }
}

// 处理组件离开悬停
const handleComponentLeave = () => {
 highlightedSourceId.value = null
}

// 输出映射列表
const outputMappings = computed(() => configStore.outputMappings)

// 输入映射列表
const inputMappings = computed(() => configStore.inputMappings)

// 获取可用的输出源（所有已添加的组件）
const availableOutputSources = computed(() => {
 return configStore.addedComponents.map(c => ({
 label: `${c.name} (${c.outputRef})`,
 value: c.outputRef
 }))
})

// 处理拖拽开始
const handleDragStart = () => {
 isDragging.value = true
}

// 处理拖拽结束
const handleDragEnd = () => {
 isDragging.value = false
 dragOverIndex.value = null
}

// 处理外部拖拽进入
const handleDragOver = (event) => {
 event.preventDefault()
 event.dataTransfer.dropEffect = 'copy'
}

// 处理外部拖拽放置
const handleDrop = (event) => {
 event.preventDefault()
 try {
 const data = JSON.parse(event.dataTransfer.getData('application/json'))
 if (data.type === 'component') {
 configStore.addComponent(data.componentId)
 }
 } catch (e) {
 console.error('Invalid drop data:', e)
 }
 dragOverIndex.value = null
}
</script>

<template>
 <div class="workflow-config">
 <!-- 头部 -->
 <div class="workflow-header">
 <h3 class="workflow-title">工作流配置</h3>
 <n-text depth="3" class="component-count">
 {{ addedComponents.length }} 个组件
 </n-text>
 </div>

 <!-- 组件列表区域 -->
 <n-scrollbar class="workflow-content">
 <!-- 输入映射配置区 -->
 <div class="input-mapping-section">
 <n-divider>
 <n-space align="center" :size="4">
 <n-icon :size="16"><EnterOutline /></n-icon>
 <span>输入映射</span>
 </n-space>
 </n-divider>

 <div class="mapping-list">
 <!-- 已有的映射 -->
 <div
 v-for="mapping in inputMappings"
 :key="mapping.id"
 class="mapping-item"
 >
 <n-input
 v-model:value="mapping.name"
 placeholder="显示名称"
 size="small"
 style="flex: 1; min-width: 50px"
 @update:value="(val) => configStore.updateInputMapping(mapping.id, { name: val })"
 />
 <n-input
 v-model:value="mapping.inputRef"
 placeholder="字段名"
 size="small"
 style="flex: 1; min-width: 50px"
 @update:value="(val) => configStore.updateInputMapping(mapping.id, { inputRef: val })"
 />
 <n-input
 v-model:value="mapping.defaultValue"
 placeholder="默认值"
 size="small"
 style="flex: 3; min-width: 60px"
 @update:value="(val) => configStore.updateInputMapping(mapping.id, { defaultValue: val })"
 />
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 size="tiny"
 type="error"
 style="flex-shrink: 0"
 @click="configStore.removeInputMapping(mapping.id)"
 >
 <template #icon>
 <n-icon><TrashOutline /></n-icon>
 </template>
 </n-button>
 </template>
 删除映射
 </n-tooltip>
 </div>

 <!-- 添加映射按钮 -->
 <n-button
 dashed
 block
 size="small"
 @click="configStore.addInputMapping('')"
 >
 <template #icon>
 <n-icon><AddOutline /></n-icon>
 </template>
 添加输入映射
 </n-button>
 </div>
 </div>

 <div
 class="drop-zone"
 :class="{ 'is-dragging': isDragging, 'is-empty': addedComponents.length === 0 }"
 @dragover="handleDragOver"
 @drop="handleDrop"
 >
 <!-- 空状态 -->
 <n-empty
 v-if="addedComponents.length === 0"
 description="从左侧组件库拖拽或点击添加组件"
 class="empty-state"
 >
 <template #icon>
 <n-icon :size="48" color="#ccc">
 <AddOutline />
 </n-icon>
 </template>
 </n-empty>

 <!-- 组件列表 - 可拖拽排序 -->
 <draggable
 v-else
 v-model="addedComponents"
 item-key="id"
 handle=".drag-handle"
 ghost-class="ghost-card"
 chosen-class="chosen-card"
 animation="200"
 @start="handleDragStart"
 @end="handleDragEnd"
 >
 <template #item="{ element, index }">
 <div class="component-wrapper">
 <!-- 连接线 -->
 <div v-if="index > 0" class="connection-line">
 <n-icon :size="16" color="#18a058">
 <ArrowDownOutline />
 </n-icon>
 </div>
 
 <!-- 组件卡片 -->
 <ComponentCard
 :component="element"
 :index="index"
 :is-selected="selectedComponentId === element.id"
 :is-highlighted="highlightedSourceId === element.id"
 :available-input-sources="configStore.getAvailableInputSources(element.id)"
 @select="handleSelectComponent(element.id)"
 @hover="handleComponentHover(element.id)"
 @leave="handleComponentLeave"
 />
 </div>
 </template>
 </draggable>
 </div>

 <!-- 输出映射配置区 -->
 <div class="output-mapping-section" v-if="addedComponents.length > 0">
 <n-divider>
 <n-space align="center" :size="4">
 <n-icon :size="16"><LinkOutline /></n-icon>
 <span>输出映射</span>
 </n-space>
 </n-divider>

 <div class="mapping-list">
 <!-- 已有的映射 -->
 <div
 v-for="mapping in outputMappings"
 :key="mapping.id"
 class="mapping-item"
 >
 <n-input
 v-model:value="mapping.name"
 placeholder="输出名称"
 size="small"
 style="width: 120px"
 @update:value="(val) => configStore.updateOutputMapping(mapping.id, { name: val })"
 />
 <n-icon :size="14" color="#999"><ArrowDownOutline /></n-icon>
 <n-select
 :value="mapping.componentRef"
 :options="availableOutputSources"
 placeholder="选择组件输出"
 size="small"
 style="flex: 1"
 @update:value="(val) => configStore.updateOutputMapping(mapping.id, { componentRef: val })"
 />
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 size="tiny"
 type="error"
 @click="configStore.removeOutputMapping(mapping.id)"
 >
 <template #icon>
 <n-icon><TrashOutline /></n-icon>
 </template>
 </n-button>
 </template>
 删除映射
 </n-tooltip>
 </div>

 <!-- 添加映射按钮 -->
 <n-button
 dashed
 block
 size="small"
 @click="configStore.addOutputMapping('', '')"
 >
 <template #icon>
 <n-icon><AddOutline /></n-icon>
 </template>
 添加输出映射
 </n-button>
 </div>
 </div>
 </n-scrollbar>
 </div>
</template>


<style scoped>
.workflow-config {
 display: flex;
 flex-direction: column;
 height: 100%;
 overflow: hidden;
}

.workflow-header {
 display: flex;
 align-items: center;
 justify-content: space-between;
 padding: 16px;
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
 background-color: var(--n-card-color, #fff);
 flex-shrink: 0;
}

.workflow-title {
 margin: 0;
 font-size: 16px;
 font-weight: 600;
 color: var(--n-text-color-1, #333);
}

.component-count {
 font-size: 12px;
}

.workflow-content {
 flex: 1;
 overflow: hidden;
}

.drop-zone {
 min-height: 100%;
 padding: 16px;
 transition: background-color 0.2s ease;
}

.drop-zone.is-dragging {
 background-color: var(--n-primary-color-suppl, rgba(24, 160, 88, 0.08));
}

.drop-zone.is-empty {
 display: flex;
 align-items: center;
 justify-content: center;
}

.empty-state {
 padding: 40px 20px;
}

/* 组件包装器 */
.component-wrapper {
 position: relative;
 margin-bottom: 8px;
}

/* 连接线样式 */
.connection-line {
 display: flex;
 justify-content: center;
 padding: 4px 0;
 color: var(--n-primary-color, #18a058);
}

/* 拖拽幽灵样式 */
.ghost-card {
 opacity: 0.5;
 background-color: var(--n-primary-color-suppl, rgba(24, 160, 88, 0.1));
 border: 2px dashed var(--n-primary-color, #18a058);
 border-radius: 8px;
}

.chosen-card {
 box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

/* 输入映射区域 */
.input-mapping-section {
 padding: 16px 16px 0;
}

/* 输出映射区域 */
.output-mapping-section {
 padding: 0 16px 16px;
}

.mapping-list {
 display: flex;
 flex-direction: column;
 gap: 8px;
}

.mapping-item {
 display: flex;
 align-items: center;
 gap: 8px;
 padding: 8px 12px;
 background-color: var(--n-card-color, #fff);
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 6px;
}

/* 滚动条样式 */
:deep(.n-scrollbar-content) {
 min-height: 100%;
}
</style>
