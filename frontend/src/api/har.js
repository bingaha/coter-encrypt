import axios from 'axios'

const API_BASE_URL = '/api'

/**
 * 使用指定项目配置处理 HAR 文件
 * @param {FormData} formData 包含 file / projectId|projectName / inputOriginalRef / finalOutputMappingId / regexPreset / inputValues
 */
export const processHarWithProject = (formData) => {
 // 让浏览器自动设置 multipart boundary，避免手动设置导致的边界不匹配
 return axios.post(`${API_BASE_URL}/har/process`, formData)
}
