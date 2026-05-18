<script setup>
import { computed, inject } from 'vue'
import {
 NButton,
 NSelect,
 NSpace,
 NTooltip,
 NIcon,
 NText,
 NDivider
} from 'naive-ui'
import {
 AppsOutline,
 SaveOutline,
 CopyOutline,
 CodeSlashOutline,
 FolderOpenOutline,
 SunnyOutline,
 MoonOutline,
 ChevronDownOutline,
 AddOutline,
 DownloadOutline,
 PushOutline
} from '@vicons/ionicons5'
import { useConfigStore } from '../../store'

// Props
const props = defineProps({
 projects: { type: Array, default: () => [] },
 loading: { type: Boolean, default: false },
 removedGenerating: { type: Boolean, default: false }
})

// Emits
const emit = defineEmits([
 'new',
 'save',
 'saveAs',
 'removedGenerate',
 'export',
 'import',
 'openProjectDrawer',
 'toggleTheme',
 'selectProject'
])

// Store
const configStore = useConfigStore()

// 从 App.vue 注入的主题状态
const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())

// 计算属性
const currentProject = computed(() => configStore.currentProject)
const currentProjectName = computed(() => currentProject.value?.name || '未选择项目')
const hasCurrentProject = computed(() => !!currentProject.value)

// 项目选项
const projectOptions = computed(() => {
 return props.projects.map(project => ({
 label: project.name,
 value: project.id,
 project: project
 }))
})

// 当前选中的项目ID
const selectedProjectId = computed({
 get: () => currentProject.value?.id || null,
 set: (value) => {
 const project = props.projects.find(p => p.id === value)
 if (project) {
 emit('selectProject', project)
 }
 }
})

// 加载中的占位文本
const selectPlaceholder = computed(() => {
 return props.loading ? '加载中...' : '选择项目'
})

// 处理新建
const handleNew = () => {
 emit('new')
}

// 处理保存
const handleSave = () => {
 emit('save')
}

// 处理另存为
const handleSaveAs = () => {
 emit('saveAs')
}

// 处理生成 代码
const handleRemovedGenerate = () => {
 emit('removedGenerate')
}

// 处理导出
const handleExport = () => {
 emit('export')
}

// 处理导入
const handleImport = () => {
 emit('import')
}

// 处理打开项目抽屉
const handleOpenProjectDrawer = () => {
 emit('openProjectDrawer')
}

// 处理主题切换
const handleToggleTheme = () => {
 toggleTheme()
 emit('toggleTheme')
}
</script>

