<script setup>
import { computed, inject, onBeforeUnmount, onMounted, ref } from 'vue'
import {
 NButton,
 NEmpty,
 NIcon,
 NInput,
 NRadioButton,
 NRadioGroup,
 NSwitch,
 NTag,
 NText,
 NTooltip,
 useMessage
} from 'naive-ui'
import {
 ArrowBackOutline,
 ClipboardOutline,
 CopyOutline,
 MoonOutline,
 ReceiptOutline,
 RefreshOutline,
 SearchOutline,
 SunnyOutline,
 TrashOutline
} from '@vicons/ionicons5'
import { useConfigStore } from '@/store'
import { queryRobotTaskFeedbackData } from '@/api/certQuery'
import { useMysqlDatasourceConfig } from '@/composables/useMysqlDatasourceConfig'

const configStore = useConfigStore()
const message = useMessage()

const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())
const {
 ensureReady,
 loadConfig
} = useMysqlDatasourceConfig()

const defaultMessages = {
 1: '成功',
 2: '系统异常'
}

const statusOptions = [
 { label: '成功', value: 1 },
 { label: '失败', value: 2 }
]

const form = ref({
 taskId: '',
 schemaName: 'platform_crawler',
 statusCondition: '',
 endpoint: 'http://localhost:8080/robotTask/handFeedBack',
 status: 1,
 msg: defaultMessages[1],
 excludeHasFeedbackEd: false,
 taskUserIdsText: '',
 taskUserInsIdsText: ''
})

const copiedKey = ref('')
const lastStatus = ref(form.value.status)
const querying = ref(false)
const queryResult = ref({
 taskUsers: [],
 taskUserIns: []
})
let copiedTimer = null

const normalizePositiveInteger = (value) => {
 const raw = String(value || '').trim()

 if (!/^\d+$/.test(raw)) {
 return ''
 }

 const normalized = raw.replace(/^0+/, '') || '0'
 return normalized === '0' ? '' : normalized
}

const parseIdList = (text) => {
 const source = String(text || '')
 const lines = source
 .split(/\r?\n/)
 .map(line => line.trim())
 .filter(Boolean)
 const looksLikeOnlyIds = /^[\s,，;；|\[\]()\d]+$/.test(source)
 const rawTokens = looksLikeOnlyIds || lines.length <= 1
 ? source.match(/\d+/g) || []
 : lines
 .map(line => line.match(/\d+/)?.[0])
 .filter(Boolean)
 const seen = new Set()
 const ids = []

 rawTokens.forEach(token => {
 const id = normalizePositiveInteger(token)

 if (id && !seen.has(id)) {
 seen.add(id)
 ids.push(id)
 }
 })

 return ids
}

