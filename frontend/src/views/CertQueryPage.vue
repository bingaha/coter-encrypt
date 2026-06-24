<script setup>
import { computed, inject, onMounted, ref } from 'vue'
import {
 NButton,
 NEmpty,
 NIcon,
 NInput,
 NModal,
 NSelect,
 NTag,
 NText,
 NTooltip,
 useMessage
} from 'naive-ui'
import {
 ArrowBackOutline,
 ClipboardOutline,
 CopyOutline,
 KeyOutline,
 MoonOutline,
 OpenOutline,
 SaveOutline,
 SearchOutline,
 SunnyOutline
} from '@vicons/ionicons5'
import { useConfigStore } from '@/store'
import {
 loadBrowserBridgeConfig,
 loadWebsiteUrlMappings,
 openDefaultBrowserWithCookies,
 queryCertInfo,
 saveBrowserBridgeConfig,
 saveWebsiteUrlMapping
} from '@/api/certQuery'
import { useMysqlDatasourceConfig } from '@/composables/useMysqlDatasourceConfig'

const configStore = useConfigStore()
const message = useMessage()

const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())
const {
 statusLabel,
 statusTagType,
 ensureReady,
 openModal,
 loadConfig
} = useMysqlDatasourceConfig()
const bridgeForm = ref({
 extensionId: ''
})
const hasSavedBridgeConfig = ref(false)
const savingBridgeConfig = ref(false)
const websiteUrlMappings = ref([])
const loadingMappings = ref(false)
const openingKey = ref('')
const mappingModalVisible = ref(false)
const mappingForm = ref({
 areaId: '',
 businessType: '',
 url: '',
 storageRules: []
})
const savingMapping = ref(false)

const storageTypeOptions = [
 { label: 'sessionStorage', value: 'sessionStorage' },
 { label: 'localStorage', value: 'localStorage' }
]

const sourceTypeOptions = [
 { label: '从 cert_info 取值', value: 'path' },
 { label: '固定值', value: 'value' }
]

const queryForm = ref({
 mainName: '',
 businessType: '公积金'
})
const querying = ref(false)
const queryItems = ref([])
const copiedKey = ref('')

const businessTypeOptions = [
 '公积金',
 '社保',
 '医保',
 '养老',
 '工伤',
 '失业',
 '三口合一',
 '公积金缴费',
 '公积金补缴',
 '养老工伤',
 '劳动关系',
 '工伤失业',
 '市公积金',
 '市养老',
 '招退工',
 '省公积金',
 '省养老',
 '社保凭证',
 '社保调基',
 '社保费客户端',
 '社保费管理客户端',
 '税务',
 '采暖费'
].map(value => ({
 label: value,
 value
}))

const canQuery = computed(() => {
 return Boolean(
 queryForm.value.mainName.trim() &&
 queryForm.value.businessType.trim()
 )
})

const resultCountLabel = computed(() => {
 if (!queryItems.value.length) {
 return '无结果'
 }

 return `${queryItems.value.length} 条结果`
})

const mappingCountLabel = computed(() => {
 if (loadingMappings.value) {
 return '加载中'
 }

 return `${websiteUrlMappings.value.length} 条映射`
})

const canSaveBridgeConfig = computed(() => {
 return /^[a-p]{32}$/.test(bridgeForm.value.extensionId.trim())
})

const canSaveMapping = computed(() => {
 return Boolean(
 mappingForm.value.areaId.trim() &&
 mappingForm.value.businessType.trim() &&
 mappingForm.value.url.trim()
 )
})

const loadBridgeConfig = async () => {
 try {
 const response = await loadBrowserBridgeConfig()
 bridgeForm.value.extensionId = response.data?.extensionId || ''
 hasSavedBridgeConfig.value = Boolean(bridgeForm.value.extensionId)
 } catch (error) {
 message.error(error?.message || '读取浏览器插件配置失败')
 }
}

