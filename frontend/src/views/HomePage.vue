<script setup>
import { computed, inject, onMounted, onBeforeUnmount, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
 NButton,
 NIcon,
 NTag,
 NText,
 NTooltip,
 useMessage
} from 'naive-ui'
import {
 KeyOutline,
 LayersOutline,
 MoonOutline,
 SunnyOutline,
 ChevronForwardOutline,
 FolderOpenOutline,
 ReceiptOutline,
 SearchOutline,
 ServerOutline,
 SwapHorizontalOutline,
 GitNetworkOutline,
 GlobeOutline
} from '@vicons/ionicons5'
import { useConfigStore } from '@/store'
import { invokeApi } from '@/api/tauriClient'
import { useMysqlDatasourceConfig } from '@/composables/useMysqlDatasourceConfig'
import { useHttpProxyConfig } from '@/composables/useHttpProxyConfig'
import { listen } from '@tauri-apps/api/event'
import { getPipelineMonitorSnapshot } from '@/api/pipelineMonitor'

const router = useRouter()
const configStore = useConfigStore()
const message = useMessage()

const isDarkMode = inject('isDarkMode', computed(() => configStore.isDarkMode))
const toggleTheme = inject('toggleTheme', () => configStore.toggleTheme())
const {
 testingDatasource,
 statusLabel,
 statusTagType,
 openModal,
 loadConfig,
 checkConnection
} = useMysqlDatasourceConfig()

const {
 statusLabel: proxyStatusLabel,
 openModal: openProxyModal,
 loadConfig: loadProxyConfig
} = useHttpProxyConfig()

const toolEntries = [
 {
 id: 'encrypt',
 title: '加解密工具',
 routeName: 'EncryptTool',
 icon: KeyOutline,
 status: '可用',
 description: '通过拖拽工作流组合编码、摘要、对称加密、非对称加密和国密算法。',
 capabilities: ['工作流编排', '本地 Rust 执行', '项目配置保存']
 },
 {
 id: 'log-expression',
 title: '日志语句生成',
 routeName: 'LogExpressionTool',
 icon: SearchOutline,
 status: '可用',
 description: '选择城市、业务类型和任务类型，实时生成日志平台查询表达式。',
 capabilities: ['分流规则', '关键词拼接', '一键复制']
 },
 {
 id: 'cert-query',
 title: '在线凭证查询',
 routeName: 'CertQueryTool',
 icon: ServerOutline,
 status: '可用',
 description: '按主体名和办理类型查询 MySQL，定位最新有效 cert_info。',
 capabilities: ['MySQL 数据源', '两段查询', '本地配置保存']
 },
 {
 id: 'robot-feedback',
 title: '任务反馈生成',
 routeName: 'RobotFeedbackTool',
 icon: ReceiptOutline,
 status: '可用',
 description: '按 task_id 和记录主键生成手动反馈 SQL 与 curl。',
 capabilities: ['更新 SQL', '反馈 curl', '一键复制']
 },
 {
 id: 'oss-transfer',
 title: 'OSS Key 转换',
 routeName: 'OssTransferTool',
 icon: SwapHorizontalOutline,
 status: '可用',
 description: '在生产环境与测试环境之间同步 OSS 文件，自动下载并重新上传。',
 capabilities: ['生产↔测试', '自动下载上传', '一键复制']
 },
 {
 id: 'pipeline-monitor',
 title: '流水线监控',
 routeName: 'PipelineMonitorTool',
 icon: GitNetworkOutline,
 status: '可用',
 description: '监控云效流水线人工卡点与分支选择，支持自动审批与后台轮询。',
 capabilities: ['多流水线', '人工卡点', '自动模式']
 }
]

const pipelinePendingCount = ref(0)
const pipelineMonitorMode = ref('idle')
let unlistenPipeline = null
let pipelinePollTimer = null

const pipelineStatusLabel = computed(() => {
  if (pipelineMonitorMode.value === 'loop') return '循环监控'
  if (pipelineMonitorMode.value === 'single') return '单次监控'
  if (pipelinePendingCount.value > 0) return `待办 ${pipelinePendingCount.value}`
  return '可用'
})

const refreshPipelineBadge = async () => {
  try {
    const { data } = await getPipelineMonitorSnapshot()
    pipelinePendingCount.value = data?.pendingCount || 0
    pipelineMonitorMode.value = data?.mode || (data?.running ? 'loop' : 'idle')
  } catch {
    // ignore
  }
}