const escapeSqlString = (value) => {
 return String(value ?? '').replace(/'/g, "''")
}

const escapePowerShellSingleQuoted = (value) => {
 return String(value ?? '').replace(/'/g, "''")
}

const normalizedStatusCondition = computed(() => normalizePositiveInteger(form.value.statusCondition))
const normalizedTaskId = computed(() => normalizePositiveInteger(form.value.taskId))
const hasTaskId = computed(() => Boolean(normalizedTaskId.value))
const schemaName = computed(() => form.value.schemaName.trim())
const hasValidSchemaName = computed(() => {
 return !schemaName.value || /^[A-Za-z0-9_]+$/.test(schemaName.value)
})
const endpoint = computed(() => form.value.endpoint.trim())
const hasEndpoint = computed(() => Boolean(endpoint.value))
const taskUserIds = computed(() => parseIdList(form.value.taskUserIdsText))
const taskUserInsIds = computed(() => parseIdList(form.value.taskUserInsIdsText))
const taskUserIdSet = computed(() => taskUserIds.value.join(','))
const taskUserInsIdSet = computed(() => taskUserInsIds.value.join(','))
const escapedSqlMsg = computed(() => escapeSqlString(form.value.msg))
const resultCountLabel = computed(() => {
 const taskUserCount = queryResult.value.taskUsers.length
 const taskUserInsCount = queryResult.value.taskUserIns.length

 return `${taskUserCount} 条人员记录，${taskUserInsCount} 条险种订单`
})

const taskIdStatus = computed(() => {
 return form.value.taskId.trim() && !hasTaskId.value ? 'error' : undefined
})

const schemaStatus = computed(() => {
 return hasValidSchemaName.value ? undefined : 'error'
})

const canGenerateSql = computed(() => hasTaskId.value && hasValidSchemaName.value)
const canGenerateCurl = computed(() => {
 return hasTaskId.value && hasEndpoint.value && taskUserIds.value.length > 0
})
const canQuery = computed(() => canGenerateSql.value && !querying.value)

const qualifiedTable = (tableName) => {
 return schemaName.value ? `${schemaName.value}.${tableName}` : tableName
}

const statusClause = computed(() => {
 return normalizedStatusCondition.value
 ? ` AND status = ${normalizedStatusCondition.value}`
 : ''
})

const querySql = computed(() => {
 if (!canGenerateSql.value) {
 return ''
 }

 return [
 `SELECT *`,
 `FROM ${qualifiedTable('robot_task_user')}`,
 `WHERE task_id = ${normalizedTaskId.value}${statusClause.value};`,
 '',
 `SELECT id, status, msg`,
 `FROM ${qualifiedTable('robot_task_user_ins')}`,
 `WHERE task_id = ${normalizedTaskId.value}${statusClause.value};`
 ].join('\n')
})

const updateSql = computed(() => {
 if (!canGenerateSql.value) {
 return ''
 }

 const userSql = taskUserIds.value.map(id => {
 return `UPDATE ${qualifiedTable('robot_task_user')} SET status = ${form.value.status}, msg = '${escapedSqlMsg.value}' WHERE id = ${id} AND task_id = ${normalizedTaskId.value}${statusClause.value};`
 })
 const insSql = taskUserInsIds.value.map(id => {
 return `UPDATE ${qualifiedTable('robot_task_user_ins')} SET status = ${form.value.status}, msg = '${escapedSqlMsg.value}' WHERE id = ${id} AND task_id = ${normalizedTaskId.value}${statusClause.value};`
 })
 const parts = []

 if (userSql.length) {
 parts.push('-- robot_task_user', ...userSql)
 }

 if (insSql.length) {
 if (parts.length) {
 parts.push('')
 }
 parts.push('-- robot_task_user_ins', ...insSql)
 }

 return parts.join('\n')
})

const curlPayload = computed(() => {
 if (!hasTaskId.value) {
 return ''
 }

 return [
 '{',
 `"excludeHasFeedbackEd":${form.value.excludeHasFeedbackEd},`,
 `"msg":${JSON.stringify(form.value.msg)},`,
 `"status":${form.value.status},`,
 `"taskId":${normalizedTaskId.value},`,
 `"userIdSet":[${taskUserIdSet.value}]`,
 '}'
 ].join('')
})

const curlCommand = computed(() => {
 if (!canGenerateCurl.value) {
 return ''
 }

 const body = escapePowerShellSingleQuoted(curlPayload.value)

 return [
 `curl.exe -X POST "${endpoint.value}"`,
 `-H "accept: */*"`,
 `-H "Content-Type: application/json"`,
 `-d '${body}'`
 ].join(' ')
})

const summaryItems = computed(() => [
 {
 label: 'task_id',
 value: normalizedTaskId.value || '-',
 type: hasTaskId.value ? 'success' : 'warning'
 },
 {
 label: 'task_user',
 value: `${taskUserIds.value.length}`,
 type: taskUserIds.value.length ? 'success' : 'default'
 },
 {
 label: 'task_user_ins',
 value: `${taskUserInsIds.value.length}`,
 type: taskUserInsIds.value.length ? 'success' : 'default'
 }
])

const handleStatusChange = (value) => {
 const currentMessage = form.value.msg.trim()

 if (!currentMessage || currentMessage === defaultMessages[lastStatus.value]) {
 form.value.msg = defaultMessages[value]
 }

 lastStatus.value = value
}

const joinIds = (rows) => {
 return rows.map(row => row.id).filter(Boolean).join('\n')
}

const handleQueryDatabase = async () => {
 if (!canGenerateSql.value) {
 message.warning('请先填写有效 task_id 和库名')
 return
 }

 const ready = await ensureReady()
 if (!ready) {
 message.warning('请先配置可用的 MySQL 数据源')
 return
 }

 querying.value = true
 copiedKey.value = ''
 try {
 const response = await queryRobotTaskFeedbackData({
 taskId: normalizedTaskId.value,
 schemaName: schemaName.value || null,
 statusCondition: normalizedStatusCondition.value || null
 })
 const data = response.data || {}
 const taskUsers = data.taskUsers || []
 const taskUserIns = data.taskUserIns || []

 queryResult.value = {
 taskUsers,
 taskUserIns
 }
 form.value.taskUserIdsText = joinIds(taskUsers)
 form.value.taskUserInsIdsText = joinIds(taskUserIns)

 if (!taskUsers.length && !taskUserIns.length) {
 message.info('未查询到对应记录')
 } else {
 message.success(`查询完成，${taskUsers.length} 条人员记录，${taskUserIns.length} 条险种订单`)
 }
 } catch (error) {
 message.error(error?.message || '查询任务反馈数据失败')
 } finally {
 querying.value = false
 }
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
 taskId: '',
 schemaName: 'platform_crawler',
 statusCondition: '',
 endpoint: 'http://localhost:8080/robotTask/handFeedBack',
 status: 1,
 msg: defaultMessages[1],
 excludeHasFeedbackEd: false,
 taskUserIdsText: '',
 taskUserInsIdsText: ''
 }
 lastStatus.value = 1
 copiedKey.value = ''
 queryResult.value = {
 taskUsers: [],
 taskUserIns: []
 }
}

const clearIds = () => {
 form.value.taskUserIdsText = ''
 form.value.taskUserInsIdsText = ''
 queryResult.value = {
 taskUsers: [],
 taskUserIns: []
 }
}

const handleToggleTheme = () => {
 toggleTheme()
}

onMounted(() => {
 loadConfig()
})

onBeforeUnmount(() => {
 if (copiedTimer) {
 window.clearTimeout(copiedTimer)
 }
})
</script>

<template>
 <main class="robot-feedback-page">
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
 <n-icon :size="23"><ReceiptOutline /></n-icon>
 </div>

 <div class="title-copy">
 <h1>机器人任务手动反馈</h1>
 <n-text depth="3">生成查询 SQL、更新 SQL 与 handFeedBack curl</n-text>
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
 <div class="form-area">
 <section class="panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>基础参数</h2>
 <n-text depth="3">按任务维度生成语句</n-text>
 </div>
 <div class="summary-tags">
 <n-tag
 v-for="item in summaryItems"
 :key="item.label"
 :type="item.type"
 size="small"
 >
 {{ item.label }}: {{ item.value }}
 </n-tag>
 </div>
 </div>

 <div class="base-grid">
 <label class="field-block">
 <span>task_id</span>
 <n-input
 v-model:value="form.taskId"
 :status="taskIdStatus"
 placeholder="0209425"
 clearable
 />
 </label>

 <label class="field-block">
 <span>库名</span>
 <n-input
 v-model:value="form.schemaName"
 :status="schemaStatus"
 placeholder="platform_crawler"
 clearable
 />
 </label>
 </div>

 <label class="field-block status-condition-field">
 <span>status 条件（可选）</span>
 <n-input
 v-model:value="form.statusCondition"
 placeholder="留空则不过滤"
 clearable
 />
 </label>

 <div class="query-actions">
 <n-button
 type="primary"
 :disabled="!canQuery"
 :loading="querying"
 @click="handleQueryDatabase"
 >
 <template #icon>
 <n-icon><SearchOutline /></n-icon>
 </template>
 按 task_id 查询数据库
 </n-button>
 <n-text depth="3">
 复用首页“数据库配置”中保存的 MySQL 数据源
 </n-text>
 </div>

 <div class="feedback-grid">
 <label class="field-block">
 <span>反馈结果</span>
 <n-radio-group
 v-model:value="form.status"
 @update:value="handleStatusChange"
 >
 <n-radio-button
 v-for="option in statusOptions"
 :key="option.value"
 :value="option.value"
 >
 {{ option.label }}
 </n-radio-button>
 </n-radio-group>
 </label>

 <label class="field-block">
 <span>排除已反馈订单</span>
 <div class="switch-row">
 <n-switch v-model:value="form.excludeHasFeedbackEd" />
 <n-text depth="3">
 {{ form.excludeHasFeedbackEd ? 'true' : 'false' }}
 </n-text>
 </div>
 </label>

 <label class="field-block message-field">
 <span>msg</span>
 <n-input
 v-model:value="form.msg"
 placeholder="成功"
 clearable
 />
 </label>
 </div>

 <label class="field-block endpoint-field">
 <span>接口地址</span>
 <n-input
 v-model:value="form.endpoint"
 placeholder="http://localhost:8080/robotTask/handFeedBack"
 clearable
 />
 </label>
 </section>

 <section class="panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>ID 输入</h2>
 <n-text depth="3">{{ resultCountLabel }}</n-text>
 </div>
 <n-button secondary size="small" @click="clearIds">
 <template #icon>
 <n-icon><TrashOutline /></n-icon>
 </template>
 清空 ID
 </n-button>
 </div>

 <div class="id-grid">
 <label class="field-block">
 <span>robot_task_user.id（自动查询，可手动修正）</span>
 <n-input
 v-model:value="form.taskUserIdsText"
 type="textarea"
 placeholder="13476221"
 :autosize="{ minRows: 5, maxRows: 10 }"
 />
 </label>

 <label class="field-block">
 <span>robot_task_user_ins.id（自动查询，可手动修正）</span>
 <n-input
 v-model:value="form.taskUserInsIdsText"
 type="textarea"
 placeholder="33595940&#10;33595941&#10;33595942"
 :autosize="{ minRows: 5, maxRows: 10 }"
 />
 </label>
 </div>

 <div
 v-if="queryResult.taskUsers.length || queryResult.taskUserIns.length"
 class="db-result-preview"
 >
 <div
 v-if="queryResult.taskUsers.length"
 class="preview-table"
 >
 <div class="preview-title">
 <strong>robot_task_user</strong>
 </div>
 <div class="preview-row preview-head">
 <span>id</span>
 <span>status</span>
 <span>msg</span>
 </div>
 <div
 v-for="row in queryResult.taskUsers"
 :key="`user-${row.id}`"
 class="preview-row"
 >
 <span>{{ row.id }}</span>
 <span>{{ row.status ?? '-' }}</span>
 <span>{{ row.msg || '-' }}</span>
 </div>
 </div>

 <div
 v-if="queryResult.taskUserIns.length"
 class="preview-table"
 >
 <div class="preview-title">
 <strong>robot_task_user_ins</strong>
 </div>
 <div class="preview-row preview-row-ins preview-head">
 <span>id</span>
 <span>task_user_id</span>
 <span>status</span>
 <span>feedback_ed</span>
 <span>msg</span>
 </div>
 <div
 v-for="row in queryResult.taskUserIns"
 :key="`ins-${row.id}`"
 class="preview-row preview-row-ins"
 >
 <span>{{ row.id }}</span>
 <span>{{ row.taskUserId || '-' }}</span>
 <span>{{ row.status ?? '-' }}</span>
 <span>{{ row.feedbackEd ?? '-' }}</span>
 <span>{{ row.msg || '-' }}</span>
 </div>
 </div>
 </div>
 </section>
 </div>

 <aside class="result-area">
 <section class="panel result-panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>查询 SQL</h2>
 <n-text depth="3">先按 task_id 查询两张表</n-text>
 </div>
 <n-button
 secondary
 size="small"
 :disabled="!querySql"
 @click="copyText(querySql, 'query')"
 >
 <template #icon>
 <n-icon>
 <ClipboardOutline v-if="copiedKey === 'query'" />
 <CopyOutline v-else />
 </n-icon>
 </template>
 {{ copiedKey === 'query' ? '已复制' : '复制' }}
 </n-button>
 </div>

 <pre v-if="querySql" class="code-box">{{ querySql }}</pre>
 <n-empty
 v-else
 class="empty-result"
 description="请输入 task_id"
 />
 </section>

 <section class="panel result-panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>更新 SQL</h2>
 <n-text depth="3">WHERE 同时限定 id 和 task_id</n-text>
 </div>
 <n-button
 secondary
 size="small"
 :disabled="!updateSql"
 @click="copyText(updateSql, 'update')"
 >
 <template #icon>
 <n-icon>
 <ClipboardOutline v-if="copiedKey === 'update'" />
 <CopyOutline v-else />
 </n-icon>
 </template>
 {{ copiedKey === 'update' ? '已复制' : '复制' }}
 </n-button>
 </div>

 <pre v-if="updateSql" class="code-box">{{ updateSql }}</pre>
 <n-empty
 v-else
 class="empty-result"
 description="请输入 task_id 和需要更新的 ID"
 />
 </section>

 <section class="panel result-panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>Curl</h2>
 <n-text depth="3">userIdSet 只放 robot_task_user.id</n-text>
 </div>
 <n-button
 type="primary"
 size="small"
 :disabled="!curlCommand"
 @click="copyText(curlCommand, 'curl')"
 >
 <template #icon>
 <n-icon>
 <ClipboardOutline v-if="copiedKey === 'curl'" />
 <CopyOutline v-else />
 </n-icon>
 </template>
 {{ copiedKey === 'curl' ? '已复制' : '复制' }}
 </n-button>
 </div>

 <pre v-if="curlCommand" class="code-box curl-box">{{ curlCommand }}</pre>
 <n-empty
 v-else
 class="empty-result"
 description="请输入 task_id 和 robot_task_user.id"
 />

 <div v-if="curlPayload" class="payload-preview">
 <div class="payload-title">
 <n-text depth="3">JSON 入参</n-text>
 <n-button
 quaternary
 size="tiny"
 :disabled="!curlPayload"
 @click="copyText(curlPayload, 'payload')"
 >
 <template #icon>
 <n-icon>
 <ClipboardOutline v-if="copiedKey === 'payload'" />
 <CopyOutline v-else />
 </n-icon>
 </template>
 {{ copiedKey === 'payload' ? '已复制' : '复制' }}
 </n-button>
 </div>
 <pre class="code-box payload-box">{{ curlPayload }}</pre>
 </div>
 </section>
 </aside>
 </section>
 </main>
</template>

<style scoped>
.robot-feedback-page {
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
 grid-template-columns: minmax(360px, 460px) minmax(0, 1fr);
 gap: 18px;
 align-items: start;
}

.form-area,
.result-area {
 min-width: 0;
 display: grid;
 gap: 18px;
}

.panel {
 min-width: 0;
 padding: 20px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 background-color: var(--n-card-color, #ffffff);
 box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.05));
}

