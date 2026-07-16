import { invokeApi } from './tauriClient'

// 项目管理 API：桌面版只通过 Tauri invoke 调用 Rust 命令。

/**
 * 查询项目列表
 * @returns {Promise} 项目列表
 */
export const listProjects = () => invokeApi('list_projects')

/**
 * 根据 ID 获取项目
 * @param {number} id - 项目 ID
 * @returns {Promise} 项目详情
 */
export const getProjectById = (id) => {
 return invokeApi('get_project_by_id', { id })
}

/**
 * 根据名称查询项目
 * @param {string} name - 项目名称
 * @returns {Promise} 项目详情或 null
 */
export const getProjectByName = (name) => {
 return invokeApi('get_project_by_name', { name })
}

/**
 * 保存新项目
 * @param {Object} project - 项目数据
 * @returns {Promise} 保存结果
 */
export const saveProject = (project) => {
 return invokeApi('save_project', { project })
}

/**
 * 更新项目
 * @param {Object} project - 项目数据，包含 id
 * @returns {Promise} 更新结果
 */
export const updateProject = (project) => {
 return invokeApi('update_project', { project })
}

/**
 * 删除项目
 * @param {number} id - 项目 ID
 * @returns {Promise} 删除结果
 */
export const deleteProject = (id) => {
 return invokeApi('delete_project', { id })
}

/**
 * 重命名项目
 * @param {number} id - 项目 ID
 * @param {string} newName - 新项目名称
 * @returns {Promise} 重命名结果
 */
export const renameProject = (id, newName) => {
 return invokeApi('rename_project', { id, newName })
}

