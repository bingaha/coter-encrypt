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
  useMessage
} from 'naive-ui'
import {
  ArrowBackOutline,
  MoonOutline,
  PlayOutline,
  StopOutline,
  SunnyOutline,
  GitMergeOutline,
  OpenOutline,
  SaveOutline,
  TrashOutline,
  ChevronDownOutline,
  ChevronUpOutline
} from '@vicons/ionicons5'
import { listen } from '@tauri-apps/api/event'
import { useConfigStore } from '@/store'
import { invokeApi } from '@/api/tauriClient'
import {
  loadMergeMonitorConfig,
  saveMergeMonitorConfig,
  startMergeMonitor,
  stopMergeMonitor,
  getMergeMonitorSnapshot,
  clearMergeMonitorLogs,
  openMergeRequestPage,
  listMergeMonitorRepositories
} from '@/api/mergeMonitor'

const router = useRouter()
const configStore = useConfigStore()
const message = useMessage()

const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())

const loading = ref(false)
const saving = ref(false)
const clearingLogs = ref(false)
const configExpanded = ref(false)
const snapshot = ref({
  running: false,
  todoCount: 0,
  current: null,
  todos: [],
  repositories: [],
  logs: []
})
const form = ref({
  token: '',
  orgId: '',
  listPollIntervalSecs: 30,
  aiPollIntervalSecs: 10,
  allowedAuthors: [],
  repositories: []
})
const authorsText = ref('')
const repoPickerVisible = ref(false)
const repoPickerLoading = ref(false)
const repoPickerSearch = ref('')
const remoteRepositories = ref([])
const selectedRepoIds = ref([])
let unlistenState = null
let pollTimer = null

const running = computed(() => !!snapshot.value.running)
const current = computed(() => snapshot.value.current)
const todos = computed(() => snapshot.value.todos || [])
const logs = computed(() => [...(snapshot.value.logs || [])].reverse())
const filteredRemoteRepositories = computed(() => {
  const keyword = String(repoPickerSearch.value || '')
    .trim()
    .toLowerCase()
  const list = remoteRepositories.value || []
  if (!keyword) return list
  return list.filter((item) => {
    const name = String(item.name || '').toLowerCase()
    const path = String(item.pathWithNamespace || '').toLowerCase()
    const id = String(item.id || '').toLowerCase()
    return name.includes(keyword) || path.includes(keyword) || id.includes(keyword)
  })
})
const selectedRepoCount = computed(() => selectedRepoIds.value.length)

const levelLabel = (level) => {
  const map = {
    info: '信息',
    warn: '警告',
    error: '错误',
    debug: '调试'
  }
  return map[level] || level
}

const applySnapshot = (data) => {
  if (!data) return
  snapshot.value = data
}

