<script setup>
import { ref, computed } from 'vue'
import { NCollapse, NCollapseItem, NTooltip, NIcon, NEmpty } from 'naive-ui'
import {
 CodeOutline,
 FingerPrintOutline,
 KeyOutline,
 ShieldCheckmarkOutline,
 DocumentTextOutline,
 CodeSlashOutline,
 LockClosedOutline,
 ShieldOutline,
 AddCircleOutline
} from '@vicons/ionicons5'
import { useConfigStore } from '@/store/index.js'

// Store
const configStore = useConfigStore()

// 图标映射
const iconMap = {
 CodeOutline,
 FingerPrintOutline,
 KeyOutline,
 ShieldCheckmarkOutline,
 DocumentTextOutline,
 CodeSlashOutline,
 LockClosedOutline,
 ShieldOutline,
 AddCircleOutline
}

// 获取图标组件
const getIcon = (iconName) => {
 return iconMap[iconName] || CodeOutline
}

// 分类折叠状态 - 使用展开的分类名称数组
const expandedCategories = ref(
 configStore.componentCategories
 .filter(cat => !cat.collapsed)
 .map(cat => cat.id)
)

// 处理折叠状态变化
const handleCollapseChange = (expandedNames) => {
 expandedCategories.value = expandedNames
 // 同步到 store
 configStore.componentCategories.forEach(cat => {
 cat.collapsed = !expandedNames.includes(cat.id)
 })
}

// 拖拽状态
const draggingComponent = ref(null)

// 开始拖拽
const handleDragStart = (event, component) => {
 draggingComponent.value = component
 // 设置拖拽数据
 event.dataTransfer.effectAllowed = 'copy'
 event.dataTransfer.setData('application/json', JSON.stringify({
 type: 'component',
 componentId: component.id,
 componentName: component.name
 }))
 
 // 添加拖拽样式
 event.target.classList.add('dragging')
}

// 拖拽结束
const handleDragEnd = (event) => {
 draggingComponent.value = null
 event.target.classList.remove('dragging')
}

// 点击添加组件
const handleAddComponent = (component) => {
 configStore.addComponent(component.id)
}
</script>

<template>
 <div class="component-library">
 <div class="library-header">
 <h3 class="library-title">组件库</h3>
 </div>
 
 <div class="library-content">
 <n-collapse
 :expanded-names="expandedCategories"
 @update:expanded-names="handleCollapseChange"
 accordion
 >
 <n-collapse-item
 v-for="category in configStore.componentCategories"
 :key="category.id"
 :name="category.id"
 >
 <template #header>
 <div class="category-header">
 <n-icon :component="getIcon(category.icon)" size="18" />
 <span class="category-name">{{ category.name }}</span>
 <span class="category-count">{{ category.components.length }}</span>
 </div>
 </template>
 
 <div class="component-list">
 <n-tooltip
 v-for="component in category.components"
 :key="component.id"
 placement="right"
 :delay="300"
 >
 <template #trigger>
 <div
 class="component-item"
 draggable="true"
 @dragstart="handleDragStart($event, component)"
 @dragend="handleDragEnd"
 @click="handleAddComponent(component)"
 >
 <n-icon :component="getIcon(component.icon)" size="16" class="component-icon" />
 <span class="component-name">{{ component.name }}</span>
 <n-icon :component="AddCircleOutline" size="14" class="add-icon" />
 </div>
 </template>
 <div class="tooltip-content">
 <div class="tooltip-title">{{ component.name }}</div>
 <div class="tooltip-desc">{{ component.description }}</div>
 <div class="tooltip-hint">点击或拖拽添加到工作流</div>
 </div>
 </n-tooltip>
 
 <n-empty
 v-if="category.components.length === 0"
 description="暂无组件"
 size="small"
 />
 </div>
 </n-collapse-item>
 </n-collapse>
 </div>
 </div>
</template>


<style scoped>
.component-library {
 display: flex;
 flex-direction: column;
 height: 100%;
 overflow: hidden;
}

.library-header {
 padding: 16px;
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
 flex-shrink: 0;
}

.library-title {
 margin: 0;
 font-size: 16px;
 font-weight: 600;
 color: var(--n-text-color-1, #333);
}

.library-content {
 flex: 1;
 overflow-y: auto;
 padding: 8px;
}

/* 分类头部样式 */
.category-header {
 display: flex;
 align-items: center;
 gap: 8px;
 width: 100%;
}

.category-name {
 flex: 1;
 font-weight: 500;
 font-size: 14px;
}

.category-count {
 font-size: 12px;
 color: var(--n-text-color-3, #999);
 background-color: var(--n-tag-color, #f0f0f0);
 padding: 2px 8px;
 border-radius: 10px;
}

/* 组件列表样式 */
.component-list {
 display: flex;
 flex-direction: column;
 gap: 4px;
 padding: 4px 0;
}

/* 组件项样式 */
.component-item {
 display: flex;
 align-items: center;
 gap: 8px;
 padding: 10px 12px;
 border-radius: 6px;
 cursor: grab;
 transition: all 0.2s ease;
 background-color: var(--n-card-color, #fff);
 border: 1px solid var(--n-border-color, #e0e0e6);
 user-select: none;
}

.component-item:hover {
 background-color: var(--n-primary-color-hover, #36ad6a);
 border-color: var(--n-primary-color, #18a058);
 color: #fff;
 transform: translateX(2px);
}

.component-item:hover .component-icon,
.component-item:hover .component-name,
.component-item:hover .add-icon {
 color: #fff;
}

.component-item:active {
 cursor: grabbing;
 transform: scale(0.98);
}

.component-item.dragging {
 opacity: 0.5;
 cursor: grabbing;
}

.component-icon {
 color: var(--n-primary-color, #18a058);
 flex-shrink: 0;
}

.component-name {
 flex: 1;
 font-size: 13px;
 font-weight: 500;
}

.add-icon {
 opacity: 0;
 color: var(--n-text-color-3, #999);
 flex-shrink: 0;
 transition: opacity 0.2s ease;
}

.component-item:hover .add-icon {
 opacity: 1;
}

/* Tooltip 内容样式 */
.tooltip-content {
 max-width: 200px;
}

.tooltip-title {
 font-weight: 600;
 font-size: 14px;
 margin-bottom: 4px;
}

.tooltip-desc {
 font-size: 12px;
 color: var(--n-text-color-2, #666);
 margin-bottom: 8px;
}

.tooltip-hint {
 font-size: 11px;
 color: var(--n-text-color-3, #999);
 padding-top: 6px;
 border-top: 1px solid var(--n-border-color, #e0e0e6);
}

/* Collapse 样式覆盖 */
:deep(.n-collapse-item) {
 margin-bottom: 4px;
}

:deep(.n-collapse-item__header) {
 padding: 10px 12px !important;
 border-radius: 6px;
 background-color: var(--n-card-color, #fff);
}

:deep(.n-collapse-item__header:hover) {
 background-color: var(--n-hover-color, #f5f5f5);
}

:deep(.n-collapse-item__content-inner) {
 padding: 0 4px !important;
}

/* 滚动条样式 */
.library-content::-webkit-scrollbar {
 width: 6px;
}

.library-content::-webkit-scrollbar-track {
 background: transparent;
}

.library-content::-webkit-scrollbar-thumb {
 background-color: var(--n-scrollbar-color, #d9d9d9);
 border-radius: 3px;
}

.library-content::-webkit-scrollbar-thumb:hover {
 background-color: var(--n-scrollbar-color-hover, #bbb);
}
</style>
