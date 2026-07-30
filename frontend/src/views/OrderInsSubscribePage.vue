<script setup>
import { computed, h, inject, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import {
  NButton,
  NCheckbox,
  NCheckboxGroup,
  NDataTable,
  NIcon,
  NInput,
  NModal,
  NSelect,
  NSwitch,
  NTag,
  NText,
  useDialog,
  useMessage
} from 'naive-ui'
import {
  AddOutline,
  ArrowBackOutline,
  ChevronDownOutline,
  ChevronUpOutline,
  CopyOutline,
  KeyOutline,
  MenuOutline,
  MoonOutline,
  PlayOutline,
  SaveOutline,
  SearchOutline,
  SunnyOutline,
  TrashOutline,
  ListOutline
} from '@vicons/ionicons5'
import draggable from 'vuedraggable'
import { useConfigStore } from '@/store'
import { loadYunshengAuthToken } from '@/api/yunshengAuth'
import { useAppSettings } from '@/composables/useAppSettings'
import {
  ACCOUNT_STATUS_OPTIONS,
  BILL_MONTH_REL_OPTIONS,
  billMonthTokenLabel,
  createDefaultSubscription,
  currentYyyymm,
  isFixedBillMonth,
  INS_CODE_OPTIONS,
  listOrderInsSubscribeAreas,
  clearOrderInsSubscribeResult,
  loadOrderInsSubscribeConfig,
  loadOrderInsSubscribeResult,
  normalizeBillMonthToken,
  ORDER_STATE_OPTIONS,
  runOrderInsSubscribeNow,
  saveOrderInsSubscribeConfig,
  searchOrderInsSubscribeOrgs,
  setOrderInsSubscribeAutoRun
} from '@/api/orderInsSubscribe'

const router = useRouter()
const configStore = useConfigStore()
const message = useMessage()
const dialog = useDialog()

const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())

const loading = ref(false)
const saving = ref(false)
const searchingOrgs = ref(false)
const loadingAreas = ref(false)
const running = ref(false)
const clearingResult = ref(false)

const cookies = ref('')
const hasYunshengCookies = computed(() => Boolean(String(cookies.value || '').trim()))
const { openSettings, settingsVisible } = useAppSettings()
/** 启动软件时是否自动查询当日待办（默认关） */
const autoRunOnStartup = ref(false)

watch(settingsVisible, async (visible, wasVisible) => {
  if (wasVisible && !visible) {
    try {
      const { data } = await loadYunshengAuthToken()
      cookies.value = data?.cookies || ''
    } catch {
      /* keep */
    }
  }
})

const subscriptions = ref([])
const areas = ref([])
const snapshot = ref({
  executedAt: '',
  executedDate: '',
  total: 0,
  subscriptions: []
})

const hasSnapshot = computed(
  () => !!(snapshot.value?.executedAt || (snapshot.value?.subscriptions || []).length)
)

const orgPickerVisible = ref(false)
const orgPickerIndex = ref(-1)
const orgPickerAreaId = ref(null)
const orgPickerKeyword = ref('')
const orgHits = ref([])
/** 选择器内已勾选主体：{ orgAccountId, accountName }[] */
const selectedOrgs = ref([])
/** 展开的订阅 id；默认折叠，保证列表一行一条 */
const expandedSubIds = ref(new Set())

const billMonthRelOptions = BILL_MONTH_REL_OPTIONS
const accountStatusOptions = ACCOUNT_STATUS_OPTIONS

const isSubExpanded = (id) => expandedSubIds.value.has(String(id || ''))

