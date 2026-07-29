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
  KeyOutline,
  MoonOutline,
  PlayOutline,
  SaveOutline,
  SearchOutline,
  SunnyOutline,
  TrashOutline,
  ListOutline
} from '@vicons/ionicons5'
import { useConfigStore } from '@/store'
import { loadYunshengAuthToken } from '@/api/yunshengAuth'
import { useAppSettings } from '@/composables/useAppSettings'
import {
  BIZ_TYPE_OPTIONS,
  createDefaultSubscription,
  INS_CODE_OPTIONS,
  listOrderSubscribeAreas,
  clearOrderSubscribeResult,
  loadOrderSubscribeConfig,
  loadOrderSubscribeResult,
  ORDER_STATE_OPTIONS,
  runOrderSubscribeNow,
  saveOrderSubscribeConfig,
  searchOrderSubscribeOrgs,
  setOrderSubscribeAutoRun
} from '@/api/orderSubscribe'

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

const billMonthModeOptions = [
  { label: '当前月（跟随系统时间）', value: 'current' },
  { label: '固定月份', value: 'fixed' }
]

const isSubExpanded = (id) => expandedSubIds.value.has(String(id || ''))

const toggleSubExpanded = (id) => {
  const key = String(id || '')
  if (!key) return
  const next = new Set(expandedSubIds.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  expandedSubIds.value = next
}

const billMonthSummary = (sub) => {
  if (sub?.billMonthMode === 'fixed' && sub?.billMonth) {
    return `固定 ${sub.billMonth}`
  }
  return '当前月'
}

const countOf = (row, bizType) => Number(row?.counts?.[bizType]?.count) || 0
const isCountHot = (row, bizType) => !!row?.counts?.[bizType]?.highlighted

const renderBizCount = (row, bizType) => {
  const n = countOf(row, bizType)
  const subscribed = isCountHot(row, bizType)
  // 仅「已订阅且 > 0」高亮绿；已订阅且为 0 用正文黑色；未订阅灰色
  const hot = subscribed && n > 0
  return h(
    'span',
    {
      style: {
        color: hot ? '#0c7a43' : subscribed ? 'var(--n-text-color, #1f2225)' : '#9ca3af',
        fontWeight: hot ? '700' : '500',
        fontVariantNumeric: 'tabular-nums'
      }
    },
    String(n)
  )
}

const resultTableRows = computed(() => {
  const rows = []
  for (const item of snapshot.value.subscriptions || []) {
    const fallbackArea = item.areaName || item.configAreaName || '—'
    if (!item.success) {
      rows.push({
        key: `${item.subscriptionId}-err`,
        success: false,
        billMonth: item.billMonth || '—',
        areaName: fallbackArea,
        pending: 0,
        error: item.error || '查询失败',
        counts: Object.fromEntries(
          BIZ_TYPE_OPTIONS.map((opt) => [opt.value, { count: 0, highlighted: false }])
        )
      })
      continue
    }
    const areas = item.areas || []
    if (!areas.length) {
      rows.push({
        key: `${item.subscriptionId}-empty`,
        success: true,
        billMonth: item.billMonth || '—',
        areaName: fallbackArea,
        pending: Number(item.subscribedTotal) || 0,
        error: '',
        counts: Object.fromEntries(
          BIZ_TYPE_OPTIONS.map((opt) => [opt.value, { count: 0, highlighted: false }])
        )
      })
      continue
    }
    for (const area of areas) {
      const counts = {}
      let pending = 0
      for (const c of area.counts || []) {
        const bizType = c.bizType || c.biz_type
        const highlighted = !!(c.highlighted ?? c.isHighlighted)
        const count = Number(c.count) || 0
        counts[bizType] = { count, highlighted }
        if (highlighted) pending += count
      }
      for (const opt of BIZ_TYPE_OPTIONS) {
        if (!counts[opt.value]) {
          counts[opt.value] = { count: 0, highlighted: false }
        }
      }
      rows.push({
        key: `${item.subscriptionId}-${area.areaName}`,
        success: true,
        billMonth: item.billMonth || '—',
        areaName: area.areaName || fallbackArea,
        pending,
        error: '',
        counts
      })
    }
  }
  return rows
})

const resultTableColumns = computed(() => {
  const bizCols = BIZ_TYPE_OPTIONS.map((opt) => ({
    title: opt.label,
    key: opt.value,
    width: 88,
    align: 'right',
    render: (row) => renderBizCount(row, opt.value)
  }))
  return [
    {
      title: '地区',
      key: 'areaName',
      ellipsis: { tooltip: true },
      minWidth: 120
    },
    {
      title: '状态',
      key: 'success',
      width: 72,
      render: (row) =>
        h(
          NTag,
          {
            size: 'small',
            bordered: false,
            type: row.success ? 'success' : 'error',
            title: row.error || undefined
          },
          { default: () => (row.success ? '成功' : '失败') }
        )
    },
    {
      title: '月份',
      key: 'billMonth',
      width: 88
    },
    ...bizCols,
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
          String(row.pending ?? 0)
        )
    }
  ]
})

