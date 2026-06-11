<script setup>
import { computed, inject, onBeforeUnmount, onMounted, ref } from 'vue'
import {
 NButton,
 NIcon,
 NInput,
 NRadioButton,
 NRadioGroup,
 NText,
 NTooltip,
 NSpin,
 NTag,
 NPopconfirm,
 useMessage,
 useDialog
} from 'naive-ui'
import {
 ArrowBackOutline,
 ClipboardOutline,
 CopyOutline,
 MoonOutline,
 SwapHorizontalOutline,
 SunnyOutline,
 RefreshOutline,
 TrashOutline,
 TimeOutline
} from '@vicons/ionicons5'
import { useConfigStore } from '@/store'
import {
 transferOssKey,
 loadTransferHistory,
 deleteTransferRecord,
 clearTransferHistory
} from '@/api/ossTransfer'

const configStore = useConfigStore()
const message = useMessage()
const dialog = useDialog()

const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())

const directionOptions = [
 { label: '生产 → 测试', value: 'prod_to_test' },
 { label: '测试 → 生产', value: 'test_to_prod' }
]

const form = ref({
 ossKey: '',
 direction: 'prod_to_test'
})

const loading = ref(false)
const result = ref(null)
const records = ref([])
const copiedKey = ref('')
let copiedTimer = null

const canTransfer = computed(() => {
 return form.value.ossKey.trim().length > 0 && !loading.value
})

const directionLabel = computed(() => {
 return form.value.direction === 'prod_to_test' ? '生产 → 测试' : '测试 → 生产'
})

const directionTagType = (dir) => {
 return dir === 'prod_to_test' ? 'warning' : 'success'
}

const directionText = (dir) => {
 return dir === 'prod_to_test' ? '生产→测试' : '测试→生产'
}

// Group records by date (YYYY-MM-DD), sorted newest first
const groupedRecords = computed(() => {
 const groups = {}
 for (const record of records.value) {
 const date = record.timestamp.split('T')[0]
 if (!groups[date]) {
 groups[date] = []
 }
 groups[date].push(record)
 }
 // Sort dates descending
 return Object.entries(groups)
 .sort(([a], [b]) => b.localeCompare(a))
 .map(([date, items]) => ({
 date,
 items: items.sort((a, b) => b.timestamp.localeCompare(a.timestamp))
 }))
})

const formatTime = (timestamp) => {
 const parts = timestamp.split('T')
 return parts[1] || timestamp
}

const truncateKey = (key, maxLen = 30) => {
 if (!key) return ''
 return key.length > maxLen ? key.slice(0, maxLen) + '...' : key
}

const handleTransfer = async () => {
 const ossKey = form.value.ossKey.trim()

 if (!ossKey) {
 message.warning('请输入 OSS Key')
 return
 }

 loading.value = true
 result.value = null

 try {
 const response = await transferOssKey(ossKey, form.value.direction)
 result.value = response.data
 message.success('转换成功')
 await refreshHistory()
 } catch (error) {
 message.error(error?.message || '转换失败')
 } finally {
 loading.value = false
 }
}

const refreshHistory = async () => {
 try {
 const response = await loadTransferHistory()
 records.value = response.data || []
 } catch {
 // silent
 }
}

const handleDeleteRecord = async (id) => {
 try {
 const response = await deleteTransferRecord(id)
 records.value = response.data || []
 message.success('已删除')
 } catch (error) {
 message.error(error?.message || '删除失败')
 }
}

const handleClearAll = () => {
 dialog.warning({
 title: '确认清空',
 content: '确定要清空所有转换记录吗？此操作不可撤销。',
 positiveText: '清空',
 negativeText: '取消',
 onPositiveClick: async () => {
 try {
 await clearTransferHistory()
 records.value = []
 message.success('已清空所有记录')
 } catch (error) {
 message.error(error?.message || '清空失败')
 }
 }
 })
}

const copyText = async (text, key) => {
 if (!text) {
 message.warning('暂无可复制内容')
 return
 }

 try {
 await navigator.clipboard.writeText(text)
 copiedKey.value = key
 message.success('已复制')

 if (copiedTimer) {
 window.clearTimeout(copiedTimer)
 }

 copiedTimer = window.setTimeout(() => {
 copiedKey.value = ''
 copiedTimer = null
 }, 1600)
 } catch {
 message.error('复制失败')
 }
}