const toggleSubExpanded = (id) => {
  const key = String(id || '')
  if (!key) return
  const next = new Set(expandedSubIds.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  expandedSubIds.value = next
}

const billMonthSelectValue = (stored) =>
  isFixedBillMonth(stored) ? '__fixed__' : normalizeBillMonthToken(stored)

const billMonthSummary = (sub) => {
  const a = billMonthTokenLabel(sub?.billMonth1)
  const b = billMonthTokenLabel(sub?.billMonth2)
  return a === b ? a : `${a}～${b}`
}

const onBillMonthKindChange = (sub, field, kind) => {
  if (kind === '__fixed__') {
    sub[field] = isFixedBillMonth(sub[field]) ? String(sub[field]).trim() : currentYyyymm()
  } else {
    sub[field] = kind || 'current'
  }
}

const accountStatusLabel = (status) => {
  const hit = ACCOUNT_STATUS_OPTIONS.find((item) => item.value === Number(status))
  return hit?.label || '未选办理类型'
}

const insCodeLabel = (code) => {
  const hit = INS_CODE_OPTIONS.find((item) => item.value === Number(code))
  return hit?.label || String(code)
}

const formatInsCodesFull = (codes) => {
  const list = Array.isArray(codes) ? codes.map(insCodeLabel).filter(Boolean) : []
  return list.length ? list.join('、') : '不限'
}

/** 执行结果固定分组列（与后端 InsGroupBreakdown 字段对齐） */
const RESULT_GROUP_COLUMNS = [
  { title: '公积金', field: 'gjj' },
  { title: '医保', field: 'medical' },
  { title: '工伤', field: 'injury' },
  { title: '采暖', field: 'heating' },
  { title: '养老', field: 'pension' },
  { title: '失业', field: 'unemployment' },
  { title: '其他', field: 'other' }
]

const normalizeGroupCell = (cell) => {
  if (!cell || typeof cell !== 'object') return { kind: 'dash' }
  const kind = String(cell.kind || 'dash')
  if (kind === 'count') return { kind: 'count', value: Number(cell.value) || 0 }
  if (kind === 'error') return { kind: 'error', message: String(cell.message || '查询失败') }
  return { kind: 'dash' }
}

const rowHasGroupError = (breakdown) =>
  RESULT_GROUP_COLUMNS.some(({ field }) => normalizeGroupCell(breakdown?.[field]).kind === 'error')

const renderGroupCell = (cell) => {
  const normalized = normalizeGroupCell(cell)
  if (normalized.kind === 'count') {
    return h(
      'span',
      { style: { fontVariantNumeric: 'tabular-nums' } },
      String(normalized.value)
    )
  }
  if (normalized.kind === 'error') {
    return h(
      'span',
      {
        style: { color: '#d03050', cursor: 'help' },
        title: normalized.message
      },
      '错误'
    )
  }
  return h('span', { style: { color: 'var(--n-text-color-3)' } }, '-')
}

const resultTableRows = computed(() =>
  (snapshot.value.subscriptions || []).map((item) => {
    const groupBreakdown = item.groupBreakdown || {}
    const success = !!item.success
    const partial = success && rowHasGroupError(groupBreakdown)
    return {
      key: String(item.subscriptionId || Math.random()),
      success,
      partial,
      billMonth: item.billMonth || '—',
      areaName: item.areaName || '—',
      orgCount: Number(item.orgCount) || 0,
      accountStatus: Number(item.accountStatus) || 0,
      accountStatusText: accountStatusLabel(item.accountStatus),
      insCodesText: formatInsCodesFull(item.insCodes),
      pending: Number(item.subscribedTotal) || 0,
      error: item.error || '',
      groupBreakdown
    }
  })
)

const resultTableColumns = computed(() => [
  {
    title: '地区',
    key: 'areaName',
    width: 88,
    ellipsis: { tooltip: true }
  },
  {
    title: '主体数',
    key: 'orgCount',
    width: 72,
    align: 'right',
    render: (row) => (row.orgCount > 0 ? String(row.orgCount) : '全部')
  },
  {
    title: '险种',
    key: 'insCodesText',
    minWidth: 120,
    ellipsis: { tooltip: true }
  },
  {
    title: '办理类型',
    key: 'accountStatusText',
    width: 88,
    ellipsis: { tooltip: true }
  },
  {
    title: '状态',
    key: 'success',
    width: 80,
    render: (row) => {
      const type = !row.success ? 'error' : row.partial ? 'warning' : 'success'
      const label = !row.success ? '失败' : row.partial ? '部分成功' : '成功'
      return h(
        NTag,
        {
          size: 'small',
          bordered: false,
          type,
          title: row.error || undefined
        },
        { default: () => label }
      )
    }
  },
  {
    title: '账单月',
    key: 'billMonth',
    width: 120,
    ellipsis: { tooltip: true }
  },
  ...RESULT_GROUP_COLUMNS.map(({ title, field }) => ({
    title,
    key: `group-${field}`,
    width: 64,
    align: 'right',
    render: (row) => renderGroupCell(row.groupBreakdown?.[field])
  })),
  {
    title: '待办',
    key: 'pending',
    width: 72,
    align: 'right',
    render: (row) =>
      h(
        'strong',
        {
          style: {
            color: row.success ? '#18a058' : undefined,
            fontVariantNumeric: 'tabular-nums'
          }
        },
        row.success ? String(row.pending ?? 0) : '—'
      )
  }
])

const orgCountOf = (sub) => (Array.isArray(sub?.orgAccounts) ? sub.orgAccounts.length : 0)

const orgDisplayText = (sub) => {
  const n = orgCountOf(sub)
  if (!sub?.areaName && !n) return ''
  if (!n) return `${sub.areaName || ''} · 全部主体`
  return `${sub.areaName || '未选地区'} · ${n} 个主体`
}

const isOrgSelected = (id) =>
  selectedOrgs.value.some((item) => item.orgAccountId === id)

const areaSelectOptions = computed(() =>
  (areas.value || []).map((item) => ({
    label: item.provinceName
      ? `${item.areaName}（${item.provinceName}）`
      : item.areaName,
    value: item.areaId
  }))
)

const orgPickerAreaOptions = areaSelectOptions

const handleToggleTheme = () => toggleTheme()

const goHome = () => {
  router.push({ name: 'Home' })
}

const areaNameOf = (areaId) => {
  const hit = (areas.value || []).find((item) => item.areaId === areaId)
  return hit?.areaName || ''
}

const loadAreas = async () => {
  loadingAreas.value = true
  try {
    const { data } = await listOrderInsSubscribeAreas()
    areas.value = data || []
  } catch (error) {
    message.error(error?.message || '加载地区失败')
  } finally {
    loadingAreas.value = false
  }
}

const applySnapshot = (data) => {
  snapshot.value = {
    executedAt: data?.executedAt || '',
    executedDate: data?.executedDate || '',
    total: Number(data?.total) || 0,
    subscriptions: Array.isArray(data?.subscriptions) ? data.subscriptions : []
  }
}

const normalizeSubscriptionItem = (item) => ({
  ...createDefaultSubscription(),
  ...item,
  orgAccounts: Array.isArray(item.orgAccounts)
    ? item.orgAccounts.map((org) => ({
        orgAccountId: Number(org.orgAccountId) || 0,
        accountName: String(org.accountName || '').trim()
      }))
    : [],
  billMonth1: normalizeBillMonthToken(item.billMonth1),
  billMonth2: normalizeBillMonthToken(item.billMonth2),
  accountStatus: [1, 2, 3, 4, 5].includes(Number(item.accountStatus))
    ? Number(item.accountStatus)
    : null,
  orderStates: Array.isArray(item.orderStates) ? [...item.orderStates] : [],
  insCodes: Array.isArray(item.insCodes) ? [...item.insCodes] : []
})

const loadAll = async () => {
  loading.value = true
  try {
    const [configRes, tokenRes, resultRes] = await Promise.all([
      loadOrderInsSubscribeConfig(),
      loadYunshengAuthToken().catch(() => ({ data: { cookies: '' } })),
      loadOrderInsSubscribeResult().catch(() => ({ data: null }))
    ])
    autoRunOnStartup.value = !!configRes.data?.autoRunOnStartup
    subscriptions.value = (configRes.data?.subscriptions || []).map(normalizeSubscriptionItem)
    expandedSubIds.value = new Set()
    cookies.value = tokenRes.data?.cookies || ''
    applySnapshot(resultRes.data)
  } catch (error) {
    message.error(error?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

const handleRunNow = async () => {
  for (const [index, sub] of subscriptions.value.entries()) {
    if (![1, 2, 3, 4, 5].includes(Number(sub.accountStatus))) {
      message.warning(`订阅 ${index + 1}：请选择办理类型`)
      return
    }
  }
  running.value = true
  try {
    const { data: saved } = await saveOrderInsSubscribeConfig(buildConfigPayload())
    autoRunOnStartup.value = !!saved?.autoRunOnStartup
    subscriptions.value = (saved?.subscriptions || []).map(normalizeSubscriptionItem)

    const { data } = await runOrderInsSubscribeNow()
    applySnapshot(data)
    const rows = data?.subscriptions || []
    const failedRows = rows.filter((item) => !item.success)
    const partialRows = rows.filter((item) => {
      if (!item.success) return false
      const breakdown = item.groupBreakdown || {}
      return RESULT_GROUP_COLUMNS.some(
        ({ field }) => normalizeGroupCell(breakdown[field]).kind === 'error'
      )
    })
    if (failedRows.length > 0 && failedRows.length === rows.length) {
      const tip = failedRows[0]?.error || '全部订阅查询失败'
      message.error(`执行失败：${tip}`)
    } else if (failedRows.length > 0 || partialRows.length > 0) {
      message.warning(
        `执行完成：待办 ${data?.total ?? 0}，${failedRows.length} 条失败` +
          (partialRows.length ? `，${partialRows.length} 条部分成功` : '')
      )
    } else {
      message.success(`执行完成：待办总数 ${data?.total ?? 0}`)
    }
  } catch (error) {
    message.error(error?.message || '执行失败')
  } finally {
    running.value = false
  }
}

const handleClearResult = () => {
  dialog.warning({
    title: '删除执行结果',
    content: '将清除已落盘的查询快照，首页待办会归零。确定删除？',
    positiveText: '删除',
    negativeText: '取消',
    onPositiveClick: async () => {
      clearingResult.value = true
      try {
        await clearOrderInsSubscribeResult()
        applySnapshot(null)
        message.success('执行结果已删除')
      } catch (error) {
        message.error(error?.message || '删除失败')
        throw error
      } finally {
        clearingResult.value = false
      }
    }
  })
}

const handleAutoRunToggle = async (value) => {
  const previous = autoRunOnStartup.value
  autoRunOnStartup.value = value
  try {
    const { data } = await setOrderInsSubscribeAutoRun(value)
    autoRunOnStartup.value = !!data?.autoRunOnStartup
    message.success(value ? '已开启启动时自动查询' : '已关闭启动时自动查询')
  } catch (error) {
    autoRunOnStartup.value = previous
    message.error(error?.message || '保存失败')
  }
}

const openYunshengSettings = async () => {
  try {
    const { data } = await loadYunshengAuthToken()
    cookies.value = data?.cookies || ''
  } catch {
    /* keep current */
  }
  openSettings('yunsheng')
}

const addSubscription = () => {
  const created = createDefaultSubscription()
  subscriptions.value.push(created)
  const next = new Set(expandedSubIds.value)
  next.add(String(created.id))
  expandedSubIds.value = next
}

const duplicateSubscription = (index) => {
  const source = subscriptions.value[index]
  if (!source) return
  const cloned = normalizeSubscriptionItem({
    ...JSON.parse(JSON.stringify(source)),
    id: createDefaultSubscription().id
  })
  subscriptions.value.splice(index + 1, 0, cloned)
  const next = new Set(expandedSubIds.value)
  next.add(String(cloned.id))
  expandedSubIds.value = next
  message.success('已复制订阅')
}

const removeSubscription = (index) => {
  subscriptions.value.splice(index, 1)
}

const openOrgPicker = (index) => {
  const sub = subscriptions.value[index]
  orgPickerIndex.value = index
  orgPickerAreaId.value = sub?.areaId > 0 ? sub.areaId : null
  orgPickerKeyword.value = ''
  orgHits.value = []
  selectedOrgs.value = Array.isArray(sub?.orgAccounts)
    ? sub.orgAccounts
        .filter((org) => Number(org.orgAccountId) > 0)
        .map((org) => ({
          orgAccountId: Number(org.orgAccountId),
          accountName: String(org.accountName || '').trim()
        }))
    : []
  orgPickerVisible.value = true
  if (!areas.value.length) {
    loadAreas()
  }
}

const onOrgPickerAreaChange = (value) => {
  orgPickerAreaId.value = value
  selectedOrgs.value = []
  orgHits.value = []
  orgPickerKeyword.value = ''
}

const handleSearchOrgs = async () => {
  // 未选地区：不调接口，表现得像搜不到
  if (!orgPickerAreaId.value) {
    orgHits.value = []
    return
  }
  searchingOrgs.value = true
  try {
    const { data } = await searchOrderInsSubscribeOrgs({
      areaId: orgPickerAreaId.value,
      accountName: String(orgPickerKeyword.value || '').trim(),
      pageNo: 1,
      pageSize: 50
    })
    orgHits.value = data || []
    if (!orgHits.value.length) {
      message.info('未找到匹配机构')
    }
  } catch (error) {
    message.error(error?.message || '搜索机构失败')
  } finally {
    searchingOrgs.value = false
  }
}

const toggleOrgHit = (hit) => {
  const id = Number(hit?.orgAccountId) || 0
  if (!id) return
  const index = selectedOrgs.value.findIndex((item) => item.orgAccountId === id)
  if (index >= 0) {
    selectedOrgs.value.splice(index, 1)
    return
  }
  selectedOrgs.value.push({
    orgAccountId: id,
    accountName: String(hit?.accountName || '').trim()
  })
}

const applyOrgPicker = () => {
  const index = orgPickerIndex.value
  if (index < 0 || index >= subscriptions.value.length) {
    orgPickerVisible.value = false
    return
  }
  if (!orgPickerAreaId.value) {
    message.warning('请先选择地区')
    return
  }
  const areaId = orgPickerAreaId.value
  const sub = subscriptions.value[index]
  sub.areaId = areaId
  sub.areaName = areaNameOf(areaId) || sub.areaName
  sub.orgAccounts = selectedOrgs.value.map((org) => ({
    orgAccountId: Number(org.orgAccountId) || 0,
    accountName: String(org.accountName || '').trim()
  }))
  orgPickerVisible.value = false
  message.success(
    sub.orgAccounts.length
      ? `已选择 ${sub.orgAccounts.length} 个主体`
      : '已选择地区（全部主体）'
  )
}

const buildConfigPayload = () => ({
  autoRunOnStartup: !!autoRunOnStartup.value,
  subscriptions: subscriptions.value.map((item) => ({
    id: String(item.id || '').trim(),
    enabled: !!item.enabled,
    areaId: Number(item.areaId) || 0,
    areaName: String(item.areaName || '').trim(),
    orgAccounts: Array.isArray(item.orgAccounts)
      ? item.orgAccounts.map((org) => ({
          orgAccountId: Number(org.orgAccountId) || 0,
          accountName: String(org.accountName || '').trim()
        }))
      : [],
    billMonth1: normalizeBillMonthToken(item.billMonth1),
    billMonth2: normalizeBillMonthToken(item.billMonth2),
    accountStatus: [1, 2, 3, 4, 5].includes(Number(item.accountStatus))
      ? Number(item.accountStatus)
      : 0,
    orderStates: Array.isArray(item.orderStates) ? item.orderStates.map(Number) : [],
    insCodes: Array.isArray(item.insCodes) ? item.insCodes.map(Number) : []
  }))
})

const handleSave = async () => {
  for (const [index, sub] of subscriptions.value.entries()) {
    if (![1, 2, 3, 4, 5].includes(Number(sub.accountStatus))) {
      message.warning(`订阅 ${index + 1}：请选择办理类型`)
      return
    }
  }
  saving.value = true
  try {
    const { data } = await saveOrderInsSubscribeConfig(buildConfigPayload())
    autoRunOnStartup.value = !!data?.autoRunOnStartup
    subscriptions.value = (data?.subscriptions || []).map(normalizeSubscriptionItem)
    message.success('所有订阅已保存')
  } catch (error) {
    message.error(error?.message || '保存失败')
  } finally {
    saving.value = false
  }
}

onMounted(async () => {
  await loadAll()
  // 地区列表依赖 token，失败不阻塞页面
  loadAreas()
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
          <n-icon :size="22"><ListOutline /></n-icon>
        </div>
        <div class="title-copy">
          <h1>后道险种订单订阅</h1>
          <n-text depth="3">配置订阅 · 手动执行 · 查看险种订单待办</n-text>
        </div>
      </div>
      <div class="right">
        <label class="auto-run-switch header-auto-run" @click.stop>
          <n-switch
            :value="autoRunOnStartup"
            size="small"
            @update:value="handleAutoRunToggle"
          />
          <span>启动时自动查询</span>
        </label>
        <n-button secondary @click="openYunshengSettings">
          <template #icon>
            <n-icon><KeyOutline /></n-icon>
          </template>
          云生
          <n-tag
            class="cookie-status-tag"
            :type="hasYunshengCookies ? 'success' : 'warning'"
            size="small"
            :bordered="false"
          >
            {{ hasYunshengCookies ? '已配置' : '未配置' }}
          </n-tag>
        </n-button>
        <n-button type="primary" :loading="running" :disabled="loading" @click="handleRunNow">
          <template #icon>
            <n-icon><PlayOutline /></n-icon>
          </template>
          立即执行
        </n-button>
        <n-button
          secondary
          :loading="clearingResult"
          :disabled="loading || !hasSnapshot"
          @click="handleClearResult"
        >
          <template #icon>
            <n-icon><TrashOutline /></n-icon>
          </template>
          删除结果
        </n-button>
        <n-button quaternary circle @click="handleToggleTheme">
          <template #icon>
            <n-icon>
              <MoonOutline v-if="!isDarkMode" />
              <SunnyOutline v-else />
            </n-icon>
          </template>
        </n-button>
      </div>
    </header>

    <div class="page-body">
      <section class="panel">
        <div class="panel-title">
          <div class="panel-title-copy">
            <strong>执行结果</strong>
            <n-text depth="3" class="panel-hint">
              <template v-if="hasSnapshot">
                上次执行 {{ snapshot.executedAt || '—' }} · 待办总数
                <span class="total-num">{{ snapshot.total }}</span>
                （有险种筛选时按组拆查；待办 = 成功分组合计）
              </template>
              <template v-else>尚未执行；点击「立即执行」会先保存全部订阅再查询</template>
            </n-text>
          </div>
        </div>

        <div v-if="!hasSnapshot" class="empty">
          <n-text depth="3">暂无结果快照</n-text>
        </div>

        <n-data-table
          v-else
          class="result-table"
          size="small"
          :bordered="false"
          :single-line="false"
          :columns="resultTableColumns"
          :data="resultTableRows"
          :row-key="(row) => row.key"
          :scroll-x="1280"
          :max-height="360"
        />
      </section>

      <section class="panel">
        <div class="panel-title">
          <div class="panel-title-copy">
            <strong>订阅列表</strong>
            <n-text depth="3" class="panel-hint">
              默认折叠为一行；左侧拖拽排序，可复制订阅。展开后改地区、主体（可空）、办理类型、账单月与筛选。停用订阅不参与执行。
            </n-text>
          </div>
          <div class="panel-title-actions">
            <n-button size="small" secondary :loading="saving" :disabled="loading" @click="handleSave">
              <template #icon>
                <n-icon><SaveOutline /></n-icon>
              </template>
              保存所有订阅
            </n-button>
            <n-button size="small" secondary :disabled="loading" @click="addSubscription">
              <template #icon>
                <n-icon><AddOutline /></n-icon>
              </template>
              新增订阅
            </n-button>
          </div>
        </div>

        <div v-if="!subscriptions.length" class="empty">
          <n-text depth="3">暂无订阅，点击「新增订阅」开始配置</n-text>
        </div>

        <draggable
          v-else
          v-model="subscriptions"
          item-key="id"
          handle=".sub-drag-handle"
          ghost-class="sub-card-ghost"
          chosen-class="sub-card-chosen"
          drag-class="sub-card-drag"
          :animation="180"
          class="sub-list"
        >
          <template #item="{ element: sub, index }">
            <article
              class="sub-card"
              :class="{ 'is-collapsed': !isSubExpanded(sub.id) }"
            >
              <div
                class="sub-card-head"
                role="button"
                tabindex="0"
                @click="toggleSubExpanded(sub.id)"
                @keydown.enter.prevent="toggleSubExpanded(sub.id)"
                @keydown.space.prevent="toggleSubExpanded(sub.id)"
              >
                <div class="sub-card-title">
                  <span
                    class="sub-drag-handle"
                    title="拖拽排序"
                    @click.stop
                    @keydown.stop
                  >
                    <n-icon :size="18"><MenuOutline /></n-icon>
                  </span>
                  <span class="sub-switch-wrap" @click.stop>
                    <n-switch v-model:value="sub.enabled" size="small" />
                  </span>
                  <strong class="sub-name">{{ sub.areaName || `订阅 ${index + 1}` }}</strong>
                  <n-tag size="small" :bordered="false">
                    {{ orgCountOf(sub) ? `${orgCountOf(sub)} 个主体` : '全部主体' }}
                  </n-tag>
                  <n-tag size="small" :bordered="false">{{ accountStatusLabel(sub.accountStatus) }}</n-tag>
                  <n-tag size="small" :bordered="false">{{ billMonthSummary(sub) }}</n-tag>
                  <n-tag v-if="!sub.enabled" size="small" type="warning" :bordered="false">已禁用</n-tag>
                </div>
                <div class="sub-card-actions">
                  <n-button
                    quaternary
                    circle
                    title="复制订阅"
                    @click.stop="duplicateSubscription(index)"
                  >
                    <template #icon>
                      <n-icon><CopyOutline /></n-icon>
                    </template>
                  </n-button>
                  <n-button
                    quaternary
                    circle
                    type="error"
                    title="删除订阅"
                    @click.stop="removeSubscription(index)"
                  >
                    <template #icon>
                      <n-icon><TrashOutline /></n-icon>
                    </template>
                  </n-button>
                  <n-button quaternary circle @click.stop="toggleSubExpanded(sub.id)">
                    <template #icon>
                      <n-icon>
                        <ChevronUpOutline v-if="isSubExpanded(sub.id)" />
                        <ChevronDownOutline v-else />
                      </n-icon>
                    </template>
                  </n-button>
                </div>
              </div>

              <div v-if="isSubExpanded(sub.id)" class="sub-card-body" @click.stop>
            <div class="form-grid">
              <label>
                <span>地区 / 主体</span>
                <div class="org-row">
                  <n-input
                    class="org-picker-input"
                    :value="orgDisplayText(sub)"
                    readonly
                    placeholder="点击选择地区；主体可多选或留空（全部主体）"
                    @click="openOrgPicker(index)"
                  />
                  <n-button secondary @click="openOrgPicker(index)">
                    <template #icon>
                      <n-icon><SearchOutline /></n-icon>
                    </template>
                    选择
                  </n-button>
                </div>
              </label>
              <label>
                <span>办理类型</span>
                <n-select
                  v-model:value="sub.accountStatus"
                  :options="accountStatusOptions"
                  placeholder="请选择办理类型"
                />
              </label>
              <div class="bill-month-field">
                <span class="bill-month-label">账单月</span>
                <div class="bill-month-range">
                  <n-select
                    class="bill-month-kind"
                    :value="billMonthSelectValue(sub.billMonth1)"
                    :options="billMonthRelOptions"
                    @update:value="(v) => onBillMonthKindChange(sub, 'billMonth1', v)"
                  />
                  <n-input
                    v-if="isFixedBillMonth(sub.billMonth1)"
                    v-model:value="sub.billMonth1"
                    class="bill-month-fixed"
                    placeholder="YYYYMM"
                    maxlength="6"
                  />
                  <span class="bill-month-tilde">～</span>
                  <n-select
                    class="bill-month-kind"
                    :value="billMonthSelectValue(sub.billMonth2)"
                    :options="billMonthRelOptions"
                    @update:value="(v) => onBillMonthKindChange(sub, 'billMonth2', v)"
                  />
                  <n-input
                    v-if="isFixedBillMonth(sub.billMonth2)"
                    v-model:value="sub.billMonth2"
                    class="bill-month-fixed"
                    placeholder="YYYYMM"
                    maxlength="6"
                  />
                </div>
              </div>
            </div>

            <div v-if="orgCountOf(sub)" class="selected-orgs">
              <n-text depth="3">已选主体：</n-text>
              <n-tag
                v-for="org in sub.orgAccounts"
                :key="org.orgAccountId"
                size="small"
                :bordered="false"
              >
                {{ org.accountName || `ID ${org.orgAccountId}` }}
              </n-tag>
            </div>
            <div v-else-if="sub.areaId" class="selected-orgs">
              <n-text depth="3">未选主体：将按该地区全部主体查询</n-text>
            </div>

            <div class="filter-block">
              <div class="filter-title">筛选条件 · 订单状态（空=不限）</div>
              <n-checkbox-group v-model:value="sub.orderStates">
                <div class="check-grid">
                  <n-checkbox
                    v-for="opt in ORDER_STATE_OPTIONS"
                    :key="opt.value"
                    :value="opt.value"
                    :label="opt.label"
                  />
                </div>
              </n-checkbox-group>
            </div>

            <div class="filter-block">
              <div class="filter-title">筛选条件 · 险种过滤（空=不限）</div>
              <n-checkbox-group v-model:value="sub.insCodes">
                <div class="check-grid dense">
                  <n-checkbox
                    v-for="opt in INS_CODE_OPTIONS"
                    :key="opt.value"
                    :value="opt.value"
                    :label="opt.label"
                  />
                </div>
              </n-checkbox-group>
            </div>
          </div>
            </article>
          </template>
        </draggable>
      </section>
    </div>

    <n-modal
      v-model:show="orgPickerVisible"
      preset="card"
      title="选择地区与主体"
      style="width: min(640px, 94vw)"
    >
      <div class="org-picker">
        <label>
          <span>地区（必选）</span>
          <n-select
            :value="orgPickerAreaId"
            :options="orgPickerAreaOptions"
            :loading="loadingAreas"
            filterable
            clearable
            placeholder="选择地区"
            @update:value="onOrgPickerAreaChange"
          />
        </label>
        <div class="org-search-row">
          <n-input
            v-model:value="orgPickerKeyword"
            clearable
            placeholder="主体名称（模糊搜索，可留空不选）"
            @keyup.enter="handleSearchOrgs"
          />
          <n-button type="primary" :loading="searchingOrgs" @click="handleSearchOrgs">
            搜索
          </n-button>
        </div>
        <n-text depth="3" class="selected-count">
          {{
            selectedOrgs.length
              ? `已选 ${selectedOrgs.length} 个主体`
              : '未选主体时表示该地区全部主体'
          }}
        </n-text>
        <div v-if="!orgHits.length" class="empty-inline">
          <n-text depth="3">选择地区后可搜索并勾选主体；也可不选主体直接确认</n-text>
        </div>
        <div v-else class="org-hit-list">
          <div
            v-for="hit in orgHits"
            :key="hit.orgAccountId"
            class="org-hit-item"
            :class="{ 'is-selected': isOrgSelected(hit.orgAccountId) }"
            role="button"
            tabindex="0"
            @click="toggleOrgHit(hit)"
            @keydown.enter.prevent="toggleOrgHit(hit)"
          >
            <n-checkbox
              :checked="isOrgSelected(hit.orgAccountId)"
              @click.stop
              @update:checked="() => toggleOrgHit(hit)"
            />
            <div class="org-hit-copy">
              <strong>{{ hit.accountName || '未命名主体' }}</strong>
              <n-text depth="3">
                ID {{ hit.orgAccountId }}
                <template v-if="hit.orderMonthGjj || hit.orderMonthSb">
                  · 月份 {{ Math.max(hit.orderMonthGjj || 0, hit.orderMonthSb || 0) }}
                </template>
              </n-text>
            </div>
          </div>
        </div>
      </div>
      <template #footer>
        <div class="modal-actions">
          <n-button @click="orgPickerVisible = false">取消</n-button>
          <n-button type="primary" @click="applyOrgPicker">确认选择</n-button>
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

.cookie-status-tag {
  margin-left: 6px;
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

.panel-title-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.auto-run-switch {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  white-space: nowrap;
  cursor: pointer;
  user-select: none;
}

.header-auto-run {
  margin-right: 4px;
  color: var(--n-text-color-2, #555);
}

.selected-orgs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  margin: 4px 0 8px;
}

.selected-count {
  font-size: 12px;
}

.panel-hint {
  font-size: 12px;
  line-height: 1.35;
}

.total-num {
  color: #18a058;
  font-weight: 700;
}

.empty {
  padding: 28px 8px;
  text-align: center;
}

.result-table {
  width: 100%;
}

.result-table :deep(.n-data-table-th) {
  white-space: nowrap;
}

.sub-list {
  display: grid;
  gap: 0;
}

.sub-card {
  border: 1px solid var(--n-border-color, #e0e0e6);
  border-radius: 12px;
  padding: 0;
  margin-bottom: 10px;
  display: grid;
  gap: 0;
  overflow: hidden;
  background: var(--n-card-color, #fff);
}

.sub-card:last-child {
  margin-bottom: 0;
}

.sub-card-ghost {
  opacity: 0.45;
}

.sub-card-chosen {
  box-shadow: 0 0 0 1px rgba(24, 160, 88, 0.35);
}

.sub-card-drag {
  opacity: 0.95;
}

.sub-card.is-collapsed .sub-card-head {
  min-height: 44px;
}

.sub-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 12px;
  cursor: pointer;
  user-select: none;
}

.sub-card-head:hover {
  background: rgba(24, 160, 88, 0.04);
}

.sub-card-title {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  flex: 1;
  pointer-events: none;
}

.sub-drag-handle {
  pointer-events: auto;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  color: var(--n-text-color-3, #9ca3af);
  cursor: grab;
  flex-shrink: 0;
}

.sub-drag-handle:active {
  cursor: grabbing;
}

.sub-drag-handle:hover {
  color: #18a058;
  background: rgba(24, 160, 88, 0.1);
}

.sub-switch-wrap {
  pointer-events: auto;
  display: inline-flex;
  align-items: center;
}

.sub-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}

.sub-card-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
  pointer-events: auto;
}

.sub-card-body {
  display: grid;
  gap: 12px;
  padding: 0 12px 14px;
  border-top: 1px solid var(--n-border-color, #e0e0e6);
  padding-top: 12px;
}

.org-picker-input {
  cursor: pointer;
}

.org-picker-input :deep(.n-input__input-el) {
  cursor: pointer;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.form-grid label,
.org-picker label {
  display: grid;
  gap: 6px;
  font-size: 13px;
}

.org-row {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
}

.bill-month-field {
  display: grid;
  gap: 6px;
  font-size: 13px;
  grid-column: 1 / -1;
}

.bill-month-label {
  font-size: 13px;
}

.bill-month-range {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.bill-month-kind {
  width: 118px;
  flex: 0 0 auto;
}

.bill-month-fixed {
  width: 110px;
  flex: 0 0 auto;
}

.bill-month-tilde {
  color: var(--n-text-color-3, #9ca3af);
  font-variant-numeric: tabular-nums;
}

.filter-block {
  display: grid;
  gap: 8px;
}

.filter-title {
  font-size: 13px;
  font-weight: 600;
}

.check-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 14px;
}

.check-grid.dense {
  gap: 6px 12px;
}

.org-picker {
  display: grid;
  gap: 12px;
}

.org-search-row {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 8px;
}

.org-hit-list {
  max-height: min(360px, 50vh);
  overflow: auto;
  display: grid;
  gap: 6px;
}

.org-hit-item {
  display: grid;
  grid-template-columns: 22px minmax(0, 1fr);
  align-items: center;
  column-gap: 10px;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid transparent;
  cursor: pointer;
}

.org-hit-item:hover {
  background: rgba(24, 160, 88, 0.06);
}

.org-hit-item.is-selected {
  border-color: rgba(24, 160, 88, 0.35);
  background: rgba(24, 160, 88, 0.1);
}

.org-hit-copy {
  display: grid;
  gap: 2px;
  min-width: 0;
}

.org-hit-copy strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.empty-inline {
  padding: 8px 0;
}

.modal-hint {
  display: block;
  margin-bottom: 12px;
  font-size: 13px;
}

.token-input {
  margin-top: 4px;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

@media (max-width: 720px) {
  .form-grid {
    grid-template-columns: 1fr;
  }

  .page-header {
    height: auto;
    padding: 12px 14px;
    flex-wrap: wrap;
  }
}
</style>
