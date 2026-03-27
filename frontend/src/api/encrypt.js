import axios from 'axios'

const API_BASE_URL = '/api'

/**
 * 批量执行加解密工作流
 * @param {Object} batchRequest - 批量执行请求体
 */
export const executeBatch = (batchRequest) => {
 return axios.post(`${API_BASE_URL}/encrypt/batch`, batchRequest)
}
