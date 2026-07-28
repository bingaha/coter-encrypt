<script setup>
import { computed, inject, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  NButton,
  NCheckbox,
  NCheckboxGroup,
  NIcon,
  NInput,
  NModal,
  NSelect,
  NSwitch,
  NTag,
  NText,
  useMessage
} from 'naive-ui'
import {
  AddOutline,
  ArrowBackOutline,
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
  loadOrderSubscribeConfig,
  loadOrderSubscribeResult,
  ORDER_STATE_OPTIONS,
  runOrderSubscribeNow,
  saveOrderSubscribeConfig,
  searchOrderSubscribeOrgs
} from '@/api/orderSubscribe'

const router = useRouter()
const configStore = useConfigStore()
const message = useMessage()

const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())

const loading = ref(false)
const saving = ref(false)
const savingToken = ref(false)
const searchingOrgs = ref(false)
const loadingAreas = ref(false)
const running = ref(false)

const tokenInner = ref('')
const tokenModalVisible = ref(false)

const subscriptions = ref([])
const areas = ref([])
const snapshot = ref({
  executedAt: '',
  executedDate: '',
  total: 0,
  subscriptions: []
})

const bizTypeLabel = (bizType) =>
  BIZ_TYPE_OPTIONS.find((item) => item.value === bizType)?.label || bizType

const hasSnapshot = computed(
  () => !!(snapshot.value?.executedAt || (snapshot.value?.subscriptions || []).length)
)

const orgPickerVisible = ref(false)
const orgPickerIndex = ref(-1)
const orgPickerAreaId = ref(null)
const orgPickerKeyword = ref('')
const orgHits = ref([])
const selectedOrgIds = ref([])

const billMonthModeOptions = [
  { label: '当前自然月', value: 'current' },
  { label: '固定账期', value: 'fixed' }
]

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
      loadYunshengAuthToken().catch(() => ({ data: { tokenInner: '' } })),
      loadOrderSubscribeResult().catch(() => ({ data: null }))
    ])
    subscriptions.value = (configRes.data?.subscriptions || []).map((item) => ({
      ...createDefaultSubscription(),
      ...item,
      orderStates: Array.isArray(item.orderStates) ? [...item.orderStates] : [],
      bizTypes: Array.isArray(item.bizTypes) ? [...item.bizTypes] : [],
      insCodes: Array.isArray(item.insCodes) ? [...item.insCodes] : []
    }))
    tokenInner.value = tokenRes.data?.tokenInner || ''
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

const openTokenModal = async () => {
  try {
    const { data } = await loadYunshengAuthToken()
    tokenInner.value = data?.tokenInner || ''
  } catch {
    /* keep current */
  }
  tokenModalVisible.value = true
}

const handleSaveToken = async () => {
  savingToken.value = true
  try {
    const { data } = await saveYunshengAuthToken({
      tokenInner: String(tokenInner.value || '').trim()
    })
    tokenInner.value = data?.tokenInner || ''
    message.success('Token 已保存')
    tokenModalVisible.value = false
  } catch (error) {
    message.error(error?.message || '保存 Token 失败')
  } finally {
    savingToken.value = false
  }
}

const addSubscription = () => {
  subscriptions.value.push(createDefaultSubscription())
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
  selectedOrgIds.value = sub?.orgAccountId > 0 ? [sub.orgAccountId] : []
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
  const selectedId = selectedOrgIds.value[0]
  if (!selectedId) {
    message.warning('请勾选一个机构')
    return
  }
  const hit = orgHits.value.find((item) => item.orgAccountId === selectedId)
  const areaId = orgPickerAreaId.value
  const sub = subscriptions.value[index]
  sub.orgAccountId = selectedId
  sub.accountName = hit?.accountName || sub.accountName
  sub.areaId = areaId
  sub.areaName = areaNameOf(areaId) || sub.areaName
  orgPickerVisible.value = false
  message.success('已选择机构')
}