.panel-heading {
 margin-bottom: 16px;
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

.summary-tags {
 flex: none;
 display: flex;
 flex-wrap: wrap;
 justify-content: flex-end;
 gap: 6px;
}

.base-grid,
.feedback-grid,
.id-grid {
 display: grid;
 gap: 14px;
}

.base-grid {
 grid-template-columns: minmax(0, 1fr) minmax(150px, 180px);
}

.status-condition-field {
 margin-top: 14px;
}

.query-actions {
 margin-top: 14px;
 display: flex;
 align-items: center;
 flex-wrap: wrap;
 gap: 10px;
}

.feedback-grid {
 margin-top: 14px;
 grid-template-columns: minmax(0, 1fr) minmax(150px, 180px);
}

.message-field,
.endpoint-field {
 grid-column: 1 / -1;
}

.endpoint-field {
 margin-top: 14px;
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

.switch-row {
 min-height: 34px;
 display: flex;
 align-items: center;
 gap: 10px;
}

.db-result-preview {
 margin-top: 16px;
 display: grid;
 gap: 12px;
}

.preview-table {
 min-width: 0;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 overflow: hidden;
}

.preview-title {
 padding: 9px 10px;
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
}

.preview-title strong {
 font-size: 13px;
}

.preview-row {
 display: grid;
 grid-template-columns: minmax(86px, 1fr) minmax(58px, 72px) minmax(0, 1.3fr);
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
}

.preview-row-ins {
 grid-template-columns:
 minmax(86px, 1fr)
 minmax(86px, 1fr)
 minmax(58px, 72px)
 minmax(78px, 90px)
 minmax(0, 1.3fr);
}

.preview-row:last-child {
 border-bottom: 0;
}

.preview-row span {
 min-width: 0;
 padding: 8px 9px;
 border-right: 1px solid var(--n-border-color, #e0e0e6);
 overflow: hidden;
 text-overflow: ellipsis;
 white-space: nowrap;
 font-family: var(--font-family-mono, Consolas, monospace);
 font-size: 12px;
}

.preview-row span:last-child {
 border-right: 0;
}

.preview-head {
 color: var(--n-text-color-2, #666666);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
}

.result-panel {
 display: grid;
 gap: 12px;
}

.result-panel .panel-heading {
 margin-bottom: 0;
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

.curl-box {
 min-height: 98px;
}

.payload-preview {
 display: grid;
 gap: 8px;
}

.payload-title {
 display: flex;
 align-items: center;
 justify-content: space-between;
 gap: 10px;
}

.payload-box {
 max-height: 150px;
}

.empty-result {
 min-height: 120px;
 display: grid;
 place-items: center;
 border: 1px dashed var(--n-border-color, #e0e0e6);
 border-radius: 8px;
}

@media (max-width: 980px) {
 .page-shell {
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

 .base-grid,
 .feedback-grid {
 grid-template-columns: 1fr;
 }

 .preview-table {
 overflow-x: auto;
 }

 .preview-row {
 min-width: 360px;
 }

 .preview-row-ins {
 min-width: 560px;
 }

 .panel-heading-row {
 flex-direction: column;
 }

 .summary-tags {
 justify-content: flex-start;
 }
}

@media (max-width: 420px) {
 .panel-heading-row :deep(.n-button),
 .payload-title :deep(.n-button) {
 width: 100%;
 }

 .payload-title {
 align-items: flex-start;
 flex-direction: column;
 }
}
</style>
