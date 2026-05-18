<script setup>
import { ref, computed, onMounted, onUnmounted, provide } from 'vue'
import { Splitpanes, Pane } from 'splitpanes'
import 'splitpanes/dist/splitpanes.css'
import { NTabs, NTabPane } from 'naive-ui'

// Props
const props = defineProps({
 // 默认面板尺寸
 defaultLeftSize: { type: Number, default: 18 },
 defaultMiddleSize: { type: Number, default: 47 },
 defaultRightSize: { type: Number, default: 35 }
})

// 响应式状态
const leftPaneSize = ref(props.defaultLeftSize)
const middlePaneSize = ref(props.defaultMiddleSize)
const rightPaneSize = ref(props.defaultRightSize)
const windowWidth = ref(window.innerWidth)
const activeTab = ref('config')

// 响应式断点
const BREAKPOINT_DESKTOP = 1200
const BREAKPOINT_TABLET = 768

// 计算布局模式
const layoutMode = computed(() => {
 if (windowWidth.value >= BREAKPOINT_DESKTOP) {
 return 'desktop' // 三栏布局
 } else if (windowWidth.value >= BREAKPOINT_TABLET) {
 return 'tablet' // 两栏布局
 } else {
 return 'mobile' // 单栏标签页布局
 }
})

// 是否为移动端视图
const isMobile = computed(() => layoutMode.value === 'mobile')
const isTablet = computed(() => layoutMode.value === 'tablet')
const isDesktop = computed(() => layoutMode.value === 'desktop')

// 提供给子组件
provide('layoutMode', layoutMode)
provide('isMobile', isMobile)

// 处理窗口大小变化
const handleResize = () => {
 windowWidth.value = window.innerWidth
}

// 处理面板大小变化
const handlePaneResize = (panes) => {
 if (panes.length >= 3) {
 leftPaneSize.value = panes[0].size
 middlePaneSize.value = panes[1].size
 rightPaneSize.value = panes[2].size
 } else if (panes.length === 2) {
 // 平板模式下只有两个面板
 leftPaneSize.value = panes[0].size
 rightPaneSize.value = panes[1].size
 }
}

// 生命周期
onMounted(() => {
 window.addEventListener('resize', handleResize)
})

onUnmounted(() => {
 window.removeEventListener('resize', handleResize)
})
</script>

<template>
 <div class="main-layout">
 <!-- 头部插槽 -->
 <header class="layout-header">
 <slot name="header"></slot>
 </header>

 <!-- 主内容区 -->
 <main class="layout-main">
 <!-- 桌面端：三栏布局 -->
 <Splitpanes
 v-if="isDesktop"
 class="default-theme"
 @resize="handlePaneResize"
 >
 <!-- 左侧：组件库 -->
 <Pane :size="leftPaneSize" :min-size="12" :max-size="25">
 <div class="pane-content pane-left">
 <slot name="left"></slot>
 </div>
 </Pane>

 <!-- 中间：工作流配置 -->
 <Pane :size="middlePaneSize" :min-size="30">
 <div class="pane-content pane-middle">
 <slot name="middle"></slot>
 </div>
 </Pane>

 <!-- 右侧：执行面板 -->
 <Pane :size="rightPaneSize" :min-size="25" :max-size="45">
 <div class="pane-content pane-right">
 <slot name="right"></slot>
 </div>
 </Pane>
 </Splitpanes>

 <!-- 平板端：两栏布局 -->
 <Splitpanes
 v-else-if="isTablet"
 class="default-theme"
 @resize="handlePaneResize"
 >
 <!-- 左侧：组件库 + 工作流配置 -->
 <Pane :size="60" :min-size="40" :max-size="70">
 <div class="pane-content pane-combined">
 <div class="combined-left">
 <slot name="left"></slot>
 </div>
 <div class="combined-middle">
 <slot name="middle"></slot>
 </div>
 </div>
 </Pane>

 <!-- 右侧：执行面板 -->
 <Pane :size="40" :min-size="30" :max-size="60">
 <div class="pane-content pane-right">
 <slot name="right"></slot>
 </div>
 </Pane>
 </Splitpanes>

 <!-- 移动端：标签页布局 -->
 <div v-else class="mobile-layout">
 <n-tabs v-model:value="activeTab" type="line" animated>
 <n-tab-pane name="library" tab="组件库">
 <div class="mobile-pane">
 <slot name="left"></slot>
 </div>
 </n-tab-pane>
 <n-tab-pane name="config" tab="工作流配置">
 <div class="mobile-pane">
 <slot name="middle"></slot>
 </div>
 </n-tab-pane>
 <n-tab-pane name="execute" tab="执行">
 <div class="mobile-pane">
 <slot name="right"></slot>
 </div>
 </n-tab-pane>
 </n-tabs>
 </div>
 </main>
 </div>
