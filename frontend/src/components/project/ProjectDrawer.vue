<script setup>
import { ref, computed, watch } from 'vue'
import {
 NDrawer,
 NDrawerContent,
 NList,
 NListItem,
 NThing,
 NButton,
 NSpace,
 NIcon,
 NInput,
 NText,
 NEmpty,
 NSpin,
 NPopconfirm,
 NTooltip,
 NTag
} from 'naive-ui'
import {
 FolderOpenOutline,
 TrashOutline,
 CreateOutline,
 CheckmarkOutline,
 CloseOutline,
 TimeOutline
} from '@vicons/ionicons5'

// Props
const props = defineProps({
 visible: { type: Boolean, default: false },
 projects: { type: Array, default: () => [] },
 loading: { type: Boolean, default: false },
 currentProjectId: { type: Number, default: null }
})

// Emits
const emit = defineEmits(['update:visible', 'close', 'load', 'delete', 'rename', 'refresh'])

// 内联重命名状态
const editingProjectId = ref(null)
const editingName = ref('')
const isRenaming = ref(false)

// 计算属性
const drawerVisible = computed({
 get: () => props.visible,
 set: (value) => emit('update:visible', value)
})

// 监听 visible 变化，关闭时重置编辑状态
watch(() => props.visible, (newVal) => {
 if (!newVal) {
 cancelRename()
 }
})

// 格式化日期时间
const formatDateTime = (dateTime) => {
 if (!dateTime) return '-'
 const date = new Date(dateTime)
 return date.toLocaleString('zh-CN', {
 year: 'numeric',
 month: '2-digit',
 day: '2-digit',
 hour: '2-digit',
 minute: '2-digit'
 })
}

// 格式化相对时间
const formatRelativeTime = (dateTime) => {
 if (!dateTime) return ''
 const date = new Date(dateTime)
 const now = new Date()
 const diff = now - date
 const minutes = Math.floor(diff / 60000)
 const hours = Math.floor(diff / 3600000)
 const days = Math.floor(diff / 86400000)
 
 if (minutes < 1) return '刚刚'
 if (minutes < 60) return `${minutes}分钟前`
 if (hours < 24) return `${hours}小时前`
 if (days < 7) return `${days}天前`
 return formatDateTime(dateTime)
}

// 处理关闭抽屉
const handleClose = () => {
 cancelRename()
 emit('close')
 emit('update:visible', false)
}

// 处理加载项目
const handleLoad = (project) => {
 emit('load', project)
}

// 处理删除项目
const handleDelete = (project) => {
 emit('delete', project)
}

// 开始重命名 - Requirements 3.7
const startRename = (project) => {
 editingProjectId.value = project.id
 editingName.value = project.name
}

// 确认重命名
const confirmRename = async (project) => {
 const newName = editingName.value.trim()
 if (!newName) {
 cancelRename()
 return
 }
 if (newName === project.name) {
 cancelRename()
 return
 }
 
 isRenaming.value = true
 try {
 await emit('rename', project, newName)
 } finally {
 isRenaming.value = false
 cancelRename()
 }
}

// 取消重命名
const cancelRename = () => {
 editingProjectId.value = null
 editingName.value = ''
}

// 处理重命名输入框按键
const handleRenameKeydown = (e, project) => {
 if (e.key === 'Enter') {
 confirmRename(project)
 } else if (e.key === 'Escape') {
 cancelRename()
 }
}

// 刷新项目列表
const handleRefresh = () => {
 emit('refresh')
}
</script>