const loadAll = async () => {
  loading.value = true
  try {
    const [configRes, snapRes] = await Promise.all([
      loadMergeMonitorConfig(),
      getMergeMonitorSnapshot()
    ])
    const config = configRes.data
    form.value = {
      token: config.token || '',
      orgId: config.orgId || '',
      listPollIntervalSecs: config.listPollIntervalSecs ?? 30,
      aiPollIntervalSecs: config.aiPollIntervalSecs ?? 10,
      allowedAuthors: config.allowedAuthors || [],
      repositories: (config.repositories || []).map((item) => ({ ...item }))
    }
    authorsText.value = (config.allowedAuthors || []).join('\n')
    applySnapshot(snapRes.data)
  } catch (error) {
    message.error(error?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

const buildConfigPayload = () => {
  const authors = authorsText.value
    .split(/[\n,，]/)
    .map((item) => item.trim())
    .filter(Boolean)
  return {
    token: form.value.token.trim(),
    orgId: form.value.orgId.trim(),
    listPollIntervalSecs: Number(form.value.listPollIntervalSecs) || 30,
    aiPollIntervalSecs: Number(form.value.aiPollIntervalSecs) || 10,
    allowedAuthors: authors,
    repositories: form.value.repositories.map((item) => ({
      name: (item.name || '').trim(),
      repositoryId: (item.repositoryId || '').trim(),
      enabled: !!item.enabled
    }))
  }
}

const handleSave = async () => {
  saving.value = true
  try {
    const { data } = await saveMergeMonitorConfig(buildConfigPayload())
    form.value.allowedAuthors = data.allowedAuthors || []
    authorsText.value = (data.allowedAuthors || []).join('\n')
    message.success('配置已保存（热更新）')
    const snap = await getMergeMonitorSnapshot()
    applySnapshot(snap.data)
  } catch (error) {
    message.error(error?.message || '保存失败')
  } finally {
    saving.value = false
  }
}

const validateConfig = () => {
  if (!String(form.value.token || '').trim()) {
    return '请先配置云效 Token'
  }
  if (!String(form.value.orgId || '').trim()) {
    return '请先配置组织 ID'
  }
  const repos = (form.value.repositories || []).filter(
    (item) => String(item.repositoryId || '').trim()
  )
  if (!repos.length) {
    return '请先配置仓库列表'
  }
  if (!repos.some((item) => item.enabled)) {
    return '请至少启用一个仓库'
  }
  return ''
}

const handleStart = async () => {
  const validationError = validateConfig()
  if (validationError) {
    configExpanded.value = true
    message.error(validationError)
    return
  }
  try {
    await handleSave()
    const { data } = await startMergeMonitor()
    applySnapshot(data)
    message.success('合并监控已启动')
  } catch (error) {
    message.error(error?.message || '启动失败')
  }
}

const handleStop = async () => {
  try {
    const { data } = await stopMergeMonitor()
    applySnapshot(data)
    message.success('监控已停止')
  } catch (error) {
    message.error(error?.message || '停止失败')
  }
}

const handleOpenTodo = async (todo) => {
  const detailUrl = todo?.detailUrl
  if (!detailUrl) {
    message.warning('合并请求链接为空')
    return
  }
  try {
    await openMergeRequestPage(detailUrl)
  } catch (error) {
    message.error(error?.message || '打开页面失败')
  }
}

const handleClearLogs = async () => {
  clearingLogs.value = true
  try {
    const { data } = await clearMergeMonitorLogs()
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

const removeRepository = (index) => {
  form.value.repositories.splice(index, 1)
}

const isRepoSelected = (id) => selectedRepoIds.value.includes(String(id))

const toggleRepoSelected = (id, checked) => {
  const key = String(id)
  if (checked) {
    if (!selectedRepoIds.value.includes(key)) {
      selectedRepoIds.value = [...selectedRepoIds.value, key]
    }
    return
  }
  selectedRepoIds.value = selectedRepoIds.value.filter((item) => item !== key)
}

const openRepoPicker = async () => {
  const token = String(form.value.token || '').trim()
  const orgId = String(form.value.orgId || '').trim()
  if (!token) {
    configExpanded.value = true
    message.error('请先配置云效 Token')
    return
  }
  if (!orgId) {
    configExpanded.value = true
    message.error('请先配置组织 ID')
    return
  }

  repoPickerVisible.value = true
  repoPickerLoading.value = true
  repoPickerSearch.value = ''
  remoteRepositories.value = []
  selectedRepoIds.value = (form.value.repositories || [])
    .map((item) => String(item.repositoryId || '').trim())
    .filter(Boolean)

  try {
    const { data } = await listMergeMonitorRepositories(token, orgId)
    remoteRepositories.value = Array.isArray(data) ? data : []
    if (!remoteRepositories.value.length) {
      message.warning('未获取到仓库，请确认 Token 与组织 ID')
    }
  } catch (error) {
    repoPickerVisible.value = false
    message.error(error?.message || '获取仓库列表失败')
  } finally {
    repoPickerLoading.value = false
  }
}

const confirmRepoPicker = () => {
  const enabledMap = new Map(
    (form.value.repositories || []).map((item) => [
      String(item.repositoryId || '').trim(),
      !!item.enabled
    ])
  )
  const selectedSet = new Set(selectedRepoIds.value.map((id) => String(id)))
  const next = (remoteRepositories.value || [])
    .filter((item) => selectedSet.has(String(item.id)))
    .map((item) => {
      const id = String(item.id)
      return {
        name: item.name || item.pathWithNamespace || id,
        repositoryId: id,
        enabled: enabledMap.has(id) ? enabledMap.get(id) : true
      }
    })

  // 保留远端未返回但仍已勾选的本地配置（防御性）
  for (const id of selectedSet) {
    if (next.some((item) => item.repositoryId === id)) continue
    const existing = (form.value.repositories || []).find(
      (item) => String(item.repositoryId || '').trim() === id
    )
    if (existing) {
      next.push({ ...existing })
    }
  }

  form.value.repositories = next
  repoPickerVisible.value = false
  message.success(next.length ? `已选择 ${next.length} 个仓库` : '已清空仓库列表')
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
    unlistenState = await listen('merge-monitor-state', (event) => {
      applySnapshot(event.payload)
    })
  } catch (error) {
    console.warn('listen failed', error)
  }
  pollTimer = setInterval(async () => {
    try {
      const snap = await getMergeMonitorSnapshot()
      applySnapshot(snap.data)
    } catch {
      // ignore background poll errors
    }
  }, 3000)
})

onBeforeUnmount(() => {
  if (unlistenState) unlistenState()
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
          <n-icon :size="22"><GitMergeOutline /></n-icon>
        </div>
        <div class="title-copy">
          <h1>合并监控</h1>
          <n-text depth="3">云效合并请求 AI 评审 · 后台轮询</n-text>
        </div>
      </div>
      <div class="right">
        <n-tag :type="running ? 'success' : 'default'" size="small">
          {{ running ? '监控中' : '已停止' }}
        </n-tag>
        <n-button
          v-if="!running"
          type="primary"
          :loading="loading"
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
          停止
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
              <span>列表轮询(秒)</span>
              <n-input-number v-model:value="form.listPollIntervalSecs" :min="5" :max="3600" style="width: 100%" />
            </label>
            <label>
              <span>AI 轮询(秒)</span>
              <n-input-number v-model:value="form.aiPollIntervalSecs" :min="3" :max="3600" style="width: 100%" />
            </label>
          </div>

          <div class="sub-block">
            <div class="sub-title">作者白名单（每行一个）</div>
            <n-input
              v-model:value="authorsText"
              type="textarea"
              :rows="4"
              placeholder="每行一个作者姓名"
            />
          </div>

          <div class="sub-block">
            <div class="sub-title">
              <span>仓库列表</span>
              <a class="token-link" href="#" @click.prevent="openRepoPicker">获取仓库列表</a>
            </div>
            <div v-if="!form.repositories.length" class="empty-inline">
              <n-text depth="3">尚未选择仓库，点击「获取仓库列表」勾选生效</n-text>
            </div>
            <div
              v-for="(item, index) in form.repositories"
              :key="item.repositoryId || index"
              class="repo-row"
            >
              <n-switch v-model:value="item.enabled" size="small" />
              <n-input v-model:value="item.name" placeholder="名称" />
              <n-input v-model:value="item.repositoryId" placeholder="Repository ID" />
              <n-button quaternary circle type="error" @click="removeRepository(index)">
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
          <div class="panel-title-copy">
            <strong>当前跟踪</strong>
            <n-text depth="3" class="panel-hint">等待 AI 评审完成，全局最多 1 条</n-text>
          </div>
        </div>
        <div v-if="!current" class="empty">
          <n-text depth="3">暂无等待 AI 评审的合并请求</n-text>
        </div>
        <div v-else class="tracking-card">
          <div class="card-heading">
            <h3>{{ current.title || '未命名合并请求' }}</h3>
            <n-tag type="warning" size="small">AI评审：进行中</n-tag>
          </div>
          <n-text depth="3">
            {{ current.repoName || '-' }}
            · !{{ current.localId }}
            · {{ current.authorName || '-' }}
          </n-text>
          <div class="tracking-actions">
            <n-button
              size="small"
              secondary
              :disabled="!current.detailUrl"
              @click="handleOpenTodo(current)"
            >
              <template #icon>
                <n-icon><OpenOutline /></n-icon>
              </template>
              打开
            </n-button>
          </div>
        </div>
      </section>

      <section class="panel todo-panel">
        <div class="panel-title">
          <div class="panel-title-copy">
            <strong>待办列表</strong>
            <n-text depth="3" class="panel-hint">AI 已完成，可打开查看；打开不会移除</n-text>
          </div>
          <n-tag v-if="snapshot.todoCount" type="warning" size="small">
            {{ snapshot.todoCount }}
          </n-tag>
        </div>

        <div v-if="!todos.length" class="empty">
          <n-text depth="3">暂无 AI 已完成的待办</n-text>
        </div>
        <div v-else class="todo-list">
          <article
            v-for="todo in todos"
            :key="`${todo.projectId}-${todo.localId}`"
            class="todo-card"
          >
            <div class="todo-meta">
              <div class="card-heading">
                <h3>{{ todo.title || '未命名合并请求' }}</h3>
                <n-tag type="success" size="small">AI评审：已完成</n-tag>
              </div>
              <n-text depth="3">
                {{ todo.repoName || '-' }}
                · !{{ todo.localId }}
                · {{ todo.authorName || '-' }}
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
      v-model:show="repoPickerVisible"
      preset="card"
      title="选择仓库"
      style="width: min(640px, calc(100vw - 32px))"
      :mask-closable="!repoPickerLoading"
    >
      <div class="repo-picker">
        <n-input
          v-model:value="repoPickerSearch"
          clearable
          placeholder="搜索名称 / 路径 / ID"
          :disabled="repoPickerLoading"
        />
        <n-text depth="3" class="repo-picker-hint">
          已选 {{ selectedRepoCount }} 个
          <template v-if="!repoPickerLoading"> · 共 {{ remoteRepositories.length }} 个</template>
        </n-text>
        <div v-if="repoPickerLoading" class="repo-picker-loading">
          <n-text depth="3">正在获取仓库列表...</n-text>
        </div>
        <div v-else-if="!filteredRemoteRepositories.length" class="repo-picker-loading">
          <n-text depth="3">没有匹配的仓库</n-text>
        </div>
        <div v-else class="repo-picker-list">
          <label
            v-for="item in filteredRemoteRepositories"
            :key="item.id"
            class="repo-picker-item"
          >
            <n-checkbox
              :checked="isRepoSelected(item.id)"
              @update:checked="(checked) => toggleRepoSelected(item.id, checked)"
            />
            <span class="repo-picker-meta">
              <strong>{{ item.name || item.id }}</strong>
              <n-text depth="3">
                {{ item.pathWithNamespace || '-' }} · ID {{ item.id }}
              </n-text>
            </span>
          </label>
        </div>
      </div>
      <template #footer>
        <div class="repo-picker-footer">
          <n-button :disabled="repoPickerLoading" @click="repoPickerVisible = false">
            取消
          </n-button>
          <n-button
            type="primary"
            :disabled="repoPickerLoading"
            @click="confirmRepoPicker"
          >
            确认生效
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
    linear-gradient(180deg, rgba(32, 128, 240, 0.08), transparent 240px),
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
  color: #2080f0;
  background: rgba(32, 128, 240, 0.12);
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

.panel-title-copy {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.panel-hint {
  font-size: 12px;
  line-height: 1.35;
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
  grid-template-columns: repeat(2, minmax(0, 1fr));
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
  color: #2080f0;
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

.empty-inline {
  margin-bottom: 8px;
}

.repo-picker {
  display: grid;
  gap: 10px;
}

.repo-picker-hint {
  font-size: 12px;
}

.repo-picker-loading {
  min-height: 160px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.repo-picker-list {
  max-height: min(420px, 55vh);
  overflow: auto;
  display: grid;
  gap: 6px;
  padding-right: 2px;
}

.repo-picker-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
}

.repo-picker-item:hover {
  background: rgba(32, 128, 240, 0.08);
}

.repo-picker-meta {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.repo-picker-meta strong {
  font-size: 13px;
  font-weight: 600;
}

.repo-picker-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.repo-row {
  display: grid;
  grid-template-columns: auto 1fr 1fr auto;
  gap: 8px;
  align-items: center;
  margin-bottom: 8px;
}

.tracking-card,
.todo-card {
  border: 1px solid rgba(32, 128, 240, 0.28);
  background: rgba(32, 128, 240, 0.06);
  border-radius: 12px;
  padding: 14px;
}

.tracking-card h3,
.todo-card h3 {
  margin: 0;
  font-size: 15px;
}

.card-heading {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 4px;
}

.tracking-actions {
  margin-top: 12px;
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
}

.todo-meta {
  min-width: 0;
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
  color: #2080f0;
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

  .todo-card {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
