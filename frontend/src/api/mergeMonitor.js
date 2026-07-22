import { invokeApi } from './tauriClient'

export const loadMergeMonitorConfig = () => invokeApi('load_merge_monitor_config')

export const saveMergeMonitorConfig = (config) =>
  invokeApi('save_merge_monitor_config', { config })

export const startMergeMonitor = () => invokeApi('start_merge_monitor')

export const stopMergeMonitor = () => invokeApi('stop_merge_monitor')

export const getMergeMonitorSnapshot = () => invokeApi('get_merge_monitor_snapshot')

export const clearMergeMonitorLogs = () => invokeApi('clear_merge_monitor_logs')

export const openMergeRequestPage = (detailUrl) =>
  invokeApi('open_merge_request_page', { detailUrl })