const onOrgCheck = (id, checked) => {
  // 单选语义：每次只保留一个
  selectedOrgIds.value = checked ? [id] : []
}

const buildConfigPayload = () => ({
  subscriptions: subscriptions.value.map((item) => ({
    id: String(item.id || '').trim(),
    enabled: !!item.enabled,
    orgAccountId: Number(item.orgAccountId) || 0,
    accountName: String(item.accountName || '').trim(),
    areaId: Number(item.areaId) || 0,
    areaName: String(item.areaName || '').trim(),
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
          Token
        </n-button>
        <n-button type="primary" :loading="running" :disabled="loading" @click="handleRunNow">
          <template #icon>
            <n-icon><PlayOutline /></n-icon>
          </template>
          立即执行
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
                （已订阅类型高亮计入，灰色仅展示）
              </template>
              <template v-else>尚未执行；配置保存后点击「立即执行」</template>
            </n-text>
          </div>
        </div>

        <div v-if="!hasSnapshot" class="empty">
          <n-text depth="3">暂无结果快照</n-text>
        </div>

        <article
          v-for="item in snapshot.subscriptions"
          :key="item.subscriptionId"
          class="result-card"
          :class="{ 'is-error': !item.success }"
        >
          <div class="result-card-head">
            <div class="result-card-title">
              <strong>{{ item.accountName || item.subscriptionId }}</strong>
              <n-tag size="small" :bordered="false" :type="item.success ? 'success' : 'error'">
                {{ item.success ? '成功' : '失败' }}
              </n-tag>
              <n-text depth="3" class="result-meta">
                账期 {{ item.billMonth || '—' }}
                <template v-if="item.configAreaName"> · {{ item.configAreaName }}</template>
              </n-text>
            </div>
            <strong v-if="item.success" class="result-subtotal">{{ item.subscribedTotal }}</strong>
          </div>

          <n-text v-if="!item.success" type="error" class="result-error">
            {{ item.error || '查询失败' }}
          </n-text>

          <div v-else-if="!(item.areas || []).length" class="empty-inline">
            <n-text depth="3">无订单记录</n-text>
          </div>

          <div v-else class="area-blocks">
            <div v-for="area in item.areas" :key="area.areaName" class="area-block">
              <div class="area-name">{{ area.areaName }}</div>
              <div class="biz-count-row">
                <span
                  v-for="count in area.counts"
                  :key="count.bizType"
                  class="biz-count"
                  :class="count.highlighted ? 'is-hot' : 'is-muted'"
                >
                  {{ bizTypeLabel(count.bizType) }}
                  <em>{{ count.count }}</em>
                </span>
              </div>
            </div>
          </div>
        </article>
      </section>

      <section class="panel">
        <div class="panel-title">
          <div class="panel-title-copy">
            <strong>订阅列表</strong>
            <n-text depth="3" class="panel-hint">
              每条绑定一个机构与账期；停用订阅不会参与执行与总数。
            </n-text>
          </div>
          <n-button size="small" secondary :disabled="loading" @click="addSubscription">
            <template #icon>
              <n-icon><AddOutline /></n-icon>
            </template>
            新增订阅
          </n-button>
        </div>

        <div v-if="!subscriptions.length" class="empty">
          <n-text depth="3">暂无订阅，点击「新增订阅」开始配置</n-text>
        </div>

        <article
          v-for="(sub, index) in subscriptions"
          :key="sub.id || index"
          class="sub-card"
        >
          <div class="sub-card-head">
            <div class="sub-card-title">
              <n-switch v-model:value="sub.enabled" size="small" />
              <strong>{{ sub.accountName || `订阅 ${index + 1}` }}</strong>
              <n-tag v-if="!sub.enabled" size="small" :bordered="false">已禁用</n-tag>
            </div>
            <n-button quaternary circle type="error" @click="removeSubscription(index)">
              <template #icon>
                <n-icon><TrashOutline /></n-icon>
              </template>
            </n-button>
          </div>

          <div class="form-grid">
            <label>
              <span>地区 / 机构</span>
              <div class="org-row">
                <n-input
                  :value="
                    sub.accountName
                      ? `${sub.accountName}${sub.areaName ? ' · ' + sub.areaName : ''}`
                      : ''
                  "
                  readonly
                  placeholder="选地区后搜索并勾选机构"
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
              <span>账期模式</span>
              <n-select
                v-model:value="sub.billMonthMode"
                :options="billMonthModeOptions"
              />
            </label>
            <label v-if="sub.billMonthMode === 'fixed'">
              <span>固定账期（YYYYMM）</span>
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
            <div class="filter-title">险种过滤 insCodes（空=不限）</div>
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
        </article>
      </section>
    </div>

    <n-modal
      v-model:show="tokenModalVisible"
      preset="card"
      title="云盛 Token（token_inner）"
      style="width: min(520px, 92vw)"
    >
      <n-text depth="3" class="modal-hint">
        共享鉴权，供后道订单及其他管理端功能复用。从浏览器 Cookie 复制 token_inner 即可。
      </n-text>
      <n-input
        v-model:value="tokenInner"
        type="password"
        show-password-on="click"
        placeholder="粘贴 token_inner"
        class="token-input"
      />
      <template #footer>
        <div class="modal-actions">
          <n-button @click="tokenModalVisible = false">取消</n-button>
          <n-button type="primary" :loading="savingToken" @click="handleSaveToken">
            保存 Token
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
          <n-text depth="3">选择地区并搜索后，勾选目标机构</n-text>
        </div>
        <div v-else class="org-hit-list">
          <label
            v-for="hit in orgHits"
            :key="hit.orgAccountId"
            class="org-hit-item"
          >
            <n-checkbox
              :checked="selectedOrgIds.includes(hit.orgAccountId)"
              @update:checked="(checked) => onOrgCheck(hit.orgAccountId, checked)"
            />
            <div class="org-hit-copy">
              <strong>{{ hit.accountName || '未命名机构' }}</strong>
              <n-text depth="3">ID {{ hit.orgAccountId }}</n-text>
            </div>
          </label>
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

.result-card {
  border: 1px solid var(--n-border-color, #e0e0e6);
  border-radius: 12px;
  padding: 14px;
  margin-bottom: 12px;
  display: grid;
  gap: 10px;
}

.result-card:last-child {
  margin-bottom: 0;
}

.result-card.is-error {
  border-color: color-mix(in srgb, #d03050 35%, var(--n-border-color, #e0e0e6));
}

.result-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.result-card-title {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.result-meta {
  font-size: 12px;
}

.result-subtotal {
  font-size: 22px;
  color: #18a058;
  line-height: 1;
}

.result-error {
  font-size: 13px;
}

.area-blocks {
  display: grid;
  gap: 10px;
}

.area-block {
  display: grid;
  gap: 6px;
}

.area-name {
  font-size: 13px;
  font-weight: 600;
}

.biz-count-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.biz-count {
  font-size: 12px;
  padding: 4px 8px;
  border-radius: 6px;
  display: inline-flex;
  align-items: baseline;
  gap: 4px;
}

.biz-count em {
  font-style: normal;
  font-weight: 700;
}

.biz-count.is-hot {
  color: #0c7a43;
  background: rgba(24, 160, 88, 0.12);
}

.biz-count.is-muted {
  color: var(--n-text-color-3, #9ca3af);
  background: rgba(128, 128, 128, 0.08);
}

.sub-card {
  border: 1px solid var(--n-border-color, #e0e0e6);
  border-radius: 12px;
  padding: 14px;
  margin-bottom: 12px;
  display: grid;
  gap: 12px;
}

.sub-card:last-child {
  margin-bottom: 0;
}

.sub-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.sub-card-title {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
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
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
}

.org-hit-item:hover {
  background: rgba(24, 160, 88, 0.06);
}

.org-hit-copy {
  display: grid;
  gap: 2px;
  min-width: 0;
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
