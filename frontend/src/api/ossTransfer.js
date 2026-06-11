import { invokeApi } from './tauriClient'

export const transferOssKey = (ossKey, direction) => {
 return invokeApi('transfer_oss_key', { ossKey, direction })
}

export const loadTransferHistory = () => {
 return invokeApi('load_oss_transfer_history')
}

export const deleteTransferRecord = (id) => {
 return invokeApi('delete_oss_transfer_record', { id })
}

export const clearTransferHistory = () => {
 return invokeApi('clear_oss_transfer_history')
}
