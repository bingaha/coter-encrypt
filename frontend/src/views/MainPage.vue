<script setup>
import { ref, onMounted, computed } from 'vue'
import { useMessage, useDialog } from 'naive-ui'
import { MainLayout, AppHeader } from '@/components/layout'
import { ComponentLibrary, WorkflowConfig } from '@/components/config'
import { ExecutePanel } from '@/components/execute'
import { ProjectDrawer } from '@/components/project'
import { useConfigStore } from '@/store'
import { useProjectManager } from '@/composables/useProjectManager'

// Store
const configStore = useConfigStore()
const message = useMessage()
const dialog = useDialog()

// 项目管理
const {
 projects,
 isLoading,
 isSaving,
 drawerVisible,
 currentProject,
 currentProjectId,
 hasCurrentProject,
 fetchProjects,
 openDrawer,
 closeDrawer,
 handleNew,
 handleSave,
 handleSaveAs,
 loadProject,
 handleDeleteProject,
 handleRenameProject,
 selectProject,
 handleExport,
 handleImport
} = useProjectManager()

// 初始化
onMounted(async () => {
 // 从 localStorage 加载配置
 configStore.loadConfigFromLocal()
 // 获取项目列表
 await fetchProjects()
})

// 处理新建项目
const onNew = () => {
 handleNew()
}

// 处理保存 - Requirements 3.3
const onSave = async () => {
 await handleSave()
}

// 处理另存为
const onSaveAs = async () => {
 await handleSaveAs()
}


// 处理导出
const onExport = () => {
 handleExport()
}

// 处理导入
const onImport = () => {
 handleImport()
}

// 处理打开项目抽屉
const onOpenProjectDrawer = () => {
 openDrawer()
}

// 处理主题切换
const onToggleTheme = () => {
 // 主题切换由 AppHeader 内部处理
}

// 处理从下拉框选择项目
const onSelectProject = (project) => {
 selectProject(project)
}

// 处理加载项目
const onLoadProject = (project) => {
 loadProject(project)
}

// 处理删除项目
const onDeleteProject = async (project) => {
 await handleDeleteProject(project)
}

// 处理重命名项目 - Requirements 3.7
const onRenameProject = async (project, newName) => {
 await handleRenameProject(project, newName)
}

// 刷新项目列表
const onRefreshProjects = async () => {
 await fetchProjects()
}

</script>

<template>
 <MainLayout>
 <!-- 头部 -->
 <template #header>
 <AppHeader
 :projects="projects"
 :loading="isLoading"
 @new="onNew"
 @save="onSave"
 @save-as="onSaveAs"
 @export="onExport"
 @import="onImport"
 @open-project-drawer="onOpenProjectDrawer"
 @toggle-theme="onToggleTheme"
 @select-project="onSelectProject"
 />
 </template>

 <!-- 左侧：组件库 -->
 <template #left>
 <ComponentLibrary />
 </template>

 <!-- 中间：工作流配置 -->
 <template #middle>
 <WorkflowConfig />
 </template>

 <!-- 右侧：执行面板 -->
 <template #right>
 <ExecutePanel />
 </template>
 </MainLayout>

 <!-- 项目管理抽屉 -->
 <ProjectDrawer
 v-model:visible="drawerVisible"
 :projects="projects"
 :loading="isLoading"
 :current-project-id="currentProjectId"
 @close="closeDrawer"
 @load="onLoadProject"
 @delete="onDeleteProject"
 @rename="onRenameProject"
 @refresh="onRefreshProjects"
 />
</template>

<style scoped>
/* MainPage 不需要额外样式，布局由 MainLayout 处理 */
</style>
