import { invokeApi } from './tauriClient'

export const loadOrderInsSubscribeConfig = () => invokeApi('load_order_ins_subscribe_config')

export const saveOrderInsSubscribeConfig = (config) =>
  invokeApi('save_order_ins_subscribe_config', { config })

export const listOrderInsSubscribeAreas = () => invokeApi('list_order_ins_subscribe_areas')

export const searchOrderInsSubscribeOrgs = (request) =>
  invokeApi('search_order_ins_subscribe_orgs', { request })

export const loadOrderInsSubscribeResult = () => invokeApi('load_order_ins_subscribe_result')

export const clearOrderInsSubscribeResult = () => invokeApi('clear_order_ins_subscribe_result')

export const setOrderInsSubscribeAutoRun = (enabled) =>
  invokeApi('set_order_ins_subscribe_auto_run', { enabled })

export const runOrderInsSubscribeNow = () => invokeApi('run_order_ins_subscribe_now')

/** 启动/进入首页：按本地自然日门控，必要时自动执行一轮并返回快照 */
export const maybeAutoRunOrderInsSubscribe = () => invokeApi('maybe_auto_run_order_ins_subscribe')

export const getHomePendingSummary = () => invokeApi('get_home_pending_summary')

export const ORDER_STATE_OPTIONS = [
  { value: 1, label: '待受理' },
  { value: 2, label: '已受理' },
  { value: 3, label: '反馈中' },
  { value: 4, label: '反馈完成' },
  { value: 5, label: '已归档' },
  { value: 7, label: '待审核' },
  { value: 8, label: '受理中' }
]

export const DEFAULT_ORDER_STATES = [1, 2, 3, 7, 8]

/** 新建订阅默认办理类型：报增 / 停缴 / 补缴 */
export const DEFAULT_ACCOUNT_STATUSES = [1, 3, 4]

/** 办理类型（accountStatuses → 请求 accountStatusList） */
export const ACCOUNT_STATUS_OPTIONS = [
  { value: 1, label: '报增' },
  { value: 2, label: '在缴' },
  { value: 3, label: '停缴' },
  { value: 4, label: '补缴' },
  { value: 5, label: '特殊补缴' }
]

/** 账单月快捷选项（另可指定 YYYYMM） */
export const BILL_MONTH_REL_OPTIONS = [
  { label: '上月', value: 'prev' },
  { label: '当月', value: 'current' },
  { label: '下月', value: 'next' },
  { label: '指定月份', value: '__fixed__' }
]

export const INS_CODE_OPTIONS = [
  { value: 20, label: '公积金' },
  { value: 21, label: '补充公积金' },
  { value: 30, label: '养老保险' },
  { value: 40, label: '医疗保险' },
  { value: 41, label: '补充医疗保险' },
  { value: 42, label: '住院医疗保险' },
  { value: 43, label: '大病医疗保险' },
  { value: 44, label: '地方附加医疗保险' },
  { value: 45, label: '综合基本医疗保险' },
  { value: 50, label: '失业保险' },
  { value: 60, label: '工伤保险' },
  { value: 61, label: '补充工伤保险' },
  { value: 70, label: '生育保险' },
  { value: 80, label: '残疾人保障金' },
  { value: 90, label: '工会费' },
  { value: 100, label: '长期护理保险' },
  { value: 110, label: '采暖费' },
  { value: 120, label: '生活垃圾处理费' },
  { value: 124, label: '重疾险' },
  { value: 125, label: '地方补充' },
  { value: 126, label: '其他费用' },
  { value: 127, label: '工本费' }
]

export const isFixedBillMonth = (value) => /^\d{6}$/.test(String(value || '').trim())

export const currentYyyymm = () => {
  const d = new Date()
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  return `${y}${m}`
}

export const normalizeBillMonthToken = (raw) => {
  const t = String(raw || '').trim()
  if (isFixedBillMonth(t)) return t
  const lower = t.toLowerCase()
  if (lower === 'prev' || t === '上月') return 'prev'
  if (lower === 'next' || t === '下月') return 'next'
  return 'current'
}

export const billMonthTokenLabel = (token) => {
  const t = normalizeBillMonthToken(token)
  if (t === 'prev') return '上月'
  if (t === 'next') return '下月'
  if (t === 'current') return '当月'
  return t
}

export const createDefaultSubscription = () => ({
  id: `tmp-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`,
  enabled: true,
  areaId: 0,
  areaName: '',
  orgAccounts: [],
  excludeSupplierAccounts: true,
  billMonth1: 'current',
  billMonth2: 'current',
  accountStatuses: [...DEFAULT_ACCOUNT_STATUSES],
  orderStates: [...DEFAULT_ORDER_STATES],
  insCodes: []
})
