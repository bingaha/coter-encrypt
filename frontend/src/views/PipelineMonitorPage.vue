<script setup>
import { computed, inject, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  NButton,
  NCheckbox,
  NIcon,
  NInput,
  NInputNumber,
  NModal,
  NSwitch,
  NTag,
  NText,
  NTooltip,
  useMessage
} from 'naive-ui'
import {
  ArrowBackOutline,
  MoonOutline,
  PlayOutline,
  StopOutline,
  SunnyOutline,
  GitNetworkOutline,
  OpenOutline,
  SaveOutline,
  AddOutline,
  TrashOutline,
  ChevronDownOutline,
  ChevronUpOutline,
  PulseOutline
} from '@vicons/ionicons5'
import { listen } from '@tauri-apps/api/event'
import { useConfigStore } from '@/store'
import { invokeApi } from '@/api/tauriClient'
import {
  loadPipelineMonitorConfig,
  savePipelineMonitorConfig,
  startPipelineMonitor,
  startPipelineMonitorSingle,
  stopPipelineMonitor,
  queryPipelineLatestRun,
  getPipelineMonitorSnapshot,
  respondPipelineMonitorAction,
  openPipelineRunPage,
  clearPipelineMonitorLogs
} from '@/api/pipelineMonitor'

const router = useRouter()
const configStore = useConfigStore()
const message = useMessage()

const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())

const loading = ref(false)
const saving = ref(false)
const acting = ref(false)
const clearingLogs = ref(false)
const configExpanded = ref(false)
const singleDialogVisible = ref(false)
const singleDialogLoading = ref(false)
const singleStarting = ref(false)
const singleTarget = ref({ pipelineId: '', pipelineName: '' })
const singleForm = ref({
  runId: '',
  statusText: '',
  triggerText: '',
  triggerUser: '',
  queryError: ''
})
const snapshot = ref({
  running: false,
  mode: 'idle',
  singlePipelineId: '',
  autoMode: false,
  pendingCount: 0,
  todoCount: 0,
  currentPending: null,
  todos: [],
  pipelines: [],
  logs: []
})
const form = ref({
  token: '',
  orgId: '',
  pollIntervalSecs: 30,
  idleLatestQueryIntervalSecs: 300,
  postActionRefreshDelaySecs: 5,
  trackedSourceBranch: '',
  allowedTriggerUsers: [],
  pipelines: [],
  autoMode: false
})
const usersText = ref('')
let unlistenState = null
let unlistenNotify = null
let pollTimer = null

const running = computed(() => !!snapshot.value.running)
const monitorMode = computed(() => snapshot.value.mode || (running.value ? 'loop' : 'idle'))
const currentPending = computed(() => snapshot.value.currentPending)
const todos = computed(() => snapshot.value.todos || [])
const logs = computed(() => [...(snapshot.value.logs || [])].reverse())
const stopButtonLabel = computed(() => {
  if (monitorMode.value === 'single') return '停止单次监控'
  if (monitorMode.value === 'loop') return '停止循环监控'
  return '停止'
})
const canStartLoop = computed(() => !running.value)
const canStartSingle = computed(() => !running.value)

const levelLabel = (level) => {
  const map = {
    info: '信息',
    warn: '警告',
    error: '错误',
    debug: '调试'
  }
  return map[level] || level
}

/** 流水线运行状态色调：running / waiting / error / finished / idle */
const resolveRunStatusTone = (status) => {
  const value = String(status || '').trim().toUpperCase()
  if (!value) return 'idle'
  if (value === 'RUNNING') return 'running'
  if (value === 'WAITING' || value === 'SWITCH_MANUAL') return 'waiting'
  if (value === 'FAIL' || value === 'CANCELED' || value === 'FAILED' || value === 'ERROR') {
    return 'error'
  }
  if (value === 'SUCCESS') return 'finished'
  return 'idle'
}

const applySnapshot = (data) => {
  if (!data) return
  snapshot.value = data
  if (typeof data.autoMode === 'boolean') {
    form.value.autoMode = data.autoMode
  }
}