const resetForm = () => {
 form.value = {
 ossKey: '',
 direction: 'prod_to_test'
 }
 result.value = null
 copiedKey.value = ''
}

const handleToggleTheme = () => {
 toggleTheme()
}

onMounted(() => {
 refreshHistory()
})

onBeforeUnmount(() => {
 if (copiedTimer) {
 window.clearTimeout(copiedTimer)
 }
})
</script>

<template>
 <main class="oss-transfer-page">
 <header class="page-header">
 <div class="header-left">
 <n-tooltip trigger="hover">
 <template #trigger>
 <router-link class="icon-link" to="/">
 <n-icon><ArrowBackOutline /></n-icon>
 </router-link>
 </template>
 返回首页
 </n-tooltip>

 <div class="title-mark">
 <n-icon :size="23"><SwapHorizontalOutline /></n-icon>
 </div>

 <div class="title-copy">
 <h1>OSS Key 转换</h1>
 <n-text depth="3">在生产环境与测试环境之间同步文件</n-text>
 </div>
 </div>

 <div class="header-actions">
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button quaternary circle @click="resetForm">
 <template #icon>
 <n-icon><RefreshOutline /></n-icon>
 </template>
 </n-button>
 </template>
 重置
 </n-tooltip>

 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button quaternary circle @click="handleToggleTheme">
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
 </div>
 </header>

 <section class="page-shell">
 <!-- Top: form + result -->
 <div class="top-row">
 <section class="panel form-panel">
 <div class="panel-heading">
 <h2>转换参数</h2>
 <n-text depth="3">输入 OSS Key 并选择转换方向</n-text>
 </div>

 <div class="field-block">
 <span>转换方向</span>
 <n-radio-group v-model:value="form.direction">
 <n-radio-button
 v-for="option in directionOptions"
 :key="option.value"
 :value="option.value"
 >
 {{ option.label }}
 </n-radio-button>
 </n-radio-group>
 </div>

 <div class="field-block">
 <span>OSS Key</span>
 <n-input
 v-model:value="form.ossKey"
 type="textarea"
 placeholder="粘贴 OSS Key，例如：Y0ZTWk9nMkgxVW1KTmgrdjlmclkxMHk4c0RJY3UwTWl2amtxYXg1NWk3VT0="
 :autosize="{ minRows: 3, maxRows: 6 }"
 clearable
 />
 </div>

 <div class="action-row">
 <n-button
 type="primary"
 :disabled="!canTransfer"
 :loading="loading"
 @click="handleTransfer"
 >
 <template #icon>
 <n-icon><SwapHorizontalOutline /></n-icon>
 </template>
 {{ loading ? '转换中...' : '开始转换' }}
 </n-button>
 <n-text depth="3">{{ directionLabel }}</n-text>
 </div>
 </section>

 <section class="panel result-panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>转换结果</h2>
 <n-text v-if="result" depth="3">{{ result.fileName }} · {{ result.contentType }}</n-text>
 <n-text v-else depth="3">等待转换</n-text>
 </div>
 <n-button
 v-if="result"
 type="primary"
 secondary
 size="small"
 @click="copyText(result.newOssKey, 'osskey')"
 >
 <template #icon>
 <n-icon>
 <ClipboardOutline v-if="copiedKey === 'osskey'" />
 <CopyOutline v-else />
 </n-icon>
 </template>
 {{ copiedKey === 'osskey' ? '已复制' : '复制' }}
 </n-button>
 </div>

 <div v-if="loading" class="loading-area">
 <n-spin size="medium" />
 <n-text depth="3">正在查询、下载并上传文件...</n-text>
 </div>

 <div v-else-if="result" class="result-content">
 <div class="result-item">
 <span class="result-label">新 OSS Key</span>
 <pre class="code-box">{{ result.newOssKey }}</pre>
 </div>
 </div>

 <div v-else class="empty-area">
 <n-text depth="3">输入 OSS Key 并点击"开始转换"查看结果</n-text>
 </div>
 </section>
 </div>

 <!-- Bottom: history -->
 <section class="panel history-panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>转换历史</h2>
 <n-text depth="3">共 {{ records.length }} 条记录</n-text>
 </div>
 <n-popconfirm
 v-if="records.length > 0"
 positive-text="清空"
 negative-text="取消"
 @positive-click="handleClearAll"
 >
 <template #trigger>
 <n-button secondary size="small" type="error">
 <template #icon>
 <n-icon><TrashOutline /></n-icon>
 </template>
 清空所有
 </n-button>
 </template>
 确定要清空所有转换记录吗？
 </n-popconfirm>
 </div>

 <div v-if="records.length > 0" class="history-list">
 <div
 v-for="group in groupedRecords"
 :key="group.date"
 class="history-group"
 >
 <div class="group-header">
 <n-icon :size="14"><TimeOutline /></n-icon>
 <span>{{ group.date }}</span>
 <n-tag size="small" :bordered="false">{{ group.items.length }} 条</n-tag>
 </div>

 <div
 v-for="record in group.items"
 :key="record.id"
 class="history-item"
 >
 <div class="item-left">
 <span class="item-time">{{ formatTime(record.timestamp) }}</span>
 <n-tag
 :type="directionTagType(record.direction)"
 size="small"
 :bordered="false"
 >
 {{ directionText(record.direction) }}
 </n-tag>
 <span class="item-filename" :title="record.fileName">{{ record.fileName }}</span>
 </div>

 <div class="item-right">
 <n-tooltip trigger="hover">
 <template #trigger>
 <span class="item-key" :title="record.newOssKey">{{ truncateKey(record.newOssKey) }}</span>
 </template>
 {{ record.newOssKey }}
 </n-tooltip>

 <n-button
 quaternary
 size="tiny"
 @click="copyText(record.newOssKey, record.id)"
 >
 <template #icon>
 <n-icon>
 <ClipboardOutline v-if="copiedKey === record.id" />
 <CopyOutline v-else />
 </n-icon>
 </template>
 </n-button>

 <n-button
 quaternary
 size="tiny"
 type="error"
 @click="handleDeleteRecord(record.id)"
 >
 <template #icon>
 <n-icon><TrashOutline /></n-icon>
 </template>
 </n-button>
 </div>
 </div>
 </div>
 </div>

 <div v-else class="empty-area">
 <n-text depth="3">暂无转换记录</n-text>
 </div>
 </section>
 </section>
 </main>