<template>
 <n-drawer
 v-model:show="drawerVisible"
 :width="400"
 placement="right"
 :mask-closable="true"
 @update:show="(val) => !val && handleClose()"
 >
 <n-drawer-content title="项目管理" closable>
 <template #header>
 <n-space align="center" justify="space-between" style="width: 100%">
 <span>项目管理</span>
 <n-button
 quaternary
 circle
 size="small"
 :loading="loading"
 @click="handleRefresh"
 >
 <template #icon>
 <n-icon><FolderOpenOutline /></n-icon>
 </template>
 </n-button>
 </n-space>
 </template>

 <!-- 加载状态 -->
 <n-spin :show="loading" description="加载中...">
 <!-- 空状态 -->
 <n-empty
 v-if="!loading && projects.length === 0"
 description="暂无保存的项目"
 style="margin-top: 60px"
 >
 <template #icon>
 <n-icon size="48" :depth="3">
 <FolderOpenOutline />
 </n-icon>
 </template>
 </n-empty>

 <!-- 项目列表 - Requirements 3.5, 3.6 -->
 <n-list v-else hoverable clickable>
 <n-list-item
 v-for="project in projects"
 :key="project.id"
 :class="{ 'current-project': project.id === currentProjectId }"
 >
 <n-thing>
 <!-- 项目名称（支持内联编辑）- Requirements 3.7 -->
 <template #header>
 <div class="project-header">
 <!-- 编辑模式 -->
 <template v-if="editingProjectId === project.id">
 <n-input
 v-model:value="editingName"
 size="small"
 placeholder="项目名称"
 autofocus
 :loading="isRenaming"
 @keydown="(e) => handleRenameKeydown(e, project)"
 style="flex: 1; margin-right: 8px"
 />
 <n-space :size="4">
 <n-button
 quaternary
 circle
 size="tiny"
 type="success"
 :loading="isRenaming"
 @click="confirmRename(project)"
 >
 <template #icon>
 <n-icon><CheckmarkOutline /></n-icon>
 </template>
 </n-button>
 <n-button
 quaternary
 circle
 size="tiny"
 :disabled="isRenaming"
 @click="cancelRename"
 >
 <template #icon>
 <n-icon><CloseOutline /></n-icon>
 </template>
 </n-button>
 </n-space>
 </template>
 <!-- 显示模式 -->
 <template v-else>
 <span class="project-name">{{ project.name }}</span>
 <n-tag
 v-if="project.id === currentProjectId"
 size="small"
 type="success"
 round
 >
 当前
 </n-tag>
 </template>
 </div>
 </template>

 <!-- 最后修改时间 - Requirements 3.6 -->
 <template #description>
 <n-space align="center" :size="4" class="project-meta">
 <n-icon size="14" :depth="3">
 <TimeOutline />
 </n-icon>
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-text depth="3" style="font-size: 12px">
 {{ formatRelativeTime(project.updateTime) }}
 </n-text>
 </template>
 {{ formatDateTime(project.updateTime) }}
 </n-tooltip>
 </n-space>
 </template>

 <!-- 操作按钮 - Requirements 3.6 -->
 <template #header-extra>
 <n-space :size="4" v-if="editingProjectId !== project.id">
 <!-- 加载按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 size="small"
 type="primary"
 @click.stop="handleLoad(project)"
 >
 <template #icon>
 <n-icon><FolderOpenOutline /></n-icon>
 </template>
 </n-button>
 </template>
 加载项目
 </n-tooltip>

 <!-- 重命名按钮 -->
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 size="small"
 @click.stop="startRename(project)"
 >
 <template #icon>
 <n-icon><CreateOutline /></n-icon>
 </template>
 </n-button>
 </template>
 重命名
 </n-tooltip>

 <!-- 删除按钮 -->
 <n-popconfirm
 @positive-click="handleDelete(project)"
 positive-text="删除"
 negative-text="取消"
 >
 <template #trigger>
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button
 quaternary
 circle
 size="small"
 type="error"
 @click.stop
 >
 <template #icon>
 <n-icon><TrashOutline /></n-icon>
 </template>
 </n-button>
 </template>
 删除项目
 </n-tooltip>
 </template>
 确定要删除项目 "{{ project.name }}" 吗？此操作不可恢复。
 </n-popconfirm>
 </n-space>
 </template>
 </n-thing>
 </n-list-item>
 </n-list>
 </n-spin>
 </n-drawer-content>
 </n-drawer>
</template>

<style scoped>
.project-header {
 display: flex;
 align-items: center;
 gap: 8px;
 min-height: 24px;
}

.project-name {
 font-weight: 500;
 color: var(--n-text-color-1);
}

.project-meta {
 margin-top: 4px;
}

.current-project {
 background-color: var(--n-item-color-hover);
 border-left: 3px solid var(--n-primary-color);
}

:deep(.n-list-item) {
 padding: 12px 16px;
 transition: all 0.2s ease;
}

:deep(.n-list-item:hover) {
 background-color: var(--n-item-color-hover);
}

:deep(.n-thing-header) {
 margin-bottom: 0;
}

:deep(.n-thing-header-wrapper) {
 align-items: center;
}
</style>