const loadMappings = async () => {
 loadingMappings.value = true
 try {
 const response = await loadWebsiteUrlMappings()
 websiteUrlMappings.value = response.data || []
 } catch (error) {
 message.error(error?.message || '读取网站地址映射失败')
 } finally {
 loadingMappings.value = false
 }
}

const handleSaveBridgeConfig = async () => {
 if (!canSaveBridgeConfig.value) {
 message.warning('插件 ID 应为 32 位小写 a-p 字符')
 return
 }

 savingBridgeConfig.value = true
 try {
 const response = await saveBrowserBridgeConfig({
 extensionId: bridgeForm.value.extensionId.trim()
 })
 bridgeForm.value.extensionId = response.data?.extensionId || bridgeForm.value.extensionId.trim()
 hasSavedBridgeConfig.value = true
 message.success('浏览器插件配置已保存')
 } catch (error) {
 message.error(error?.message || '保存浏览器插件配置失败')
 } finally {
 savingBridgeConfig.value = false
 }
}

const handleQuery = async () => {
 if (!canQuery.value) {
 message.warning('请填写查询条件')
 return
 }

 const ready = await ensureReady()
 if (!ready) {
 message.warning('请先配置可用的 MySQL 数据源')
 return
 }

 querying.value = true
 copiedKey.value = ''
 try {
 const response = await queryCertInfo({
 mainName: queryForm.value.mainName.trim(),
 businessType: queryForm.value.businessType.trim()
 })
 queryItems.value = response.data?.items || []

 if (queryItems.value.length === 0) {
 message.info('未查询到账号记录')
 } else {
 message.success(`查询完成，共 ${queryItems.value.length} 条`)
 }
 } catch (error) {
 message.error(error?.message || '查询失败')
 } finally {
 querying.value = false
 }
}

const formatCertInfo = (certInfo) => {
 if (!certInfo) {
 return ''
 }

 try {
 return JSON.stringify(JSON.parse(certInfo), null, 2)
 } catch {
 return certInfo
 }
}

const copyText = async (text, key) => {
 if (!text) {
 message.warning('暂无可复制内容')
 return
 }

 try {
 await navigator.clipboard.writeText(text)
 copiedKey.value = key
 message.success('已复制')
 } catch {
 message.error('复制失败')
 }
}

const canOpenItem = (item) => {
 return Boolean(
 hasSavedBridgeConfig.value &&
 item.account.areaId &&
 item.account.businessType &&
 item.cert?.certInfo
 )
}

const getWebsiteUrlMapping = (item) => {
 const areaId = String(item.account.areaId || '').trim()
 const businessType = String(item.account.businessType || '').trim()

 if (!areaId || !businessType) {
 return null
 }

 return websiteUrlMappings.value.find(mapping =>
 String(mapping.areaId || '').trim() === areaId &&
 String(mapping.businessType || '').trim() === businessType
 ) || null
}

const hasWebsiteUrlMapping = (item) => {
 const mapping = getWebsiteUrlMapping(item)
 return Boolean(mapping && String(mapping.url || '').trim())
}

const openMappingModal = (item) => {
 const existing = getWebsiteUrlMapping(item)
 mappingForm.value = {
 areaId: String(item.account.areaId || '').trim(),
 businessType: String(item.account.businessType || '').trim(),
 url: existing?.url || '',
 storageRules: existing?.storageRules ? JSON.parse(JSON.stringify(existing.storageRules)) : []
 }
 mappingModalVisible.value = true
}

const addStorageRule = () => {
 mappingForm.value.storageRules.push({
 storage: 'sessionStorage',
 key: '',
 source: { path: '', value: '' }
 })
}

const removeStorageRule = (index) => {
 mappingForm.value.storageRules.splice(index, 1)
}

const moveStorageRule = (index, direction) => {
 const rules = mappingForm.value.storageRules
 const newIndex = index + direction
 if (newIndex < 0 || newIndex >= rules.length) return
 const temp = rules[index]
 rules[index] = rules[newIndex]
 rules[newIndex] = temp
}