const openTool = (tool) => {
 router.push({ name: tool.routeName })
}

const handleToggleTheme = () => {
 toggleTheme()
}

const handleOpenConfigDir = async () => {
 try {
 await invokeApi('open_app_config_dir')
 } catch (error) {
 message.error(error?.message || '打开本地配置目录失败')
 }
}

onMounted(async () => {
 const loaded = await loadConfig()
 if (loaded) {
 checkConnection()
 }
 await loadProxyConfig()
 await refreshPipelineBadge()
 try {
 unlistenPipeline = await listen('pipeline-monitor-state', (event) => {
 pipelinePendingCount.value = event.payload?.pendingCount || 0
 pipelineMonitorMode.value =
 event.payload?.mode || (event.payload?.running ? 'loop' : 'idle')
 })
 } catch {
 // ignore
 }
 pipelinePollTimer = setInterval(refreshPipelineBadge, 5000)
})

onBeforeUnmount(() => {
 if (unlistenPipeline) unlistenPipeline()
 if (pipelinePollTimer) clearInterval(pipelinePollTimer)
})
</script>

<template>
 <main class="home-page">
 <header class="home-header">
 <div class="brand">
 <div class="brand-mark">
 <n-icon :size="24"><LayersOutline /></n-icon>
 </div>
 <div class="brand-copy">
 <h1>Coter 本地工具台</h1>
 <n-text depth="3">统一入口，能力由 Rust/Tauri 本地命令提供</n-text>
 </div>
 </div>

 <div class="home-actions">
 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button secondary size="small" :loading="testingDatasource" @click="openModal">
 <template #icon>
 <n-icon><ServerOutline /></n-icon>
 </template>
 数据库配置
 <n-tag
 class="db-status-tag"
 :type="statusTagType"
 size="small"
 :bordered="false"
 >
 {{ statusLabel }}
 </n-tag>
 </n-button>
 </template>
 配置并测试 MySQL 数据源
 </n-tooltip>

 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button secondary size="small" @click="openProxyModal">
 <template #icon>
 <n-icon><GlobeOutline /></n-icon>
 </template>
 网络代理
 <n-tag class="db-status-tag" type="default" size="small" :bordered="false">
 {{ proxyStatusLabel }}
 </n-tag>
 </n-button>
 </template>
 配置出站 HTTP 代理（直连 / 系统代理 / 指定代理）
 </n-tooltip>

 <n-tooltip trigger="hover">
 <template #trigger>
 <n-button secondary size="small" @click="handleOpenConfigDir">
 <template #icon>
 <n-icon><FolderOpenOutline /></n-icon>
 </template>
 打开本地配置目录
 </n-button>
 </template>
 打开应用配置文件所在目录
 </n-tooltip>

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
 </div>
 </header>

 <section class="home-content">
 <div class="content-heading">
 <n-text depth="3" class="heading-label">TOOLS</n-text>
 <h2>选择工具</h2>
 </div>

 <div class="tool-grid">
 <button
 v-for="tool in toolEntries"
 :key="tool.id"
 class="tool-card"
 type="button"
 @click="openTool(tool)"
 >
 <div class="tool-card-top">
 <div class="tool-icon">
 <n-icon :size="28">
 <component :is="tool.icon" />
 </n-icon>
 </div>
 <span
 class="tool-status"
 :class="{
 'is-running':
 tool.id === 'pipeline-monitor' &&
 (pipelineMonitorMode === 'loop' || pipelineMonitorMode === 'single')
 }"
 >
 <template v-if="tool.id === 'pipeline-monitor'">
 {{ pipelineStatusLabel }}
 </template>
 <template v-else>
 {{ tool.status }}
 </template>
 </span>
 </div>

 <div class="tool-card-main">
 <h3>{{ tool.title }}</h3>
 <n-text depth="3">{{ tool.description }}</n-text>
 </div>

 <div class="tool-card-footer">
 <div class="capability-list">
 <span
 v-for="capability in tool.capabilities"
 :key="capability"
 class="capability"
 >
 {{ capability }}
 </span>
 </div>
 <n-icon :size="22" class="open-icon"><ChevronForwardOutline /></n-icon>
 </div>
 </button>
 </div>
 </section>
 </main>
</template>

