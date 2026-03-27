import { ref, computed, h, nextTick } from 'vue'
import { useMessage, useDialog, NInput } from 'naive-ui'
import { useConfigStore } from '../store'
import { listProjects, getProjectByName, saveProject, updateProject, deleteProject, renameProject } from '../api/project'

/**
 * 项目管理 composable
 * 处理项目的保存、加载、删除、重命名等功能
 * Requirements: 3.2, 3.3, 3.5, 3.6, 3.7, 3.8
 */
export function useProjectManager() {
 const message = useMessage()
 const dialog = useDialog()
 const configStore = useConfigStore()

 // 状态
 const projects = ref([])
 const isLoading = ref(false)
 const isSaving = ref(false)
 const drawerVisible = ref(false)

 // 计算属性
 const currentProject = computed(() => configStore.currentProject)
 const currentProjectId = computed(() => configStore.currentProjectId)
 const currentProjectName = computed(() => configStore.currentProjectName)
 const hasCurrentProject = computed(() => !!currentProject.value)

 /**
 * 获取项目列表
 */
 const fetchProjects = async () => {
 isLoading.value = true
 try {
 const response = await listProjects()
 projects.value = response.data || []
 } catch (error) {
 message.error('获取项目列表失败: ' + (error.message || '未知错误'))
 projects.value = []
 } finally {
 isLoading.value = false
 }
 }

 /**
 * 打开项目管理抽屉
 */
 const openDrawer = async () => {
 drawerVisible.value = true
 await fetchProjects()
 }

 /**
 * 关闭项目管理抽屉
 */
 const closeDrawer = () => {
 drawerVisible.value = false
 }

 /**
 * 获取当前配置数据
 */
 const getConfigData = () => {
 return {
 components: configStore.addedComponents,
 inputMappings: configStore.inputMappings,
 outputMappings: configStore.outputMappings,
 executionConfig: configStore.executionConfig
 }
 }

 /**
 * 保存项目 - Requirements 3.3, 3.8
 * 如果当前有已加载的项目，自动使用该项目名保存
 * 如果是新项目，弹出对话框输入项目名
 */
 const handleSave = async () => {
 // 如果当前有已加载的项目，直接更新 - Requirements 3.3
 if (hasCurrentProject.value) {
 await doUpdateCurrentProject()
 } else {
 // 新项目，弹出输入框
 await promptForProjectName()
 }
 }

 /**
 * 另存为新项目
 * 始终弹出对话框输入新项目名
 */
 const handleSaveAs = async () => {
 await promptForProjectName(true)
 }

 /**
 * 弹出项目名输入对话框
 * @param {boolean} isSaveAs - 是否为另存为操作
 */
 const promptForProjectName = (isSaveAs = false) => {
 return new Promise((resolve) => {
 // 使用 ref 保证响应式更新
 const inputValue = ref('')
 
 dialog.create({
 title: isSaveAs ? '另存为新项目' : '保存项目',
 content: () => {
 return h(NInput, {
 placeholder: '请输入项目名称',
 value: inputValue.value,
 'onUpdate:value': (v) => { inputValue.value = v },
 autofocus: true
 })
 },
 positiveText: '保存',
 negativeText: '取消',
 maskClosable: false,
 onPositiveClick: async () => {
 const name = inputValue.value.trim()
 if (!name) {
 message.warning('请输入项目名称')
 return false // 阻止关闭
 }
 await checkAndSaveProject(name)
 resolve(true)
 },
 onNegativeClick: () => {
 resolve(false)
 }
 })
 })
 }

 /**
 * 检查项目名是否存在并保存 - Requirements 3.8
 * @param {string} name - 项目名称
 */
 const checkAndSaveProject = async (name) => {
 isSaving.value = true
 try {
 // 检查项目名是否存在
 const response = await getProjectByName(name)
 if (response.data) {
 // 项目已存在，询问是否覆盖 - Requirements 3.8
 const existingProject = response.data
 await confirmOverwrite(existingProject, name)
 } else {
 // 项目不存在，直接保存
 await doSaveNewProject(name)
 }
 } catch (error) {
 // 如果是404错误，说明项目不存在，可以直接保存
 if (error.response && error.response.status === 404) {
 await doSaveNewProject(name)
 } else {
 message.error('检查项目名失败: ' + (error.message || '未知错误'))
 }
 } finally {
 isSaving.value = false
 }
 }

 /**
 * 确认覆盖已存在的项目 - Requirements 3.8
 * @param {Object} existingProject - 已存在的项目
 * @param {string} name - 项目名称
 */
 const confirmOverwrite = (existingProject, name) => {
 return new Promise((resolve) => {
 dialog.warning({
 title: '项目已存在',
 content: `项目 "${name}" 已存在，是否覆盖？`,
 positiveText: '覆盖',
 negativeText: '取消',
 onPositiveClick: async () => {
 await doUpdateProject(existingProject.id, name)
 resolve(true)
 },
 onNegativeClick: () => {
 resolve(false)
 }
 })
 })
 }

 /**
 * 保存新项目
 * @param {string} name - 项目名称
 */
 const doSaveNewProject = async (name) => {
 try {
 const config = getConfigData()
 const response = await saveProject({
 name: name,
 description: '',
 config: JSON.stringify(config),
 status: 1
 })
 
 // 保存成功后，设置当前项目 - Requirements 3.2
 const savedProject = response.data || { id: null, name: name }
 configStore.setCurrentProject({
 id: savedProject.id,
 name: name,
 updateTime: new Date().toISOString()
 })
 
 // 同步到 localStorage
 configStore.saveConfigToLocal()
 
 message.success('项目保存成功')
 
 // 刷新项目列表
 await fetchProjects()
 } catch (error) {
 message.error('保存项目失败: ' + (error.message || '未知错误'))
 throw error
 }
 }

 /**
 * 更新已存在的项目
 * @param {number} projectId - 项目ID
 * @param {string} name - 项目名称
 */
 const doUpdateProject = async (projectId, name) => {
 try {
 const config = getConfigData()
 await updateProject({
 id: projectId,
 name: name,
 config: JSON.stringify(config),
 status: 1
 })
 
 // 更新成功后，更新当前项目信息
 configStore.setCurrentProject({
 id: projectId,
 name: name,
 updateTime: new Date().toISOString()
 })
 
 // 同步到 localStorage
 configStore.saveConfigToLocal()
 
 message.success('项目更新成功')
 
 // 刷新项目列表
 await fetchProjects()
 } catch (error) {
 message.error('更新项目失败: ' + (error.message || '未知错误'))
 throw error
 }
 }

 /**
 * 更新当前已加载的项目 - Requirements 3.3
 */
 const doUpdateCurrentProject = async () => {
 if (!currentProject.value) {
 message.warning('没有已加载的项目')
 return
 }
 
 isSaving.value = true
 try {
 await doUpdateProject(currentProject.value.id, currentProject.value.name)
 } finally {
 isSaving.value = false
 }
 }

 /**
 * 加载项目
 * @param {Object} project - 项目对象
 */
 const loadProject = (project) => {
 try {
 const config = JSON.parse(project.config)
 configStore.addedComponents = config.components || []
 configStore.inputMappings = config.inputMappings || []
 configStore.outputMappings = config.outputMappings || []
 configStore.executionConfig = config.executionConfig || { inputs: [], outputs: [] }
 
 // 设置当前项目 - Requirements 3.2
 configStore.setCurrentProject({
 id: project.id,
 name: project.name,
 updateTime: project.updateTime
 })
 
 // 同步到 localStorage
 configStore.saveConfigToLocal()
 
 message.success(`项目 "${project.name}" 加载成功`)
 closeDrawer()
 } catch (error) {
 message.error('加载项目配置失败: ' + (error.message || '配置格式错误'))
 }
 }

 /**
 * 删除项目
 * @param {Object} project - 项目对象
 */
 const handleDeleteProject = async (project) => {
 try {
 await deleteProject(project.id)
 message.success('项目删除成功')
 
 // 如果删除的是当前项目，清除当前项目状态
 if (currentProjectId.value === project.id) {
 configStore.clearCurrentProject()
 configStore.saveConfigToLocal()
 }
 
 // 刷新项目列表
 await fetchProjects()
 } catch (error) {
 message.error('删除项目失败: ' + (error.message || '未知错误'))
 }
 }

 /**
 * 重命名项目 - Requirements 3.7
 * @param {Object} project - 项目对象
 * @param {string} newName - 新名称
 */
 const handleRenameProject = async (project, newName) => {
 try {
 await renameProject(project.id, newName)
 message.success('项目重命名成功')
 
 // 如果重命名的是当前项目，更新当前项目名称
 if (currentProjectId.value === project.id) {
 configStore.setCurrentProject({
 ...currentProject.value,
 name: newName
 })
 configStore.saveConfigToLocal()
 }
 
 // 刷新项目列表
 await fetchProjects()
 } catch (error) {
 message.error('重命名项目失败: ' + (error.message || '未知错误'))
 throw error
 }
 }

 /**
 * 选择项目（从下拉框选择）
 * @param {Object} project - 项目对象
 */
 const selectProject = (project) => {
 loadProject(project)
 }

 /**
 * 新建项目 - 清空当前配置，开始新项目
 */
 const handleNew = () => {
 dialog.warning({
 title: '新建项目',
 content: '新建项目将清空当前配置，是否继续？',
 positiveText: '确定',
 negativeText: '取消',
 onPositiveClick: () => {
 // 清空当前项目状态
 configStore.clearCurrentProject()
 // 清空组件配置
 configStore.addedComponents = []
 configStore.inputMappings = []
 configStore.outputMappings = []
 configStore.executionConfig = { inputs: [], outputs: [] }
 // 同步到 localStorage
 configStore.saveConfigToLocal()
 message.success('已创建新项目')
 }
 })
 }

 /**
 * 导出当前配置为 JSON 文件
 */
 const handleExport = () => {
 const config = getConfigData()
 const exportData = {
 name: currentProjectName.value || '未命名项目',
 config
 }
 const json = JSON.stringify(exportData, null, 2)
 const blob = new Blob([json], { type: 'application/json' })
 const url = URL.createObjectURL(blob)
 const a = document.createElement('a')
 a.href = url
 a.download = `${exportData.name}.json`
 a.click()
 URL.revokeObjectURL(url)
 message.success('配置已导出')
 }

 /**
 * 从 JSON 文件导入配置
 * 导入后弹窗让用户确认项目名，名称冲突时不允许保存，需改名
 */
 const handleImport = () => {
 const input = document.createElement('input')
 input.type = 'file'
 input.accept = '.json'
 input.onchange = async (e) => {
 const file = e.target.files[0]
 if (!file) return
 try {
 const text = await file.text()
 const data = JSON.parse(text)
 const config = data.config || data
 const defaultName = data.name || file.name.replace(/\.json$/i, '')

 if (!config.components && !config.inputMappings) {
 message.error('无效的配置文件格式')
 return
 }

 // 先加载配置到工作区
 configStore.addedComponents = config.components || []
 configStore.inputMappings = config.inputMappings || []
 configStore.outputMappings = config.outputMappings || []
 configStore.executionConfig = config.executionConfig || { inputs: [], outputs: [] }
 configStore.clearCurrentProject()
 configStore.saveConfigToLocal()

 // 弹窗让用户确认保存的项目名
 await promptImportSave(defaultName)
 } catch (err) {
 message.error('导入失败: ' + (err.message || '文件格式错误'))
 }
 }
 input.click()
 }

 /**
 * 导入后弹窗确认项目名并保存
 * 名称冲突时提示用户修改，不允许直接覆盖
 * @param {string} defaultName - 默认项目名（来自文件）
 */
 const promptImportSave = (defaultName) => {
 return new Promise((resolve) => {
 const inputValue = ref(defaultName)
 const errorMsg = ref('')

 const doCheck = async () => {
 const name = inputValue.value.trim()
 if (!name) {
 errorMsg.value = '请输入项目名称'
 return false // 阻止关闭
 }
 errorMsg.value = ''
 try {
 const response = await getProjectByName(name)
 if (response.data) {
 errorMsg.value = `项目「${name}」已存在，请输入其他名称`
 return false // 阻止关闭
 }
 } catch (err) {
 if (!(err.response && err.response.status === 404)) {
 errorMsg.value = '检查项目名失败: ' + (err.message || '未知错误')
 return false
 }
 // 404 表示不存在，可以继续
 }
 // 名称可用，执行保存
 try {
 await doSaveNewProject(name)
 resolve(true)
 } catch (saveErr) {
 message.error('保存导入项目失败: ' + (saveErr.message || '未知错误'))
 errorMsg.value = '保存失败，请重试或更换名称'
 return false // 阻止关闭
 }
 }

 dialog.create({
 title: '保存导入的项目',
 content: () => {
 return h('div', [
 h(NInput, {
 placeholder: '请输入项目名称',
 value: inputValue.value,
 status: errorMsg.value ? 'error' : undefined,
 'onUpdate:value': (v) => { inputValue.value = v; errorMsg.value = '' },
 autofocus: true
 }),
 errorMsg.value
 ? h('div', { style: 'color: #d03050; font-size: 13px; margin-top: 6px' }, errorMsg.value)
 : null
 ])
 },
 positiveText: '保存',
 negativeText: '不保存',
 maskClosable: false,
 onPositiveClick: doCheck,
 onNegativeClick: () => {
 message.warning('导入的配置未保存，离开页面后将丢失')
 resolve(false)
 }
 })
 })
 }

 return {
 // 状态
 projects,
 isLoading,
 isSaving,
 drawerVisible,
 currentProject,
 currentProjectId,
 currentProjectName,
 hasCurrentProject,
 
 // 方法
 fetchProjects,
 openDrawer,
 closeDrawer,
 handleNew,
 handleSave,
 handleSaveAs,
 loadProject,
 handleDeleteProject,
 handleRenameProject,
 selectProject,
 handleExport,
 handleImport
 }
}
