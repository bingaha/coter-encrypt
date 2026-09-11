import { invokeApi } from './tauriClient'

export const detectOssKeys = (text) => invokeApi('detect_oss_keys', { text })

export const transferOssText = (text, direction) =>
  invokeApi('transfer_oss_text', { text, direction })

export const loadTransferHistory = () => {
  return invokeApi('load_oss_transfer_history')
}

export const deleteTransferRecord = (id) => {
  return invokeApi('delete_oss_transfer_record', { id })
}

export const clearTransferHistory = () => {
  return invokeApi('clear_oss_transfer_history')
}
