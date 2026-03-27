import axios from 'axios'

const API_BASE_URL = '/api'

// 项目管理API（文件存储版）

/**
 * 分页查询项目列表
 * @param {number} current - 当前页码
 * @param {number} size - 每页数量
 * @returns {Promise} 项目列表
 */
export const listProjects = () => axios.get(`${API_BASE_URL}/projects/list`)

/**
 * 根据ID获取项目
 * @param {number} id - 项目ID
 * @returns {Promise} 项目详情
 */
export const getProjectById = (id) => {
 return axios.get(`${API_BASE_URL}/projects/${id}`)
}

/**
 * 根据名称查询项目 - 需求5.7
 * @param {string} name - 项目名称
 * @returns {Promise} 项目详情或null
 */
export const getProjectByName = (name) => {
 return axios.get(`${API_BASE_URL}/projects/name/${encodeURIComponent(name)}`)
}

/**
 * 保存新项目
 * @param {Object} project - 项目数据
 * @returns {Promise} 保存结果
 */
export const saveProject = (project) => {
 return axios.post(`${API_BASE_URL}/projects`, project)
}

/**
 * 更新项目
 * @param {Object} project - 项目数据（包含id）
 * @returns {Promise} 更新结果
 */
export const updateProject = (project) => {
 return axios.put(`${API_BASE_URL}/projects`, project)
}

/**
 * 删除项目
 * @param {number} id - 项目ID
 * @returns {Promise} 删除结果
 */
export const deleteProject = (id) => {
 return axios.delete(`${API_BASE_URL}/projects/${id}`)
}

/**
 * 重命名项目 - 需求3.7
 * @param {number} id - 项目ID
 * @param {string} newName - 新项目名称
 * @returns {Promise} 重命名结果
 */
export const renameProject = (id, newName) => axios.put(`${API_BASE_URL}/projects`, { id, name: newName })

/**
 * 根据当前工作流配置生成 代码
 * @param {Object} payload - 生成参数
 * @returns {Promise} 代码结果
 */
export const removedGenerateCode = (payload) => {
 return axios.post(`${API_BASE_URL}/projects/generate-`, payload)
}