const loadAll = async () => {
  loading.value = true
  try {
    const [configRes, snapRes] = await Promise.all([
      loadPipelineMonitorConfig(),
      getPipelineMonitorSnapshot()
    ])
    const config = configRes.data
    form.value = {
      token: config.token || '',
      orgId: config.orgId || '',
      pollIntervalSecs: config.pollIntervalSecs ?? 30,
      idleLatestQueryIntervalSecs: config.idleLatestQueryIntervalSecs ?? 300,
      postActionRefreshDelaySecs: config.postActionRefreshDelaySecs ?? 5,
      trackedSourceBranch: config.trackedSourceBranch || '',
      allowedTriggerUsers: config.allowedTriggerUsers || [],
      pipelines: (config.pipelines || []).map((item) => ({ ...item })),
      autoMode: !!config.autoMode
    }
    usersText.value = (config.allowedTriggerUsers || []).join('\n')
    applySnapshot(snapRes.data)
  } catch (error) {
    message.error(error?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

const buildConfigPayload = () => {
  const users = usersText.value
    .split(/[\n,，]/)
    .map((item) => item.trim())
    .filter(Boolean)
  return {
    token: form.value.token.trim(),
    orgId: form.value.orgId.trim(),
    pollIntervalSecs: Number(form.value.pollIntervalSecs) || 30,
    idleLatestQueryIntervalSecs: Number(form.value.idleLatestQueryIntervalSecs) || 300,
    postActionRefreshDelaySecs: Number(form.value.postActionRefreshDelaySecs) || 5,
    trackedSourceBranch: form.value.trackedSourceBranch.trim(),
    allowedTriggerUsers: users,
    pipelines: form.value.pipelines.map((item) => ({
      name: (item.name || '').trim(),
      pipelineId: (item.pipelineId || '').trim(),
      enabled: !!item.enabled
    })),
    autoMode: !!form.value.autoMode
  }
}

const handleSave = async () => {
  saving.value = true
  try {
    const { data } = await savePipelineMonitorConfig(buildConfigPayload())
    form.value.autoMode = !!data.autoMode
    message.success('配置已保存（热更新）')
    const snap = await getPipelineMonitorSnapshot()
    applySnapshot(snap.data)
  } catch (error) {
    message.error(error?.message || '保存失败')
  } finally {
    saving.value = false
  }
}

const validateLoopMonitorConfig = () => {
  if (!String(form.value.token || '').trim()) {
    return '请先配置云效 Token'
  }
  if (!String(form.value.orgId || '').trim()) {
    return '请先配置组织 ID'
  }
  if (!String(form.value.trackedSourceBranch || '').trim()) {
    return '请先配置跟踪分支'
  }
  const pipelines = (form.value.pipelines || []).filter(
    (item) => String(item.pipelineId || '').trim()
  )
  if (!pipelines.length) {
    return '请先配置流水线列表'
  }
  if (!pipelines.some((item) => item.enabled)) {
    return '请至少启用一条流水线'
  }
  return ''
}

const handleStart = async () => {
  const validationError = validateLoopMonitorConfig()
  if (validationError) {
    configExpanded.value = true
    message.error(validationError)
    return
  }
  try {
    await handleSave()
    const { data } = await startPipelineMonitor()
    applySnapshot(data)
    message.success('循环监控已启动')
  } catch (error) {
    message.error(error?.message || '启动失败')
  }
}

const handleStop = async () => {
  try {
    const { data } = await stopPipelineMonitor()
    applySnapshot(data)
    message.success('监控已停止')
  } catch (error) {
    message.error(error?.message || '停止失败')
  }
}

const openSingleDialog = async (item) => {
  if (!canStartSingle.value) {
    message.warning(
      monitorMode.value === 'loop' ? '循环监控运行中，请先停止' : '单次监控运行中，请先停止'
    )
    return
  }
  singleTarget.value = {
    pipelineId: item.pipelineId,
    pipelineName: item.pipelineName || item.pipelineId
  }
  singleForm.value = {
    runId: '',
    statusText: '',
    triggerText: '',
    triggerUser: '',
    queryError: ''
  }
  singleDialogVisible.value = true
  singleDialogLoading.value = true
  try {
    const { data } = await queryPipelineLatestRun(item.pipelineId)
    singleForm.value = {
      runId: data.runId || '',
      statusText: data.statusText || data.status || '',
      triggerText: data.triggerText || '',
      triggerUser: data.triggerUser || '',
      queryError: ''
    }
  } catch (error) {
    singleForm.value.queryError = error?.message || '查询最新运行失败'
  } finally {
    singleDialogLoading.value = false
  }
}

const handleStartSingle = async () => {
  const runId = String(singleForm.value.runId || '').trim()
  if (!runId) {
    message.warning('请输入要监控的运行 ID')
    return
  }
  singleStarting.value = true
  try {
    await handleSave()
    const { data } = await startPipelineMonitorSingle({
      pipelineId: singleTarget.value.pipelineId,
      runId
    })
    applySnapshot(data)
    singleDialogVisible.value = false
    message.success('单次监控已启动')
  } catch (error) {
    message.error(error?.message || '启动单次监控失败')
  } finally {
    singleStarting.value = false
  }
}

const handleAutoChange = async (value) => {
  form.value.autoMode = value
  try {
    await savePipelineMonitorConfig(buildConfigPayload())
    const snap = await getPipelineMonitorSnapshot()
    applySnapshot(snap.data)
  } catch (error) {
    message.error(error?.message || '更新自动模式失败')
  }
}

const handleAction = async (action, jobId = '') => {
  if (!currentPending.value) return
  acting.value = true
  try {
    const { data } = await respondPipelineMonitorAction({
      action,
      pendingId: currentPending.value.id,
      jobId
    })
    applySnapshot(data)
  } catch (error) {
    message.error(error?.message || '操作失败')
  } finally {
    acting.value = false
  }
}

const handleOpenRun = async (pipelineId, runId = '') => {
  if (!pipelineId) return
  try {
    await openPipelineRunPage(pipelineId, runId || '')
  } catch (error) {
    message.error(error?.message || '打开页面失败')
  }
}

const handleOpenTodo = async (todo) => {
  if (!todo?.pipelineId) {
    message.warning('流水线 ID 为空')
    return
  }
  try {
    await openPipelineRunPage(todo.pipelineId, todo.runId || '')
  } catch (error) {
    message.error(error?.message || '打开页面失败')
  }
}

const handleClearLogs = async () => {
  clearingLogs.value = true
  try {
    const { data } = await clearPipelineMonitorLogs()
    applySnapshot(data)
  } catch (error) {
    message.error(error?.message || '清空日志失败')
  } finally {
    clearingLogs.value = false
  }
}

const toggleConfigPanel = () => {
  configExpanded.value = !configExpanded.value
}

const TOKEN_HELP_URL = 'https://account-devops.aliyun.com/settings/personalAccessToken'
const ORG_HELP_URL = 'https://account-devops.aliyun.com/settings/joinedOrganizations'

const handleOpenHelpUrl = async (url) => {
  try {
    await invokeApi('open_external_url', { url })
  } catch (error) {
    message.error(error?.message || '打开页面失败')
  }
}

const addPipeline = () => {
  form.value.pipelines.push({
    name: '',
    pipelineId: '',
    enabled: true
  })
}

const removePipeline = (index) => {
  form.value.pipelines.splice(index, 1)
}

const goHome = () => {
  router.push({ name: 'Home' })
}

const handleToggleTheme = () => {
  toggleTheme()
}

onMounted(async () => {
  await loadAll()
  try {
    unlistenState = await listen('pipeline-monitor-state', (event) => {
      applySnapshot(event.payload)
    })
    unlistenNotify = await listen('pipeline-monitor-notify', (event) => {
      const payload = event.payload || {}
      if (typeof Notification !== 'undefined') {
        if (Notification.permission === 'granted') {
          new Notification(payload.title || '流水线待办', {
            body: payload.body || ''
          })
        } else if (Notification.permission !== 'denied') {
          Notification.requestPermission().then((permission) => {
            if (permission === 'granted') {
              new Notification(payload.title || '流水线待办', {
                body: payload.body || ''
              })
            }
          })
        }
      }
      message.info(payload.body || '有新的流水线待办')
    })
  } catch (error) {
    console.warn('listen failed', error)
  }
  pollTimer = setInterval(async () => {
    try {
      const snap = await getPipelineMonitorSnapshot()
      applySnapshot(snap.data)
    } catch {
      // ignore background poll errors
    }
  }, 3000)
})

onBeforeUnmount(() => {
  if (unlistenState) unlistenState()
  if (unlistenNotify) unlistenNotify()
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<template>
  <main class="page">
    <header class="page-header">
      <div class="left">
        <n-button quaternary circle @click="goHome">
          <template #icon>
            <n-icon><ArrowBackOutline /></n-icon>
          </template>
        </n-button>
        <div class="title-mark">
          <n-icon :size="22"><GitNetworkOutline /></n-icon>
        </div>
        <div class="title-copy">
          <h1>流水线监控</h1>
          <n-text depth="3">云效人工卡点 / 分支选择 · 后台轮询</n-text>
        </div>
      </div>
      <div class="right">
        <n-tag
          :type="running ? 'success' : 'default'"
          size="small"
        >
          {{
            monitorMode === 'loop'
              ? '循环监控'
              : monitorMode === 'single'
                ? '单次监控'
                : '已停止'
          }}
        </n-tag>
        <n-checkbox
          :checked="form.autoMode"
          @update:checked="handleAutoChange"
        >
          自动模式
        </n-checkbox>
        <n-button
          v-if="!running"
          type="primary"
          :loading="loading"
          :disabled="!canStartLoop"
          @click="handleStart"
        >
          <template #icon>
            <n-icon><PlayOutline /></n-icon>
          </template>
          开启监控
        </n-button>
        <n-button
          v-else
          type="error"
          secondary
          @click="handleStop"
        >
          <template #icon>
            <n-icon><StopOutline /></n-icon>
          </template>
          {{ stopButtonLabel }}
        </n-button>
        <n-button quaternary circle @click="handleToggleTheme">
          <template #icon>
            <n-icon>
              <MoonOutline v-if="isDarkMode" />
              <SunnyOutline v-else />
            </n-icon>
          </template>
        </n-button>
      </div>
    </header>

    <div class="page-body">
      <section class="panel config-panel" :class="{ collapsed: !configExpanded }">
        <div class="panel-title config-title" @click="toggleConfigPanel">
          <div class="config-title-left">
            <n-icon :size="16">
              <ChevronUpOutline v-if="configExpanded" />
              <ChevronDownOutline v-else />
            </n-icon>
            <strong>配置</strong>
            <n-text v-if="!configExpanded" depth="3">已折叠，点击展开</n-text>
          </div>
          <n-button
            size="small"
            type="primary"
            secondary
            :loading="saving"
            @click.stop="handleSave"
          >
            <template #icon>
              <n-icon><SaveOutline /></n-icon>
            </template>
            保存并热更新
          </n-button>
        </div>

        <div v-if="configExpanded" class="config-body">
          <div class="form-grid">
            <label>
              <span class="field-label">
                Token
                <a class="token-link" href="#" @click.prevent="handleOpenHelpUrl(TOKEN_HELP_URL)">获取 Token</a>
              </span>
              <n-input v-model:value="form.token" type="password" show-password-on="click" placeholder="Yunxiao Token" />
            </label>
            <label>
              <span class="field-label">
                组织 ID
                <a class="token-link" href="#" @click.prevent="handleOpenHelpUrl(ORG_HELP_URL)">获取组织 ID</a>
              </span>
              <n-input v-model:value="form.orgId" placeholder="Org ID" />
            </label>
            <label>
              <span>跟踪分支</span>
              <n-input v-model:value="form.trackedSourceBranch" />
            </label>
            <label>
              <span>轮询间隔(秒)</span>
              <n-input-number v-model:value="form.pollIntervalSecs" :min="5" :max="3600" style="width: 100%" />
            </label>
            <label>
              <span>空闲重查(秒)</span>
              <n-input-number v-model:value="form.idleLatestQueryIntervalSecs" :min="30" :max="7200" style="width: 100%" />
            </label>
            <label>
              <span>操作后短刷(秒)</span>
              <n-input-number v-model:value="form.postActionRefreshDelaySecs" :min="1" :max="60" style="width: 100%" />
            </label>
          </div>

          <div class="sub-block">
            <div class="sub-title">触发人白名单（每行一个）</div>
            <n-input
              v-model:value="usersText"
              type="textarea"
              :rows="4"
              placeholder="每行一个触发人姓名"
            />
          </div>

          <div class="sub-block">
            <div class="sub-title">
              <span>流水线列表</span>
              <n-button size="tiny" secondary @click="addPipeline">
                <template #icon>
                  <n-icon><AddOutline /></n-icon>
                </template>
                添加
              </n-button>
            </div>
            <div
              v-for="(item, index) in form.pipelines"
              :key="index"
              class="pipeline-row"
            >
              <n-switch v-model:value="item.enabled" size="small" />
              <n-input v-model:value="item.name" placeholder="名称" />
              <n-input v-model:value="item.pipelineId" placeholder="Pipeline ID" />
              <n-button quaternary circle type="error" @click="removePipeline(index)">
                <template #icon>
                  <n-icon><TrashOutline /></n-icon>
                </template>
              </n-button>
            </div>
          </div>
        </div>
      </section>

      <section class="panel">
        <div class="panel-title">
          <strong>流水线状态</strong>
        </div>
        <div v-if="!snapshot.pipelines?.length" class="empty">
          <n-text depth="3">暂无流水线状态</n-text>
        </div>
        <div v-else class="status-grid">
          <article
            v-for="item in snapshot.pipelines"
            :key="item.pipelineId"
            class="status-card"
            :class="{ disabled: !item.enabled }"
          >
            <div class="status-head">
              <div>
                <h3>{{ item.pipelineName }}</h3>
                <n-text depth="3">ID: {{ item.pipelineId }}</n-text>
              </div>
              <n-tag size="small" :type="item.enabled ? 'info' : 'default'">
                {{ item.enabled ? '启用' : '停用' }}
              </n-tag>
            </div>
            <div class="status-body">
              <div>运行: {{ item.currentRunId || '-' }}</div>
              <div>
                状态:
                <span
                  class="run-status"
                  :class="`tone-${resolveRunStatusTone(item.currentRunStatus)}`"
                >
                  {{ item.currentRunStatusText || item.currentRunStatus || '-' }}
                </span>
              </div>
              <div>触发人: {{ item.triggerUser || '-' }}</div>
              <div class="summary">{{ item.summary || '—' }}</div>
            </div>
            <div class="status-actions">
              <n-button
                size="tiny"
                secondary
                :disabled="!item.pipelineId || !canStartSingle"
                @click="openSingleDialog(item)"
              >
                <template #icon>
                  <n-icon><PulseOutline /></n-icon>
                </template>
                单次监控
              </n-button>
              <n-button
                size="tiny"
                secondary
                :disabled="!item.pipelineId"
                @click="handleOpenRun(item.pipelineId, item.currentRunId)"
              >
                <template #icon>
                  <n-icon><OpenOutline /></n-icon>
                </template>
                打开云效
              </n-button>
            </div>
          </article>
        </div>
      </section>

      <section v-if="!form.autoMode" class="panel pending-panel">
        <div class="panel-title">
          <strong>当前待办</strong>
          <n-tag v-if="snapshot.pendingCount" type="warning" size="small">
            {{ snapshot.pendingCount }}
          </n-tag>
        </div>

        <div v-if="!currentPending" class="empty">
          <n-text depth="3">暂无待办。发现卡点后会在此显示（全局一次只处理一条）。</n-text>
        </div>
        <div v-else class="pending-card">
          <div class="pending-meta">
            <h3>
              {{ currentPending.kind === 'validate' ? '人工卡点审批' : '分支选择' }}
            </h3>
            <n-text depth="3">
              {{ currentPending.pipelineName }}#{{ currentPending.pipelineId }}
              · 运行 #{{ currentPending.runId }}
            </n-text>
            <div v-if="currentPending.kind === 'validate'" class="pending-detail">
              阶段：{{ currentPending.stageName }} · 状态：{{ currentPending.jobStatus }}
            </div>
          </div>

          <div class="pending-actions">
            <template v-if="currentPending.kind === 'validate'">
              <n-button
                v-if="currentPending.canApprove"
                type="primary"
                :loading="acting"
                @click="handleAction('pass')"
              >
                通过
              </n-button>
              <n-button
                v-if="currentPending.canApprove"
                type="error"
                secondary
                :loading="acting"
                @click="handleAction('refuse')"
              >
                拒绝
              </n-button>
            </template>
            <template v-else>
              <n-button
                v-for="candidate in currentPending.candidates"
                :key="candidate.jobId"
                type="primary"
                secondary
                :loading="acting"
                @click="handleAction('execute', candidate.jobId)"
              >
                {{ candidate.label }}
              </n-button>
            </template>
            <n-button secondary :loading="acting" @click="handleAction('open')">
              打开云效
            </n-button>
            <n-button quaternary :loading="acting" @click="handleAction('later')">
              稍后
            </n-button>
          </div>
        </div>
      </section>

      <section class="panel todo-panel">
        <div class="panel-title">
          <div class="panel-title-copy">
            <strong>待办列表</strong>
            <n-text depth="3" class="panel-hint">
              无审批权限等需手动处理的卡点；打开不会移除
            </n-text>
          </div>
          <n-tag v-if="snapshot.todoCount" type="warning" size="small">
            {{ snapshot.todoCount }}
          </n-tag>
        </div>

        <div v-if="!todos.length" class="empty">
          <n-text depth="3">暂无待办</n-text>
        </div>
        <div v-else class="todo-list">
          <article v-for="todo in todos" :key="todo.id" class="todo-card">
            <div class="todo-meta">
              <div class="card-heading">
                <h3>需手动审批</h3>
                <n-tag v-if="todo.reason === 'no_permission'" type="warning" size="small">
                  无审批权限
                </n-tag>
              </div>
              <n-text depth="3">
                {{ todo.pipelineName }}#{{ todo.pipelineId }}
                · 运行 #{{ todo.runId }}
                · {{ todo.stageName || '-' }}
              </n-text>
            </div>
            <n-button size="small" type="primary" secondary @click="handleOpenTodo(todo)">
              <template #icon>
                <n-icon><OpenOutline /></n-icon>
              </template>
              打开
            </n-button>
          </article>
        </div>
      </section>

      <section class="panel log-panel">
        <div class="panel-title">
          <strong>事件日志</strong>
          <n-button
            size="small"
            secondary
            :loading="clearingLogs"
            :disabled="!logs.length"
            @click="handleClearLogs"
          >
            <template #icon>
              <n-icon><TrashOutline /></n-icon>
            </template>
            清空
          </n-button>
        </div>
        <div class="log-list">
          <div v-if="!logs.length" class="empty">
            <n-text depth="3">暂无日志</n-text>
          </div>
          <div
            v-for="(entry, index) in logs"
            :key="`${entry.timestamp}-${index}`"
            class="log-item"
            :class="entry.level"
          >
            <span class="log-time">{{ entry.timestamp }}</span>
            <span class="log-level">{{ levelLabel(entry.level) }}</span>
            <span class="log-msg">{{ entry.message }}</span>
          </div>
        </div>
      </section>
    </div>

    <n-modal
      v-model:show="singleDialogVisible"
      preset="card"
      :title="`单次监控 · ${singleTarget.pipelineName || singleTarget.pipelineId}`"
      style="width: 480px"
      :mask-closable="!singleStarting"
    >
      <div v-if="singleDialogLoading" class="single-dialog-loading">
        <n-text depth="3">正在查询最新运行...</n-text>
      </div>
      <div v-else class="single-dialog-body">
        <n-text v-if="singleForm.queryError" type="error">
          {{ singleForm.queryError }}
        </n-text>
        <div v-else class="single-meta">
          <div>流水线 ID：{{ singleTarget.pipelineId }}</div>
          <div>运行状态：{{ singleForm.statusText || '-' }}</div>
          <div>触发方式：{{ singleForm.triggerText || '-' }}</div>
          <div>触发人：{{ singleForm.triggerUser || '-' }}</div>
        </div>
        <label class="single-run-field">
          <span>运行 ID</span>
          <n-input
            v-model:value="singleForm.runId"
            placeholder="可直接确认最新运行，或手动输入运行 ID"
          />
        </label>
      </div>
      <template #footer>
        <div class="single-dialog-footer">
          <n-button :disabled="singleStarting" @click="singleDialogVisible = false">
            取消
          </n-button>
          <n-button
            type="primary"
            :loading="singleStarting"
            :disabled="singleDialogLoading || !String(singleForm.runId || '').trim()"
            @click="handleStartSingle"
          >
            开始单次监控
          </n-button>
        </div>
      </template>
    </n-modal>
  </main>
</template>

<style scoped>
.page {
  width: 100%;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  color: var(--n-text-color-1, #333639);
  background:
    linear-gradient(180deg, rgba(24, 160, 88, 0.08), transparent 240px),
    var(--n-body-color, #f5f7fa);
}

.page-header {
  height: 64px;
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border-bottom: 1px solid var(--n-border-color, #e0e0e6);
  background-color: color-mix(in srgb, var(--n-card-color, #ffffff) 88%, transparent);
  flex-shrink: 0;
}

.left,
.right {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.title-mark {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  color: #18a058;
  background: rgba(24, 160, 88, 0.12);
}

.title-copy h1 {
  margin: 0;
  font-size: 18px;
  line-height: 1.2;
}

.page-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding: 16px 20px 28px;
  display: grid;
  gap: 16px;
  /* 高分辨率/大窗口下避免 auto 行被均分拉高，导致折叠配置栏仍占大块空白 */
  align-content: start;
}

.panel {
  border: 1px solid var(--n-border-color, #e0e0e6);
  border-radius: 14px;
  background: var(--n-card-color, #fff);
  padding: 14px 16px;
}

.panel-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}

.config-panel.collapsed {
  padding-top: 10px;
  padding-bottom: 10px;
}

.config-panel.collapsed .panel-title {
  margin-bottom: 0;
}

.config-panel.collapsed .config-body {
  display: none;
}

.config-title {
  cursor: pointer;
  user-select: none;
}

.config-title-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.form-grid label {
  display: grid;
  gap: 6px;
  font-size: 13px;
}

.field-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.token-link {
  color: #18a058;
  text-decoration: none;
  font-size: 12px;
  font-weight: 500;
}

.token-link:hover {
  text-decoration: underline;
}

.sub-block {
  margin-top: 14px;
}

.sub-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
  font-size: 13px;
  font-weight: 600;
}

.pipeline-row {
  display: grid;
  grid-template-columns: auto 1fr 1fr auto;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}

.status-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 12px;
}

.status-card {
  border: 1px solid var(--n-border-color, #e0e0e6);
  border-radius: 12px;
  padding: 12px;
  background: color-mix(in srgb, var(--n-card-color, #fff) 92%, #18a058 4%);
}

.status-card.disabled {
  opacity: 0.55;
}

.status-head {
  display: flex;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 8px;
}

.status-head h3 {
  margin: 0 0 2px;
  font-size: 15px;
}

.status-body {
  display: grid;
  gap: 4px;
  font-size: 13px;
  margin-bottom: 10px;
}

.run-status {
  font-weight: 600;
}

.run-status.tone-running {
  color: #18a058;
}

.run-status.tone-waiting {
  color: #f0a020;
}

.run-status.tone-error {
  color: #d03050;
}

.run-status.tone-finished {
  color: var(--n-text-color-1, #1f1f1f);
}

.run-status.tone-idle {
  color: var(--n-text-color-3, #888);
}

.summary {
  color: var(--n-text-color-3, #888);
  word-break: break-all;
}

.status-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}

.single-dialog-loading,
.single-dialog-body {
  display: grid;
  gap: 12px;
}

.single-meta {
  display: grid;
  gap: 6px;
  font-size: 13px;
  color: var(--n-text-color-2, #555);
}

.single-run-field {
  display: grid;
  gap: 6px;
  font-size: 13px;
}

.single-dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.pending-card {
  border: 1px solid rgba(240, 160, 32, 0.35);
  background: rgba(240, 160, 32, 0.08);
  border-radius: 12px;
  padding: 14px;
}

.pending-meta h3 {
  margin: 0 0 4px;
}

.pending-detail {
  margin-top: 6px;
  font-size: 13px;
}

.pending-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

.panel-title-copy {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.panel-hint {
  font-size: 12px;
}

.todo-list {
  display: grid;
  gap: 10px;
}

.todo-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border: 1px solid rgba(240, 160, 32, 0.35);
  background: rgba(240, 160, 32, 0.08);
  border-radius: 12px;
  padding: 12px 14px;
}

.todo-meta {
  min-width: 0;
}

.todo-card .card-heading {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 4px;
}

.todo-card h3 {
  margin: 0;
  font-size: 15px;
}

.log-list {
  max-height: 280px;
  overflow: auto;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 12px;
}

.log-item {
  display: grid;
  grid-template-columns: 72px 56px 1fr;
  gap: 8px;
  padding: 4px 0;
  border-bottom: 1px dashed var(--n-border-color, #eee);
}

.log-item.error .log-level {
  color: #d03050;
}

.log-item.warn .log-level {
  color: #f0a020;
}

.log-item.info .log-level {
  color: #18a058;
}

.log-time {
  color: var(--n-text-color-3, #999);
}

.empty {
  padding: 18px 4px;
}

@media (max-width: 960px) {
  .form-grid {
    grid-template-columns: 1fr;
  }

  .page-header {
    height: auto;
    padding: 12px;
    flex-direction: column;
    align-items: stretch;
  }

  .right {
    flex-wrap: wrap;
  }
}
</style>