</template>

<style scoped>
.oss-transfer-page {
 width: 100%;
 height: 100%;
 min-height: 0;
 display: flex;
 flex-direction: column;
 overflow: hidden;
 color: var(--n-text-color-1, #333639);
 background:
 linear-gradient(180deg, rgba(24, 160, 88, 0.08), transparent 280px),
 var(--n-body-color, #f5f7fa);
}

.page-header {
 height: 64px;
 flex: none;
 padding: 0 28px;
 display: flex;
 align-items: center;
 justify-content: space-between;
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
 background-color: color-mix(in srgb, var(--n-card-color, #ffffff) 88%, transparent);
}

.header-left,
.header-actions {
 min-width: 0;
 display: flex;
 align-items: center;
 gap: 12px;
}

.icon-link {
 width: 34px;
 height: 34px;
 flex: none;
 display: grid;
 place-items: center;
 border-radius: 6px;
 color: var(--n-text-color-2, #666666);
 transition: background-color 0.15s ease, color 0.15s ease;
}

.icon-link:hover {
 color: var(--n-primary-color, #18a058);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
}

.title-mark {
 width: 40px;
 height: 40px;
 flex: none;
 display: grid;
 place-items: center;
 border-radius: 8px;
 color: #ffffff;
 background-color: #18a058;
}

.title-copy {
 min-width: 0;
}

.title-copy h1 {
 margin-bottom: 2px;
 font-size: 18px;
 line-height: 1.2;
}

.page-shell {
 width: min(1280px, calc(100vw - 48px));
 margin: 0 auto;
 padding: 32px 0;
 flex: 1;
 min-height: 0;
 overflow-y: auto;
 overflow-x: hidden;
 display: grid;
 gap: 18px;
 align-items: start;
}

.top-row {
 display: grid;
 grid-template-columns: minmax(360px, 1fr) minmax(0, 1fr);
 gap: 18px;
 align-items: start;
}

.panel {
 min-width: 0;
 padding: 20px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 background-color: var(--n-card-color, #ffffff);
 box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.05));
 display: grid;
 gap: 16px;
}

.panel-heading {
 margin-bottom: 0;
}

.panel-heading h2 {
 font-size: 18px;
 line-height: 1.25;
}

.panel-heading-row {
 display: flex;
 align-items: flex-start;
 justify-content: space-between;
 gap: 12px;
}

.field-block {
 min-width: 0;
 display: grid;
 gap: 7px;
}

.field-block > span {
 font-size: 13px;
 font-weight: 600;
 color: var(--n-text-color-2, #666666);
}

.action-row {
 display: flex;
 align-items: center;
 flex-wrap: wrap;
 gap: 10px;
}

.loading-area {
 min-height: 120px;
 display: flex;
 flex-direction: column;
 align-items: center;
 justify-content: center;
 gap: 12px;
}

.result-content {
 display: grid;
 gap: 12px;
}

.result-item {
 display: grid;
 gap: 6px;
}

.result-label {
 font-size: 13px;
 font-weight: 600;
 color: var(--n-text-color-2, #666666);
}

.code-box {
 margin: 0;
 padding: 13px 14px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 overflow: auto;
 white-space: pre-wrap;
 overflow-wrap: anywhere;
 color: var(--n-text-color-1, #333639);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
 font-family: var(--font-family-mono, Consolas, monospace);
 font-size: 12px;
 line-height: 1.7;
 user-select: text;
}

.empty-area {
 min-height: 80px;
 display: grid;
 place-items: center;
 border: 1px dashed var(--n-border-color, #e0e0e6);
 border-radius: 8px;
}

/* History */
.history-list {
 display: grid;
 gap: 18px;
}

.history-group {
 display: grid;
 gap: 2px;
}

.group-header {
 display: flex;
 align-items: center;
 gap: 6px;
 padding: 6px 0;
 font-size: 13px;
 font-weight: 600;
 color: var(--n-text-color-2, #666666);
}

.history-item {
 display: flex;
 align-items: center;
 justify-content: space-between;
 gap: 12px;
 padding: 10px 12px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 6px;
 background-color: var(--n-card-color, #ffffff);
 transition: border-color 0.15s ease;
}

.history-item:hover {
 border-color: var(--n-primary-color, #18a058);
}

.item-left {
 min-width: 0;
 display: flex;
 align-items: center;
 gap: 8px;
 flex: 1;
}

.item-time {
 flex: none;
 font-family: var(--font-family-mono, Consolas, monospace);
 font-size: 12px;
 color: var(--n-text-color-3, #999999);
}

.item-filename {
 min-width: 0;
 overflow: hidden;
 text-overflow: ellipsis;
 white-space: nowrap;
 font-size: 12px;
 color: var(--n-text-color-2, #666666);
}

.item-right {
 flex: none;
 display: flex;
 align-items: center;
 gap: 4px;
}

.item-key {
 max-width: 200px;
 overflow: hidden;
 text-overflow: ellipsis;
 white-space: nowrap;
 font-family: var(--font-family-mono, Consolas, monospace);
 font-size: 12px;
 color: var(--n-text-color-2, #666666);
 cursor: default;
}

@media (max-width: 980px) {
 .top-row {
 grid-template-columns: 1fr;
 }
}

@media (max-width: 720px) {
 .page-header {
 height: 56px;
 padding: 0 16px;
 }

 .title-mark {
 width: 34px;
 height: 34px;
 }

 .title-copy h1 {
 font-size: 16px;
 }

 .title-copy :deep(.n-text) {
 display: none;
 }

 .page-shell {
 width: calc(100vw - 28px);
 padding: 20px 0;
 }

 .panel {
 padding: 16px;
 }

 .history-item {
 flex-direction: column;
 align-items: flex-start;
 gap: 8px;
 }

 .item-right {
 align-self: flex-end;
 }

 .item-key {
 max-width: 140px;
 }
}
</style>
