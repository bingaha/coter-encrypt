import { invokeApi } from './tauriClient'

export const loadPipelineMonitorConfig = () => invokeApi('load_pipeline_monitor_config')

export const savePipelineMonitorConfig = (config) =>
  invokeApi('save_pipeline_monitor_config', { config })

export const startPipelineMonitor = () => invokeApi('start_pipeline_monitor')

export const startPipelineMonitorSingle = (request) =>
  invokeApi('start_pipeline_monitor_single', { request })

export const stopPipelineMonitor = () => invokeApi('stop_pipeline_monitor')

export const queryPipelineLatestRun = (pipelineId) =>
  invokeApi('query_pipeline_latest_run', { pipelineId })

export const getPipelineMonitorSnapshot = () => invokeApi('get_pipeline_monitor_snapshot')

export const respondPipelineMonitorAction = (request) =>
  invokeApi('respond_pipeline_monitor_action', { request })

export const openPipelineRunPage = (pipelineId, runId = '') =>
  invokeApi('open_pipeline_run_page', { pipelineId, runId })

export const clearPipelineMonitorLogs = () => invokeApi('clear_pipeline_monitor_logs')
