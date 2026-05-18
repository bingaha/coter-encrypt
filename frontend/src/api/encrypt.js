import { invokeApi } from './tauriClient'

/**
 * 批量执行加解密工作流
 * @param {Object} batchRequest - 批量执行请求体
 */
export const executeBatch = (batchRequest) => {
 return invokeApi('execute_batch', { request: batchRequest })
}
