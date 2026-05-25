import { computed, ref } from 'vue'
import {
 loadMysqlDatasourceConfig,
 saveMysqlDatasourceConfig,
 testMysqlDatasource
} from '@/api/certQuery'

const defaultForm = () => ({
 host: '',
 port: 3306,
 database: '',
 username: '',
 password: '',
 connectTimeoutSeconds: 8
})

const modalVisible = ref(false)
const datasourceForm = ref(defaultForm())
const hasSavedDatasource = ref(false)
const hasSavedPassword = ref(false)
const loadingConfig = ref(false)
const savingDatasource = ref(false)
const testingDatasource = ref(false)
const connectionStatus = ref('unknown')
const connectionMessage = ref('')

let loadedOnce = false
let loadingPromise = null

const canSaveDatasource = computed(() => {
 const form = datasourceForm.value

 return Boolean(
 form.host.trim() &&
 form.port &&
 form.database.trim() &&
 form.username.trim() &&
 (form.password || hasSavedPassword.value) &&
 form.connectTimeoutSeconds > 0
 )
})

const statusLabel = computed(() => {
 switch (connectionStatus.value) {
 case 'success':
 return '连接成功'
 case 'failed':
 return '连接失败'
 case 'checking':
 return '检测中'
 case 'unconfigured':
 return '未配置'
 default:
 return hasSavedDatasource.value ? '未检测' : '未配置'
 }
})

const statusTagType = computed(() => {
 switch (connectionStatus.value) {
 case 'success':
 return 'success'
 case 'failed':
 return 'error'
 case 'checking':
 return 'info'
 case 'unconfigured':
 return 'warning'
 default:
 return hasSavedDatasource.value ? 'default' : 'warning'
 }
})

const setFormFromConfig = (config) => {
 datasourceForm.value = {
 host: config.host || '',
 port: config.port || 3306,
 database: config.database || '',
 username: config.username || '',
 password: '',
 connectTimeoutSeconds: config.connectTimeoutSeconds || 8
 }
 hasSavedDatasource.value = true
 hasSavedPassword.value = Boolean(config.hasPassword)
}

const toDatasourcePayload = () => {
 const form = datasourceForm.value

 return {
 host: form.host.trim(),
 port: Number(form.port),
 database: form.database.trim(),
 username: form.username.trim(),
 password: form.password ? form.password : null,
 connectTimeoutSeconds: Number(form.connectTimeoutSeconds)
 }
}

const loadConfig = async ({ force = false } = {}) => {
 if (loadingPromise) {
 return loadingPromise
 }

 if (loadedOnce && !force) {
 return hasSavedDatasource.value
 }

 loadingConfig.value = true
 loadingPromise = loadMysqlDatasourceConfig()
 .then(response => {
 const config = response.data

 loadedOnce = true
 if (!config) {
 datasourceForm.value = defaultForm()
 hasSavedDatasource.value = false
 hasSavedPassword.value = false
 connectionStatus.value = 'unconfigured'
 connectionMessage.value = '尚未保存 MySQL 数据源'
 return false
 }

 setFormFromConfig(config)
 if (connectionStatus.value === 'unconfigured') {
 connectionStatus.value = 'unknown'
 connectionMessage.value = ''
 }
 return true
 })
 .finally(() => {
 loadingConfig.value = false
 loadingPromise = null
 })

 return loadingPromise
}

const checkConnection = async () => {
 await loadConfig()

 if (!hasSavedDatasource.value) {
 connectionStatus.value = 'unconfigured'
 connectionMessage.value = '请先配置 MySQL 数据源'
 return false
 }

 testingDatasource.value = true
 connectionStatus.value = 'checking'
 connectionMessage.value = ''

 try {
 await testMysqlDatasource(null)
 connectionStatus.value = 'success'
 connectionMessage.value = 'MySQL 连接正常'
 return true
 } catch (error) {
 connectionStatus.value = 'failed'
 connectionMessage.value = error?.message || 'MySQL 连接失败'
 return false
 } finally {
 testingDatasource.value = false
 }
}

const saveConfig = async () => {
 if (!canSaveDatasource.value) {
 throw new Error('请补全数据源配置')
 }

 savingDatasource.value = true
 try {
 const response = await saveMysqlDatasourceConfig(toDatasourcePayload())
 const saved = response.data

 hasSavedDatasource.value = true
 hasSavedPassword.value = Boolean(saved?.hasPassword)
 datasourceForm.value.password = ''
 loadedOnce = true

 return await checkConnection()
 } finally {
 savingDatasource.value = false
 }
}

const openModal = async () => {
 modalVisible.value = true
 await loadConfig()
}

const closeModal = () => {
 modalVisible.value = false
}

const ensureReady = async () => {
 const loaded = await loadConfig()

 if (!loaded) {
 modalVisible.value = true
 return false
 }

 const connected = await checkConnection()
 if (!connected) {
 modalVisible.value = true
 }

 return connected
}

const resetLoadedState = () => {
 loadedOnce = false
}

export const useMysqlDatasourceConfig = () => ({
 modalVisible,
 datasourceForm,
 hasSavedDatasource,
 hasSavedPassword,
 loadingConfig,
 savingDatasource,
 testingDatasource,
 connectionStatus,
 connectionMessage,
 canSaveDatasource,
 statusLabel,
 statusTagType,
 loadConfig,
 checkConnection,
 saveConfig,
 openModal,
 closeModal,
 ensureReady,
 resetLoadedState
})