const orgCountOf = (sub) => (Array.isArray(sub?.orgAccounts) ? sub.orgAccounts.length : 0)

const orgDisplayText = (sub) => {
  const n = orgCountOf(sub)
  if (!sub?.areaName && !n) return ''
  if (!n) return sub.areaName || ''
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
    const { data } = await listOrderSubscribeAreas()
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

const loadAll = async () => {
  loading.value = true
  try {
    const [configRes, tokenRes, resultRes] = await Promise.all([
      loadOrderSubscribeConfig(),
      loadYunshengAuthToken().catch(() => ({ data: { cookies: '' } })),
      loadOrderSubscribeResult().catch(() => ({ data: null }))
    ])
    autoRunOnStartup.value = !!configRes.data?.autoRunOnStartup
    subscriptions.value = (configRes.data?.subscriptions || []).map((item) => ({
      ...createDefaultSubscription(),
      ...item,
      orgAccounts: Array.isArray(item.orgAccounts)
        ? item.orgAccounts.map((org) => ({
            orgAccountId: Number(org.orgAccountId) || 0,
            accountName: String(org.accountName || '').trim()
          }))
        : [],
      orderStates: Array.isArray(item.orderStates) ? [...item.orderStates] : [],
      bizTypes: Array.isArray(item.bizTypes) ? [...item.bizTypes] : [],
      insCodes: Array.isArray(item.insCodes) ? [...item.insCodes] : []
    }))
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
  running.value = true
  try {
    const { data } = await runOrderSubscribeNow()
    applySnapshot(data)
    const rows = data?.subscriptions || []
    const failedRows = rows.filter((item) => !item.success)
    if (failedRows.length > 0 && failedRows.length === rows.length) {
      const tip = failedRows[0]?.error || '全部订阅查询失败'
      message.error(`执行失败：${tip}`)
    } else if (failedRows.length > 0) {
      message.warning(
        `执行完成：待办 ${data?.total ?? 0}，${failedRows.length} 条订阅失败`
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
        await clearOrderSubscribeResult()
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
    const { data } = await setOrderSubscribeAutoRun(value)
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
    const { data } = await searchOrderSubscribeOrgs({
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
  if (!selectedOrgs.value.length) {
    message.warning('请至少选择一个主体')
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
  message.success(`已选择 ${sub.orgAccounts.length} 个主体`)
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
    billMonthMode: item.billMonthMode === 'fixed' ? 'fixed' : 'current',
    billMonth: String(item.billMonth || '').trim(),
    orderStates: Array.isArray(item.orderStates) ? item.orderStates.map(Number) : [],
    bizTypes: Array.isArray(item.bizTypes) ? [...item.bizTypes] : [],
    insCodes: Array.isArray(item.insCodes) ? item.insCodes.map(Number) : []
  }))
})

const handleSave = async () => {
  saving.value = true
  try {
    const { data } = await saveOrderSubscribeConfig(buildConfigPayload())
    autoRunOnStartup.value = !!data?.autoRunOnStartup
    subscriptions.value = (data?.subscriptions || []).map((item) => ({
      ...createDefaultSubscription(),
      ...item,
      orgAccounts: Array.isArray(item.orgAccounts) ? [...item.orgAccounts] : []
    }))
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
          <h1>后道订单订阅</h1>
          <n-text depth="3">配置订阅 · 手动执行 · 查看待办明细</n-text>
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
                （绿色=已订阅且有待办，黑色=已订阅为 0，灰色=未订阅；每订阅仅统计前 100 条订单）
              </template>
              <template v-else>尚未执行；保存订阅后点击「立即执行」</template>
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
          :scroll-x="980"
          :max-height="360"
        />
      </section>

      <section class="panel">
        <div class="panel-title">
          <div class="panel-title-copy">
            <strong>订阅列表</strong>
            <n-text depth="3" class="panel-hint">
              默认折叠为一行；展开后可改地区主体、筛选条件与订阅业务。停用订阅不参与执行。
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

        <article
          v-for="(sub, index) in subscriptions"
          :key="sub.id || index"
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
              <span class="sub-switch-wrap" @click.stop>
                <n-switch v-model:value="sub.enabled" size="small" />
              </span>
              <strong class="sub-name">{{ sub.areaName || `订阅 ${index + 1}` }}</strong>
              <n-tag size="small" :bordered="false">{{ orgCountOf(sub) }} 个主体</n-tag>
              <n-tag size="small" :bordered="false">{{ billMonthSummary(sub) }}</n-tag>
              <n-tag v-if="!sub.enabled" size="small" type="warning" :bordered="false">已禁用</n-tag>
            </div>
            <div class="sub-card-actions">
              <n-button
                quaternary
                circle
                type="error"
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
                    placeholder="点击选择地区与多个主体"
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
                <span>月份模式</span>
                <n-select
                  v-model:value="sub.billMonthMode"
                  :options="billMonthModeOptions"
                />
              </label>
              <label v-if="sub.billMonthMode === 'fixed'">
                <span>固定月份（YYYYMM）</span>
                <n-input v-model:value="sub.billMonth" placeholder="例如 202607" maxlength="6" />
              </label>
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

            <div class="filter-block">
              <div class="filter-title">订阅业务 · 业务类型（勾选计入待办，不含在缴）</div>
              <n-checkbox-group v-model:value="sub.bizTypes">
                <div class="check-grid">
                  <n-checkbox
                    v-for="opt in BIZ_TYPE_OPTIONS"
                    :key="opt.value"
                    :value="opt.value"
                    :label="opt.label"
                  />
                </div>
              </n-checkbox-group>
            </div>
          </div>
        </article>
      </section>
    </div>

    <n-modal
      v-model:show="orgPickerVisible"
      preset="card"
      title="选择主体"
      style="width: min(640px, 94vw)"
    >
      <div class="org-picker">
        <label>
          <span>地区</span>
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
            placeholder="主体名称（模糊搜索）"
            @keyup.enter="handleSearchOrgs"
          />
          <n-button type="primary" :loading="searchingOrgs" @click="handleSearchOrgs">
            搜索
          </n-button>
        </div>
        <n-text v-if="selectedOrgs.length" depth="3" class="selected-count">
          已选 {{ selectedOrgs.length }} 个主体
        </n-text>
        <div v-if="!orgHits.length" class="empty-inline">
          <n-text depth="3">选择地区并搜索后，勾选一个或多个主体</n-text>
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

.sub-card {
  border: 1px solid var(--n-border-color, #e0e0e6);
  border-radius: 12px;
  padding: 0;
  margin-bottom: 10px;
  display: grid;
  gap: 0;
  overflow: hidden;
}

.sub-card:last-child {
  margin-bottom: 0;
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
