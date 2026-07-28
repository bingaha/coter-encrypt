<script setup>
import { computed, h, inject, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  NButton,
  NCheckbox,
  NCheckboxGroup,
  NDataTable,
  NIcon,
  NInput,
  NModal,
  NRadio,
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
import { loadYunshengAuthToken, saveYunshengAuthToken } from '@/api/yunshengAuth'
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
const savingToken = ref(false)
const searchingOrgs = ref(false)
const loadingAreas = ref(false)
const running = ref(false)
const clearingResult = ref(false)

const cookies = ref('')
const tokenModalVisible = ref(false)
/** 启动软件时是否自动查询当日待办（默认关） */
const autoRunOnStartup = ref(false)

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
const selectedOrgId = ref(null)
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
    if (!item.success) {
      rows.push({
        key: `${item.subscriptionId}-err`,
        accountName: item.accountName || item.subscriptionId || '—',
        success: false,
        billMonth: item.billMonth || '—',
        areaName: item.configAreaName || '—',
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
        accountName: item.accountName || item.subscriptionId || '—',
        success: true,
        billMonth: item.billMonth || '—',
        areaName: '—',
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
      // 保证六列都有值，避免缺字段时不高亮
      for (const opt of BIZ_TYPE_OPTIONS) {
        if (!counts[opt.value]) {
          counts[opt.value] = { count: 0, highlighted: false }
        }
      }
      rows.push({
        key: `${item.subscriptionId}-${area.areaName}`,
        accountName: item.accountName || item.subscriptionId || '—',
        success: true,
        billMonth: item.billMonth || '—',
        areaName: area.areaName || '—',
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
      title: '机构',
      key: 'accountName',
      ellipsis: { tooltip: true },
      minWidth: 180
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
    {
      title: '地区',
      key: 'areaName',
      width: 100,
      ellipsis: { tooltip: true }
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

const orgDisplayText = (sub) => {
  if (!sub?.accountName) return ''
  return sub.areaName ? `${sub.accountName} · ${sub.areaName}` : sub.accountName
}

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
      businessBillMonth: item.businessBillMonth || '',
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
    const failed = (data?.subscriptions || []).filter((item) => !item.success).length
    if (failed > 0) {
      message.warning(`执行完成：总数 ${data?.total ?? 0}，${failed} 条订阅失败`)
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

const openTokenModal = async () => {
  try {
    const { data } = await loadYunshengAuthToken()
    cookies.value = data?.cookies || ''
  } catch {
    /* keep current */
  }
  tokenModalVisible.value = true
}

const handleSaveToken = async () => {
  savingToken.value = true
  try {
    const { data } = await saveYunshengAuthToken({
      cookies: String(cookies.value || '').trim()
    })
    cookies.value = data?.cookies || ''
    message.success('Cookie 已保存')
    tokenModalVisible.value = false
  } catch (error) {
    message.error(error?.message || '保存 Cookie 失败')
  } finally {
    savingToken.value = false
  }
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
  selectedOrgId.value = sub?.orgAccountId > 0 ? sub.orgAccountId : null
  orgPickerVisible.value = true
  if (!areas.value.length) {
    loadAreas()
  }
}

const handleSearchOrgs = async () => {
  if (!orgPickerAreaId.value) {
    message.warning('请先选择地区')
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

const applyOrgPicker = () => {
  const index = orgPickerIndex.value
  if (index < 0 || index >= subscriptions.value.length) {
    orgPickerVisible.value = false
    return
  }
  const selectedId = selectedOrgId.value
  if (!selectedId) {
    message.warning('请选择一个机构')
    return
  }
  const hit = orgHits.value.find((item) => item.orgAccountId === selectedId)
  const areaId = orgPickerAreaId.value
  const sub = subscriptions.value[index]
  sub.orgAccountId = selectedId
  sub.accountName = hit?.accountName || sub.accountName
  sub.areaId = areaId
  sub.areaName = areaNameOf(areaId) || sub.areaName
  sub.businessBillMonth = String(hit?.businessBillMonth || '').trim()
  orgPickerVisible.value = false
  message.success('已选择机构')
}

const selectOrgHit = (id) => {
  selectedOrgId.value = id
}

const buildConfigPayload = () => ({
  autoRunOnStartup: !!autoRunOnStartup.value,
  subscriptions: subscriptions.value.map((item) => ({
    id: String(item.id || '').trim(),
    enabled: !!item.enabled,
    orgAccountId: Number(item.orgAccountId) || 0,
    accountName: String(item.accountName || '').trim(),
    areaId: Number(item.areaId) || 0,
    areaName: String(item.areaName || '').trim(),
    billMonthMode: item.billMonthMode === 'fixed' ? 'fixed' : 'current',
    billMonth: String(item.billMonth || '').trim(),
    businessBillMonth: String(item.businessBillMonth || '').trim(),
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
      ...item
    }))
    message.success('订阅配置已保存')
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
        <n-button secondary @click="openTokenModal">
          <template #icon>
            <n-icon><KeyOutline /></n-icon>
          </template>
          Cookie
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
        <n-button secondary :loading="saving" @click="handleSave">
          <template #icon>
            <n-icon><SaveOutline /></n-icon>
          </template>
          保存配置
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
                （绿色=已订阅且有待办，黑色=已订阅为 0，灰色=未订阅）
              </template>
              <template v-else>尚未执行；配置保存后点击「立即执行」</template>
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
          :scroll-x="1100"
          :max-height="360"
        />
      </section>

      <section class="panel">
        <div class="panel-title">
          <div class="panel-title-copy">
            <strong>订阅列表</strong>
            <n-text depth="3" class="panel-hint">
              默认折叠为一行；展开后可改机构与过滤条件。停用订阅不参与执行。
            </n-text>
          </div>
          <div class="panel-title-actions">
            <label class="auto-run-switch" @click.stop>
              <n-switch
                :value="autoRunOnStartup"
                size="small"
                @update:value="handleAutoRunToggle"
              />
              <span>启动时自动查询</span>
            </label>
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
              <strong class="sub-name">{{ sub.accountName || `订阅 ${index + 1}` }}</strong>
              <n-tag v-if="sub.areaName" size="small" :bordered="false">{{ sub.areaName }}</n-tag>
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
                <span>地区 / 机构</span>
                <div class="org-row">
                  <n-input
                    class="org-picker-input"
                    :value="orgDisplayText(sub)"
                    readonly
                    placeholder="点击选择地区与机构"
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

            <div class="filter-block">
              <div class="filter-title">订单状态（空=不限）</div>
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
              <div class="filter-title">业务类型（不含在缴）</div>
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

            <div class="filter-block">
              <div class="filter-title">险种过滤（空=不限）</div>
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
      </section>
    </div>

    <n-modal
      v-model:show="tokenModalVisible"
      preset="card"
      title="云生 Cookie"
      style="width: min(560px, 92vw)"
    >
      <n-text depth="3" class="modal-hint">
        从浏览器复制完整 Cookie 并粘贴，须包含 token_inner=...
      </n-text>
      <n-input
        v-model:value="cookies"
        type="textarea"
        :autosize="{ minRows: 4, maxRows: 10 }"
        placeholder="token_inner=eyJ..."
        class="token-input"
      />
      <template #footer>
        <div class="modal-actions">
          <n-button @click="tokenModalVisible = false">取消</n-button>
          <n-button type="primary" :loading="savingToken" @click="handleSaveToken">
            保存 Cookie
          </n-button>
        </div>
      </template>
    </n-modal>

    <n-modal
      v-model:show="orgPickerVisible"
      preset="card"
      title="选择机构"
      style="width: min(640px, 94vw)"
    >
      <div class="org-picker">
        <label>
          <span>地区</span>
          <n-select
            v-model:value="orgPickerAreaId"
            :options="orgPickerAreaOptions"
            :loading="loadingAreas"
            filterable
            clearable
            placeholder="选择地区"
          />
        </label>
        <div class="org-search-row">
          <n-input
            v-model:value="orgPickerKeyword"
            clearable
            placeholder="机构名称（模糊搜索）"
            @keyup.enter="handleSearchOrgs"
          />
          <n-button type="primary" :loading="searchingOrgs" @click="handleSearchOrgs">
            搜索
          </n-button>
        </div>
        <div v-if="!orgHits.length" class="empty-inline">
          <n-text depth="3">选择地区并搜索后，点击一行选择目标机构</n-text>
        </div>
        <div v-else class="org-hit-list">
          <div
            v-for="hit in orgHits"
            :key="hit.orgAccountId"
            class="org-hit-item"
            :class="{ 'is-selected': selectedOrgId === hit.orgAccountId }"
            role="button"
            tabindex="0"
            @click="selectOrgHit(hit.orgAccountId)"
            @keydown.enter.prevent="selectOrgHit(hit.orgAccountId)"
          >
            <n-radio
              :checked="selectedOrgId === hit.orgAccountId"
              :value="hit.orgAccountId"
              @click.stop
              @update:checked="(checked) => checked && selectOrgHit(hit.orgAccountId)"
            />
            <div class="org-hit-copy">
              <strong>{{ hit.accountName || '未命名机构' }}</strong>
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