<template>
 <div class="app-header">
 <!-- 左侧：Logo 和标题 -->
 <div class="header-left">
 <n-tooltip trigger="hover">
 <template #trigger>
 <router-link class="home-link" to="/">
 <n-icon><AppsOutline /></n-icon>
 </router-link>
 </template>
 返回首页
 </n-tooltip>
 <div class="logo">
 <span class="logo-icon">🔐</span>
 <span class="logo-text">加解密工具</span>
 </div>
 </div>

 <!-- 中间：项目选择器 -->
 <div class="header-center">
 <n-space align="center" :size="12">
 <n-text depth="3" class="project-label">当前项目:</n-text>
 <n-select
 v-model:value="selectedProjectId"
 :options="projectOptions"
 :loading="loading"
 :placeholder="selectPlaceholder"
 :disabled="loading"
 clearable
 filterable
 style="width: 200px"
 size="small"
 >
 <template #empty>
 <div class="empty-projects">
 <n-text depth="3">暂无项目</n-text>
 </div>
 </template>
 </n-select>
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 size="small"
 @click="handleOpenProjectDrawer"
 >
 <template #icon>
 <n-icon><FolderOpenOutline /></n-icon>
 </template>
 </n-button>
 </template>
 项目管理
 </n-tooltip>
 </n-space>
 </div>

 <!-- 右侧：操作按钮 -->
 <div class="header-right">
 <n-space align="center" :size="8">
 <!-- 新建按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 secondary
 size="small"
 @click="handleNew"
 >
 <template #icon>
 <n-icon><AddOutline /></n-icon>
 </template>
 新建
 </n-button>
 </template>
 新建项目
 </n-tooltip>

 <!-- 保存按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 type="primary"
 size="small"
 @click="handleSave"
 >
 <template #icon>
 <n-icon><SaveOutline /></n-icon>
 </template>
 保存
 </n-button>
 </template>
 {{ hasCurrentProject ? '保存到当前项目' : '保存为新项目' }}
 </n-tooltip>

 <!-- 另存为按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 secondary
 size="small"
 @click="handleSaveAs"
 >
 <template #icon>
 <n-icon><CopyOutline /></n-icon>
 </template>
 另存为
 </n-button>
 </template>
 保存为新项目
 </n-tooltip>

 <!-- 生成 代码按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 secondary
 size="small"
 :loading="removedGenerating"
 @click="handleRemovedGenerate"
 >
 <template #icon>
 <n-icon><CodeSlashOutline /></n-icon>
 </template>
 已移除的代码生成
 </n-button>
 </template>
 生成当前工作流对应的 代码
 </n-tooltip>

 <n-divider vertical />

 <!-- 导出按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 secondary
 size="small"
 @click="handleExport"
 >
 <template #icon>
 <n-icon><DownloadOutline /></n-icon>
 </template>
 导出
 </n-button>
 </template>
 导出当前配置为 JSON 文件
 </n-tooltip>

 <!-- 导入按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 secondary
 size="small"
 @click="handleImport"
 >
 <template #icon>
 <n-icon><PushOutline /></n-icon>
 </template>
 导入
 </n-button>
 </template>
 从 JSON 文件导入配置
 </n-tooltip>

 <n-divider vertical />

 <!-- 主题切换按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 size="small"
 @click="handleToggleTheme"
 >
 <template #icon>
 <n-icon>
 <MoonOutline v-if="!isDarkMode" />
 <SunnyOutline v-else />
 </n-icon>
 </template>
 </n-button>
 </template>
 {{ isDarkMode ? '切换到亮色模式' : '切换到暗色模式' }}
 </n-tooltip>
 </n-space>
 </div>
 </div>
</template>

<style scoped>
.app-header {
 display: flex;
 align-items: center;
 justify-content: space-between;
 padding: 0 20px;
 height: 56px;
 background-color: var(--n-card-color, #ffffff);
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
 box-shadow: 0 1px 4px rgba(0, 0, 0, 0.05);
}

.header-left {
 display: flex;
 align-items: center;
 gap: 10px;
}

.home-link {
 width: 32px;
 height: 32px;
 flex: none;
 display: grid;
 place-items: center;
 border-radius: 6px;
 color: var(--n-text-color-2, #666666);
 transition: background-color 0.15s ease, color 0.15s ease;
}

.home-link:hover {
 color: var(--n-primary-color, #18a058);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
}

.logo {
 display: flex;
 align-items: center;
 gap: 8px;
}

.logo-icon {
 font-size: 24px;
}

.logo-text {
 font-size: 18px;
 font-weight: 600;
 color: var(--n-text-color-1, #333);
 white-space: nowrap;
}

.header-center {
 display: flex;
 align-items: center;
 justify-content: center;
 flex: 1;
 padding: 0 20px;
}

.project-label {
 font-size: 13px;
 white-space: nowrap;
}

.header-right {
 display: flex;
 align-items: center;
}

.empty-projects {
 padding: 12px;
 text-align: center;
}

/* 响应式调整 */
@media (max-width: 768px) {
 .app-header {
 padding: 0 12px;
 height: 48px;
 }

 .logo-text {
 display: none;
 }

 .project-label {
 display: none;
 }

 .header-center :deep(.n-select) {
 width: 140px !important;
 }

 .header-right :deep(.n-button__content) {
 /* 移动端隐藏按钮文字 */
 }
}

@media (max-width: 480px) {
 .header-center {
 display: none;
 }
}
</style>
