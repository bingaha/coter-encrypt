import { invokeApi } from './tauriClient'

export const loadOrderSubscribeConfig = () => invokeApi('load_order_subscribe_config')

export const saveOrderSubscribeConfig = (config) =>
  invokeApi('save_order_subscribe_config', { config })

export const listOrderSubscribeAreas = () => invokeApi('list_order_subscribe_areas')

export const searchOrderSubscribeOrgs = (request) =>
  invokeApi('search_order_subscribe_orgs', { request })

export const loadOrderSubscribeResult = () => invokeApi('load_order_subscribe_result')

export const runOrderSubscribeNow = () => invokeApi('run_order_subscribe_now')

/** 启动/进入首页：按本地自然日门控，必要时自动执行一轮并返回快照 */
export const maybeAutoRunOrderSubscribe = () => invokeApi('maybe_auto_run_order_subscribe')

/** 首页待办摘要（v1 仅后道订单；结构预留多业务） */
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

export const BIZ_TYPE_OPTIONS = [
  { value: 'sbAdd', label: '社保增员' },
  { value: 'sbFill', label: '社保补缴' },
  { value: 'sbStop', label: '社保减员' },
  { value: 'gjjAdd', label: '公积金增员' },
  { value: 'gjjFill', label: '公积金补缴' },
  { value: 'gjjStop', label: '公积金减员' }
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

export const createDefaultSubscription = () => ({
  id: `tmp-${Date.now()}-${Math.random().toString(16).slice(2, 8)}`,
  enabled: true,
  orgAccountId: 0,
  accountName: '',
  areaId: 0,
  areaName: '',
  billMonthMode: 'current',
  billMonth: '',
  orderStates: [...DEFAULT_ORDER_STATES],
  bizTypes: [],
  insCodes: []
})