</template>

<style scoped>
.main-layout {
 display: flex;
 flex-direction: column;
 height: 100%;
 min-height: 0;
 width: 100%;
 overflow: hidden;
 background-color: var(--n-body-color, #f5f7fa);
}

.layout-header {
 flex-shrink: 0;
 z-index: 100;
}

.layout-main {
 flex: 1;
 min-height: 0;
 overflow: hidden;
 display: flex;
}

/* Splitpanes 样式覆盖 */
:deep(.splitpanes) {
 height: 100%;
 min-height: 0;
}

:deep(.splitpanes__pane) {
 min-height: 0;
 background-color: transparent;
}

:deep(.splitpanes__splitter) {
 background-color: var(--n-border-color, #e0e0e6);
 position: relative;
}

:deep(.splitpanes__splitter:before) {
 content: '';
 position: absolute;
 left: 0;
 top: 0;
 transition: opacity 0.3s;
 background-color: var(--n-primary-color, #18a058);
 opacity: 0;
 z-index: 1;
}

:deep(.splitpanes__splitter:hover:before) {
 opacity: 1;
}

:deep(.splitpanes--vertical > .splitpanes__splitter:before) {
 left: -2px;
 right: -2px;
 height: 100%;
 width: auto;
}

:deep(.splitpanes--horizontal > .splitpanes__splitter:before) {
 top: -2px;
 bottom: -2px;
 width: 100%;
 height: auto;
}

/* 面板内容样式 */
.pane-content {
 height: 100%;
 min-height: 0;
 overflow: hidden;
 display: flex;
 flex-direction: column;
}

.pane-left {
 background-color: var(--n-card-color, #ffffff);
 border-right: 1px solid var(--n-border-color, #e0e0e6);
}

.pane-middle {
 background-color: var(--n-body-color, #f5f7fa);
}

.pane-right {
 background-color: var(--n-card-color, #ffffff);
 border-left: 1px solid var(--n-border-color, #e0e0e6);
}

/* 平板端组合面板样式 */
.pane-combined {
 display: flex;
 flex-direction: row;
 height: 100%;
 min-height: 0;
}

.combined-left {
 width: 220px;
 min-width: 180px;
 max-width: 280px;
 flex-shrink: 0;
 min-height: 0;
 background-color: var(--n-card-color, #ffffff);
 border-right: 1px solid var(--n-border-color, #e0e0e6);
 overflow: hidden;
}

.combined-middle {
 flex: 1;
 min-width: 0;
 min-height: 0;
 overflow: hidden;
 background-color: var(--n-body-color, #f5f7fa);
}

/* 移动端布局样式 */
.mobile-layout {
 flex: 1;
 min-height: 0;
 display: flex;
 flex-direction: column;
 overflow: hidden;
 padding: 0;
}

.mobile-layout :deep(.n-tabs) {
 height: 100%;
 min-height: 0;
 display: flex;
 flex-direction: column;
}

.mobile-layout :deep(.n-tabs-nav) {
 padding: 0 12px;
 background-color: var(--n-card-color, #ffffff);
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
}

.mobile-layout :deep(.n-tabs-pane-wrapper) {
 flex: 1;
 min-height: 0;
 overflow: hidden;
}

.mobile-layout :deep(.n-tab-pane) {
 height: 100%;
 min-height: 0;
 padding: 0;
}

.mobile-pane {
 height: 100%;
 min-height: 0;
 overflow: auto;
 background-color: var(--n-body-color, #f5f7fa);
}
</style>