<style scoped>
.home-page {
 width: 100%;
 height: 100%;
 min-height: 0;
 overflow: auto;
 color: var(--n-text-color-1, #333639);
 background:
 linear-gradient(180deg, rgba(24, 160, 88, 0.08), transparent 280px),
 var(--n-body-color, #f5f7fa);
}

.home-header {
 height: 64px;
 padding: 0 28px;
 display: flex;
 align-items: center;
 justify-content: space-between;
 border-bottom: 1px solid var(--n-border-color, #e0e0e6);
 background-color: color-mix(in srgb, var(--n-card-color, #ffffff) 88%, transparent);
}

.brand {
 min-width: 0;
 display: flex;
 align-items: center;
 gap: 12px;
}

.brand-mark {
 width: 40px;
 height: 40px;
 flex: none;
 display: grid;
 place-items: center;
 border-radius: 8px;
 color: #ffffff;
 background-color: var(--n-primary-color, #18a058);
}

.brand-copy {
 min-width: 0;
}

.brand-copy h1 {
 margin-bottom: 2px;
 font-size: 18px;
 line-height: 1.2;
}

.home-actions {
 display: flex;
 align-items: center;
 gap: 8px;
}

.db-status-tag {
 margin-left: 4px;
}

.home-content {
 width: min(1120px, calc(100vw - 48px));
 margin: 0 auto;
 padding: 40px 0;
}

.content-heading {
 margin-bottom: 18px;
}

.heading-label {
 display: block;
 margin-bottom: 6px;
 font-size: 12px;
 font-weight: 700;
}

.content-heading h2 {
 font-size: 24px;
 line-height: 1.25;
}

.tool-grid {
 display: grid;
 grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
 gap: 16px;
 align-items: stretch;
}

.tool-card {
 min-height: 236px;
 padding: 20px;
 display: flex;
 flex-direction: column;
 justify-content: space-between;
 gap: 18px;
 border: 1px solid var(--n-border-color, #e0e0e6);
 border-radius: 8px;
 background-color: var(--n-card-color, #ffffff);
 color: inherit;
 text-align: left;
 box-shadow: var(--shadow-sm, 0 1px 2px rgba(0, 0, 0, 0.05));
 transition: transform 0.15s ease, border-color 0.15s ease, box-shadow 0.15s ease;
}

.tool-card:hover {
 transform: translateY(-1px);
 border-color: var(--n-primary-color, #18a058);
 box-shadow: var(--shadow-md, 0 2px 8px rgba(0, 0, 0, 0.08));
}

.tool-card-top,
.tool-card-footer {
 display: flex;
 align-items: center;
 justify-content: space-between;
 gap: 12px;
}

.tool-icon {
 width: 52px;
 height: 52px;
 display: grid;
 place-items: center;
 border-radius: 8px;
 color: var(--n-primary-color, #18a058);
 background-color: rgba(24, 160, 88, 0.12);
}

.tool-status {
 padding: 3px 8px;
 border-radius: 999px;
 font-size: 12px;
 color: var(--n-primary-color, #18a058);
 background-color: rgba(24, 160, 88, 0.12);
}

.tool-status.is-running {
 color: #d03050;
 background-color: rgba(208, 48, 80, 0.12);
}

.tool-card-main {
 min-width: 0;
}

.tool-card-main h3 {
 margin-bottom: 8px;
 font-size: 18px;
 line-height: 1.25;
}

.capability-list {
 min-width: 0;
 display: flex;
 flex-wrap: wrap;
 gap: 6px;
}

.capability {
 max-width: 100%;
 padding: 3px 7px;
 border-radius: 6px;
 font-size: 12px;
 color: var(--n-text-color-2, #666666);
 background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
}

.open-icon {
 flex: none;
 color: var(--n-text-color-3, #999999);
}

@media (max-width: 720px) {
 .home-header {
 height: 56px;
 padding: 0 16px;
 }

 .brand-copy h1 {
 font-size: 16px;
 }

 .brand-copy :deep(.n-text) {
 display: none;
 }

 .home-content {
 width: calc(100vw - 28px);
 padding: 28px 0;
 }

 .home-actions :deep(.n-button__content) {
 display: none;
 }

 .tool-grid {
 grid-template-columns: 1fr;
 }

 .tool-card {
 min-height: 220px;
 padding: 18px;
 }
}
</style>