const normalizeStorageRules = (rules) => {
 return rules
 .filter(rule => rule.key.trim())
 .map(rule => {
 const normalized = {
 storage: rule.storage,
 key: rule.key.trim(),
 source: {}
 }
 if (rule.source.path.trim()) {
 normalized.source.path = rule.source.path.trim()
 } else if (rule.source.value.trim()) {
 normalized.source.value = rule.source.value.trim()
 }
 return normalized
 })
 .filter(rule => rule.source.path || rule.source.value)
}

const handleSaveMapping = async () => {
 if (!canSaveMapping.value) {
 message.warning('请填写首页地址')
 return
 }

 savingMapping.value = true
 try {
 const normalizedRules = normalizeStorageRules(mappingForm.value.storageRules)
 const response = await saveWebsiteUrlMapping({
 areaId: mappingForm.value.areaId.trim(),
 businessType: mappingForm.value.businessType.trim(),
 url: mappingForm.value.url.trim(),
 storageRules: normalizedRules.length > 0 ? normalizedRules : undefined
 })
 websiteUrlMappings.value = response.data || []
 mappingModalVisible.value = false
 message.success('配置已保存')
 } catch (error) {
 message.error(error?.message || '保存配置失败')
 } finally {
 savingMapping.value = false
 }
}

const handleOpenDefaultBrowser = async (item, index) => {
 if (!canOpenItem(item)) {
 message.warning('请确认插件 ID、area_id、办理类型和 cert_info 均已存在')
 return
 }

 if (!hasWebsiteUrlMapping(item)) {
 message.warning('请先配置网站地址')
 openMappingModal(item)
 return
 }

 const key = `${item.account.loginKey}-${index}-open`
 openingKey.value = key
 try {
 const response = await openDefaultBrowserWithCookies({
 areaId: item.account.areaId,
 businessType: item.account.businessType,
 certInfo: item.cert.certInfo
 })
 const written = response.data?.written ?? 0
 message.success(`已打开默认浏览器，写入 ${written} 个 Cookie`)
 } catch (error) {
 message.error(error?.message || '默认浏览器打开失败')
 } finally {
 openingKey.value = ''
 }
}

const handleToggleTheme = () => {
 toggleTheme()
}

onMounted(() => {
 loadConfig()
 loadBridgeConfig()
 loadMappings()
})
</script>

<template>
 <main class="cert-query-page">
 <header class="page-header">
 <div class="header-left">
 <n-tooltip trigger="hover">
 <template #trigger>
 <router-link class="icon-link" to="/">
 <n-icon><ArrowBackOutline /></n-icon>
 </router-link>
 </template>
 返回首页
 </n-tooltip>

 <div class="title-mark">
 <n-icon :size="23"><KeyOutline /></n-icon>
 </div>

 <div class="title-copy">
 <h1>在线凭证查询</h1>
 <n-text depth="3">查询条件</n-text>
 </div>
 </div>

 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button quaternary circle @click="handleToggleTheme">
 <template #icon>
 <n-icon>
 <MoonOutline v-if="!isDarkMode" />
 <SunnyOutline v-else />
 </n-icon>
 </template>
 </n-button>
 </template>
 {{ isDarkMode ? '切换到亮色模式' : '切换到暗色模式' }}
 </n-tooltip>
 </header>

 <section class="page-shell">
 <div class="form-area">
 <section class="panel">
 <div class="panel-heading">
 <h2>查询条件</h2>
 </div>

 <div class="query-grid">
 <label class="field-block">
 <span>主体名</span>
 <n-input
 v-model:value="queryForm.mainName"
 clearable
 @keydown.enter.prevent="handleQuery"
 />
 </label>

 <label class="field-block">
 <span>办理类型</span>
 <n-select
 v-model:value="queryForm.businessType"
 :options="businessTypeOptions"
 placeholder="请选择办理类型"
 clearable
 filterable
 @keydown.enter.prevent="handleQuery"
 />
 </label>
 </div>

 <div class="panel-actions">
 <n-button
 type="primary"
 :disabled="!canQuery"
 :loading="querying"
 @click="handleQuery"
 >
 <template #icon>
 <n-icon><SearchOutline /></n-icon>
 </template>
 查询
 </n-button>
 </div>
 </section>

 <section class="panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>数据库连接</h2>
 <n-text depth="3">与首页“数据库配置”共用同一份 MySQL 数据源</n-text>
 </div>
 <n-tag :type="statusTagType" size="small">
 {{ statusLabel }}
 </n-tag>
 </div>

 <div class="panel-actions">
 <n-button
 type="primary"
 secondary
 @click="openModal"
 >
 <template #icon>
 <n-icon><SaveOutline /></n-icon>
 </template>
 打开数据库配置
 </n-button>
 </div>
 </section>

 <section class="panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>浏览器插件</h2>
 <n-text depth="3">
 {{ hasSavedBridgeConfig ? '已保存插件 ID' : '未配置插件 ID' }}
 · {{ mappingCountLabel }}
 </n-text>
 </div>
 <n-tag :type="hasSavedBridgeConfig ? 'success' : 'warning'" size="small">
 {{ hasSavedBridgeConfig ? '可打开' : '待配置' }}
 </n-tag>
 </div>

 <label class="field-block">
 <span>插件 ID</span>
 <n-input
 v-model:value="bridgeForm.extensionId"
 placeholder="Chrome/Edge 扩展页面中的 32 位插件 ID"
 clearable
 />
 </label>

 <div class="panel-actions">
 <n-button
 type="primary"
 :disabled="!canSaveBridgeConfig"
 :loading="savingBridgeConfig"
 @click="handleSaveBridgeConfig"
 >
 <template #icon>
 <n-icon><SaveOutline /></n-icon>
 </template>
 保存插件 ID
 </n-button>
 </div>
 </section>
 </div>

 <aside class="result-area">
 <section class="panel result-panel">
 <div class="panel-heading panel-heading-row">
 <div>
 <h2>查询结果</h2>
 <n-text depth="3">{{ resultCountLabel }}</n-text>
 </div>
 </div>

 <div v-if="queryItems.length" class="result-list">
 <article
 v-for="(item, index) in queryItems"
 :key="`${item.account.loginKey}-${index}`"
 class="result-item"
 >
 <div class="result-item-header">
 <div class="result-title">
 <strong>{{ item.account.companyName || queryForm.mainName }}</strong>
 <n-tag size="small" :type="item.cert ? 'success' : 'warning'">
 {{ item.cert ? 'valid=1' : '无有效 cert' }}
 </n-tag>
 </div>
 <n-button
 secondary
 size="small"
 :disabled="!item.cert?.certInfo"
 @click="copyText(item.cert?.certInfo, `${item.account.loginKey}-cert`)"
 >
 <template #icon>
 <n-icon>
 <ClipboardOutline
 v-if="copiedKey === `${item.account.loginKey}-cert`"
 />
 <CopyOutline v-else />
 </n-icon>
 </template>
 {{ copiedKey === `${item.account.loginKey}-cert` ? '已复制' : '复制 cert_info' }}
 </n-button>
 <n-button
 secondary
 size="small"
 @click="openMappingModal(item)"
 >
 <template #icon>
 <n-icon><SaveOutline /></n-icon>
 </template>
 配置地址
 </n-button>
 <n-button
 type="primary"
 size="small"
 :disabled="!canOpenItem(item)"
 :loading="openingKey === `${item.account.loginKey}-${index}-open`"
 @click="handleOpenDefaultBrowser(item, index)"
 >
 <template #icon>
 <n-icon><OpenOutline /></n-icon>
 </template>
 默认浏览器打开
 </n-button>
 </div>

 <dl class="meta-grid">
 <div>
 <dt>login_key</dt>
 <dd>{{ item.account.loginKey }}</dd>
 </div>
 <div>
 <dt>办理类型</dt>
 <dd>{{ item.account.businessType || '-' }}</dd>
 </div>
 <div>
 <dt>dwbh</dt>
 <dd>{{ item.account.dwbh || '-' }}</dd>
 </div>
 <div>
 <dt>area_id</dt>
 <dd>{{ item.account.areaId || '-' }}</dd>
 </div>
 <div>
 <dt>更新时间</dt>
 <dd>{{ item.cert?.updateTime || '-' }}</dd>
 </div>
 </dl>

 <pre v-if="item.cert?.certInfo" class="cert-info">{{ formatCertInfo(item.cert.certInfo) }}</pre>
 <n-empty v-else class="empty-cert" description="未查询到有效 cert_info" />
 </article>
 </div>

 <n-empty
 v-else
 class="empty-result"
 :description="querying ? '查询中' : '暂无结果'"
 />
 </section>
 </aside>
 </section>

 <n-modal
 v-model:show="mappingModalVisible"
 preset="card"
 title="配置网站地址和存储规则"
 style="width: min(640px, calc(100vw - 32px))"
 >
 <div class="mapping-modal-form">
 <label class="field-block">
 <span>area_id</span>
 <n-input
 v-model:value="mappingForm.areaId"
 readonly
 />
 </label>

 <label class="field-block">
 <span>办理类型</span>
 <n-input
 v-model:value="mappingForm.businessType"
 readonly
 />
 </label>

 <label class="field-block">
 <span>登录地址</span>
 <n-input
 v-model:value="mappingForm.url"
 placeholder="https://example.com/login"
 clearable
 />
 </label>

 <div class="storage-rules-section">
 <div class="storage-rules-header">
 <span>存储写入规则</span>
 <n-button size="small" @click="addStorageRule">
 + 添加规则
 </n-button>
 </div>

 <div v-if="mappingForm.storageRules.length === 0" class="storage-rules-empty">
 暂无规则，点击上方按钮添加
 </div>

 <div
 v-for="(rule, index) in mappingForm.storageRules"
 :key="index"
 class="storage-rule-item"
 >
 <div class="storage-rule-row">
 <label class="field-inline">
 <span>存储类型</span>
 <n-select
 v-model:value="rule.storage"
 :options="storageTypeOptions"
 size="small"
 style="width: 160px"
 />
 </label>

 <label class="field-inline">
 <span>键名</span>
 <n-input
 v-model:value="rule.key"
 placeholder="key"
 size="small"
 style="width: 160px"
 />
 </label>

 <div class="storage-rule-actions">
 <n-button
 size="small"
 quaternary
n :disabled="index === 0"
 @click="moveStorageRule(index, -1)"
 >
 ↑
 </n-button>
 <n-button
 size="small"
 quaternary
 :disabled="index === mappingForm.storageRules.length - 1"
 @click="moveStorageRule(index, 1)"
 >
 ↓
 </n-button>
 <n-button
 size="small"
 quaternary
 type="error"
 @click="removeStorageRule(index)"
 >
 删除
 </n-button>
 </div>
 </div>

 <div class="storage-rule-row">
 <label class="field-inline">
 <span>来源</span>
 <n-select
 :value="rule.source.path ? 'path' : 'value'"
 :options="sourceTypeOptions"
 size="small"
 style="width: 160px"
 @update:value="(val) => {
 if (val === 'path') {
 rule.source.path = rule.source.value || ''
 rule.source.value = ''
 } else {
 rule.source.value = rule.source.path || ''
 rule.source.path = ''
 }
 }"
 />
 </label>

 <label class="field-inline" style="flex: 1">
 <span>{{ rule.source.path ? '路径' : '值' }}</span>
 <n-input
 v-if="rule.source.path"
 v-model:value="rule.source.path"
 placeholder="token 或 data.user.token"
 size="small"
 />
 <n-input
 v-else
 v-model:value="rule.source.value"
 placeholder="固定值"
 size="small"
 />
 </label>
 </div>
 </div>
 </div>

 <div class="modal-actions">
 <n-button @click="mappingModalVisible = false">
 取消
 </n-button>
 <n-button
 type="primary"
 :disabled="!canSaveMapping"
 :loading="savingMapping"
 @click="handleSaveMapping"
 >
 保存
 </n-button>
 </div>
 </div>
 </n-modal>
 </main>
</template>

<style scoped>
.cert-query-page {
 width: 100%;
 height: 100%;
 min-height: 0;
 display: flex;
 flex-direction: column;
 overflow: hidden;
 color: var(--n-text-color-1, #333639);
 background:
 linear-gradient(180deg, rgba(240, 160, 32, 0.08), transparent 280px),
 var(--n-body-color, #f5f7fa);
}

.page-header {
 height: 64px;
 flex: none;
 padding: 0 28px;
 display: flex;
 align-items: center;
 justify-content: space-between;
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
 background-color: color-mix(in srgb, var(--n-card-color, #ffffff) 88%, transparent);
}

.header-left {
 min-width: 0;
 display: flex;
 align-items: center;
 gap: 12px;
}

.icon-link {
 width: 34px;
 height: 34px;
 flex: none;
 display: grid;
 place-items: center;
 border-radius: 6px;
 color: var(--n-text-color-2, #666666);
 transition: background-color 0.15s ease, color 0.15s ease;
}

.icon-link:hover {
 color: var(--n-primary-color, #18a058);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
}

.title-mark {
 width: 40px;
 height: 40px;
 flex: none;
 display: grid;
 place-items: center;
 border-radius: 8px;
 color: #ffffff;
 background-color: #f0a020;
}

.title-copy {
 min-width: 0;
}

.title-copy h1 {
 margin-bottom: 2px;
 font-size: 18px;
 line-height: 1.2;
}

.page-shell {
 width: min(1240px, calc(100vw - 48px));
 margin: 0 auto;
 padding: 32px 0;
 flex: 1;
 min-height: 0;
 overflow-y: auto;
 overflow-x: hidden;
 display: grid;
 grid-template-columns: minmax(360px, 440px) minmax(0, 1fr);
 gap: 18px;
 align-items: start;
}

.form-area {
 min-width: 0;
 display: grid;
 gap: 18px;
}

.result-area {
 min-width: 0;
}

.panel {
 min-width: 0;
 padding: 20px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 background-color: var(--n-card-color, #ffffff);
 box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.05));
}

.panel-heading {
 margin-bottom: 16px;
}

.panel-heading h2 {
 font-size: 18px;
 line-height: 1.25;
}

.panel-heading-row {
 display: flex;
 align-items: center;
 justify-content: space-between;
 gap: 12px;
}

.datasource-grid,
.query-grid {
 display: grid;
 gap: 14px;
}

.datasource-grid {
 grid-template-columns: minmax(0, 1fr) 132px;
 opacity: 1;
 transition: opacity 0.15s ease;
}

.datasource-grid.is-loading {
 opacity: 0.6;
}

.password-field {
 grid-column: 1 / -1;
}

.field-block {
 min-width: 0;
 display: grid;
 gap: 7px;
}

.field-block > span {
 font-size: 13px;
 font-weight: 600;
 color: var(--n-text-color-2, #666666);
}

.field-block :deep(.n-input-number) {
 width: 100%;
}

.panel-actions {
 margin-top: 18px;
 display: flex;
 flex-wrap: wrap;
 justify-content: flex-end;
 gap: 8px;
}

.result-panel {
 min-height: 520px;
}

.result-list {
 display: grid;
 gap: 14px;
}

.result-item {
 min-width: 0;
 padding: 16px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.025));
}

.result-item-header {
 display: flex;
 align-items: flex-start;
 justify-content: space-between;
 gap: 12px;
}

.result-title {
 min-width: 0;
 display: flex;
 align-items: center;
 flex-wrap: wrap;
 gap: 8px;
}

.result-title strong {
 min-width: 0;
 overflow-wrap: anywhere;
 font-size: 15px;
}

.meta-grid {
 margin-top: 14px;
 display: grid;
 grid-template-columns: repeat(2, minmax(0, 1fr));
 gap: 8px;
}

.meta-grid > div {
 min-width: 0;
 padding: 9px 10px;
 border-radius: 6px;
 background-color: var(--n-card-color, #ffffff);
}

.meta-grid dt {
 margin-bottom: 3px;
 font-size: 12px;
 color: var(--n-text-color-3, #999999);
}

.meta-grid dd {
 min-width: 0;
 overflow-wrap: anywhere;
 font-family: var(--font-family-mono, Consolas, monospace);
 font-size: 12px;
 color: var(--n-text-color-1, #333639);
}

.cert-info {
 max-height: 360px;
 margin-top: 12px;
 padding: 12px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 overflow: auto;
 white-space: pre-wrap;
 overflow-wrap: anywhere;
 color: var(--n-text-color-1, #333639);
 background-color: var(--n-card-color, #ffffff);
 font-family: var(--font-family-mono, Consolas, monospace);
 font-size: 12px;
 line-height: 1.6;
 user-select: text;
}

.empty-result,
.empty-cert {
 display: grid;
 place-items: center;
 border: 1px dashed var(--n-border-color, #e0e0e6);
 border-radius: 8px;
}

.empty-result {
 min-height: 420px;
}

.empty-cert {
 min-height: 120px;
 margin-top: 12px;
}

.mapping-modal-form {
 display: grid;
 gap: 14px;
}

.modal-actions {
 display: flex;
 justify-content: flex-end;
 gap: 8px;
 margin-top: 6px;
}

.storage-rules-section {
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 padding: 14px;
}

.storage-rules-header {
 display: flex;
 align-items: center;
 justify-content: space-between;
 margin-bottom: 12px;
 font-size: 13px;
 font-weight: 600;
 color: var(--n-text-color-2, #666666);
}

.storage-rules-empty {
 text-align: center;
 padding: 16px;
 color: var(--n-text-color-3, #999999);
 font-size: 13px;
}

.storage-rule-item {
 padding: 12px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 6px;
 margin-bottom: 10px;
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.02));
}

.storage-rule-item:last-child {
 margin-bottom: 0;
}

.storage-rule-row {
 display: flex;
 align-items: flex-end;
 gap: 10px;
 margin-bottom: 10px;
}

.storage-rule-row:last-child {
 margin-bottom: 0;
}

.field-inline {
 display: flex;
 flex-direction: column;
 gap: 4px;
}

.field-inline > span {
 font-size: 12px;
 color: var(--n-text-color-3, #999999);
}

.storage-rule-actions {
 display: flex;
 gap: 4px;
 margin-left: auto;
}

@media (max-width: 960px) {
 .page-shell {
 grid-template-columns: 1fr;
 }
}

@media (max-width: 720px) {
 .page-header {
 height: 56px;
 padding: 0 16px;
 }

 .title-mark {
 width: 34px;
 height: 34px;
 }

 .title-copy h1 {
 font-size: 16px;
 }

 .title-copy :deep(.n-text) {
 display: none;
 }

 .page-shell {
 width: calc(100vw - 28px);
 padding: 20px 0;
 }

 .panel {
 padding: 16px;
 }

 .datasource-grid,
 .meta-grid {
 grid-template-columns: 1fr;
 }

 .result-item-header,
 .panel-heading-row {
 align-items: flex-start;
 }
}

@media (max-width: 420px) {
 .panel-actions {
 flex-direction: column;
 }

 .panel-actions :deep(.n-button),
 .result-item-header :deep(.n-button) {
 width: 100%;
 }

 .result-item-header {
 flex-direction: column;
 }
}
</style>
