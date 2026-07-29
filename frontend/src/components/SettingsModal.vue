<script setup>
import { computed, ref, watch } from 'vue'
import {
  NButton,
  NCheckbox,
  NIcon,
  NInput,
  NInputNumber,
  NModal,
  NRadio,
  NRadioGroup,
  NSpace,
  NTabPane,
  NTabs,
  NTag,
  NText,
  useMessage
} from 'naive-ui'
import {
  AddOutline,
  OpenOutline,
  RemoveOutline,
  SaveOutline,
  ServerOutline
} from '@vicons/ionicons5'
import { useAppSettings } from '@/composables/useAppSettings'
import { useMysqlDatasourceConfig } from '@/composables/useMysqlDatasourceConfig'
import { useHttpProxyConfig } from '@/composables/useHttpProxyConfig'
import {
  loadBrowserBridgeConfig,
  saveBrowserBridgeConfig
} from '@/api/certQuery'
import {
  loadYunshengAuthToken,
  loginYunsheng,
  openYunshengBrowserWithCookies,
  saveYunshengAuthToken
} from '@/api/yunshengAuth'

const message = useMessage()
const { settingsVisible, settingsTab, closeSettings } = useAppSettings()

const {
  datasourceForm,
  hasSavedDatasource,
  hasSavedPassword,
  loadingConfig: loadingDatasource,
  savingDatasource,
  testingDatasource,
  connectionMessage,
  canSaveDatasource,
  statusLabel: dbStatusLabel,
  statusTagType: dbStatusTagType,
  checkConnection,
  saveConfig: saveDatasourceConfig,
  loadConfig: loadDatasourceConfig
} = useMysqlDatasourceConfig()

const {
  MODE_OPTIONS,
  form: proxyForm,
  loading: loadingProxy,
  saving: savingProxy,
  isCustom,
  canSave: canSaveProxy,
  saveConfig: saveProxyConfig,
  loadConfig: loadProxyConfig
} = useHttpProxyConfig()

const bridgeForm = ref({ extensionId: '' })
const hasSavedBridgeConfig = ref(false)
const loadingBridge = ref(false)
const savingBridge = ref(false)

const canSaveBridgeConfig = computed(() =>
  /^[a-p]{32}$/.test(String(bridgeForm.value.extensionId || '').trim())
)

const defaultCookieFile = () => ({
  path: '',
  enabled: false
})

const yunshengForm = ref({
  account: '',
  password: '',
  cookies: '',
  cookieFiles: [defaultCookieFile()],
  openBrowserOnLogin: false
})
const loadingYunsheng = ref(false)
const savingYunsheng = ref(false)
const loggingInYunsheng = ref(false)
const openingBrowser = ref(false)

const applyYunshengConfig = (data) => {
  const files = Array.isArray(data?.cookieFiles) && data.cookieFiles.length
    ? data.cookieFiles.map((item) => ({
        path: String(item?.path || ''),
        enabled: !!item?.enabled
      }))
    : [defaultCookieFile()]
  yunshengForm.value = {
    account: data?.account || '',
    password: data?.password || '',
    cookies: data?.cookies || '',
    cookieFiles: files,
    openBrowserOnLogin: !!data?.openBrowserOnLogin
  }
}

const loadBridgeConfig = async () => {
  loadingBridge.value = true
  try {
    const response = await loadBrowserBridgeConfig()
    bridgeForm.value.extensionId = response.data?.extensionId || ''
    hasSavedBridgeConfig.value = Boolean(bridgeForm.value.extensionId)
  } catch (error) {
    message.error(error?.message || '读取浏览器插件配置失败')
  } finally {
    loadingBridge.value = false
  }
}

const loadYunshengConfig = async () => {
  loadingYunsheng.value = true
  try {
    const response = await loadYunshengAuthToken()
    applyYunshengConfig(response.data)
  } catch (error) {
    message.error(error?.message || '读取云生鉴权配置失败')
  } finally {
    loadingYunsheng.value = false
  }
}

const loadTabData = async (tab) => {
  if (tab === 'database') {
    await loadDatasourceConfig({ force: true })
  } else if (tab === 'proxy') {
    await loadProxyConfig({ force: true })
  } else if (tab === 'browser') {
    await loadBridgeConfig()
  } else if (tab === 'yunsheng') {
    await loadYunshengConfig()
  }
}

watch(
  () => [settingsVisible.value, settingsTab.value],
  ([visible, tab]) => {
    if (visible) {
      loadTabData(tab)
    }
  }
)

const handleSaveDatasource = async () => {
  if (!canSaveDatasource.value) {
    message.warning('请补全数据源配置')
    return
  }
  try {
    const connected = await saveDatasourceConfig()
    message.success(connected ? '数据源配置已保存，连接正常' : '数据源配置已保存，但连接失败')
  } catch (error) {
    message.error(error?.message || '保存数据源配置失败')
  }
}

const handleTestDatasource = async () => {
  const connected = await checkConnection()
  if (connected) {
    message.success('MySQL 连接正常')
  } else {
    message.error(connectionMessage.value || '测试 MySQL 连接失败')
  }
}

const handleSaveProxy = async () => {
  if (!canSaveProxy.value) {
    message.warning('指定代理模式下请填写代理地址')
    return
  }
  try {
    await saveProxyConfig()
    message.success('代理配置已保存并生效')
  } catch (error) {
    message.error(error?.message || '保存代理配置失败')
  }
}

const handleSaveBridge = async () => {
  if (!canSaveBridgeConfig.value) {
    message.warning('请填写 32 位 a-p 格式的浏览器插件 ID')
    return
  }
  savingBridge.value = true
  try {
    const response = await saveBrowserBridgeConfig({
      extensionId: bridgeForm.value.extensionId.trim()
    })
    bridgeForm.value.extensionId = response.data?.extensionId || bridgeForm.value.extensionId.trim()
    hasSavedBridgeConfig.value = Boolean(bridgeForm.value.extensionId)
    message.success('浏览器插件配置已保存')
  } catch (error) {
    message.error(error?.message || '保存浏览器插件配置失败')
  } finally {
    savingBridge.value = false
  }
}

const addCookieFile = () => {
  yunshengForm.value.cookieFiles.push(defaultCookieFile())
}

const removeCookieFile = (index) => {
  if (yunshengForm.value.cookieFiles.length <= 1) {
    yunshengForm.value.cookieFiles[0] = defaultCookieFile()
    return
  }
  yunshengForm.value.cookieFiles.splice(index, 1)
}

const handleSaveYunsheng = async () => {
  savingYunsheng.value = true
  try {
    const payload = {
      account: String(yunshengForm.value.account || '').trim(),
      password: String(yunshengForm.value.password || '').trim(),
      cookies: String(yunshengForm.value.cookies || '').trim(),
      cookieFiles: yunshengForm.value.cookieFiles.map((item) => ({
        path: String(item.path || '').trim(),
        enabled: !!item.enabled
      })),
      openBrowserOnLogin: !!yunshengForm.value.openBrowserOnLogin
    }
    const response = await saveYunshengAuthToken(payload)
    applyYunshengConfig(response.data)
    message.success(
      payload.cookies
        ? '云生配置已保存（已按勾选同步 Cookie 文件，未打开浏览器）'
        : '云生配置已保存'
    )
  } catch (error) {
    message.error(error?.message || '保存云生配置失败')
  } finally {
    savingYunsheng.value = false
  }
}

const handleLoginYunsheng = async () => {
  const account = String(yunshengForm.value.account || '').trim()
  const password = String(yunshengForm.value.password || '')
  if (!account || !password) {
    message.warning('请先填写账号和密码')
    return
  }
  loggingInYunsheng.value = true
  try {
    // 先落盘账号与勾选偏好，再立即登录，保证副作用按最新勾选执行
    const cookies = String(yunshengForm.value.cookies || '').trim()
    const payload = {
      account,
      password,
      cookies: !cookies || cookies.includes('token_inner=') ? cookies : '',
      cookieFiles: yunshengForm.value.cookieFiles.map((item) => ({
        path: String(item.path || '').trim(),
        enabled: !!item.enabled
      })),
      openBrowserOnLogin: !!yunshengForm.value.openBrowserOnLogin
    }
    await saveYunshengAuthToken(payload)
    const response = await loginYunsheng()
    applyYunshengConfig(response.data)
    message.success('登录成功，Cookie 已更新')
  } catch (error) {
    message.error(error?.message || '登录失败')
  } finally {
    loggingInYunsheng.value = false
  }
}

const handleOpenYunshengBrowser = async () => {
  const cookieHeader = String(yunshengForm.value.cookies || '').trim()
  if (!cookieHeader) {
    message.warning('请先填写 Cookie')
    return
  }
  openingBrowser.value = true
  try {
    const response = await openYunshengBrowserWithCookies(cookieHeader)
    const written = response.data?.written ?? 0
    message.success(`已打开浏览器并写入 ${written} 条 Cookie`)
  } catch (error) {
    message.error(error?.message || '打开浏览器失败')
  } finally {
    openingBrowser.value = false
  }
}
</script>

<template>
  <n-modal
    v-model:show="settingsVisible"
    preset="card"
    title="设置"
    style="width: min(820px, calc(100vw - 32px))"
    :mask-closable="false"
    @close="closeSettings"
  >
    <n-tabs v-model:value="settingsTab" type="line" animated>
      <n-tab-pane name="database" tab="数据库">
        <div class="tab-panel" :class="{ 'is-loading': loadingDatasource }">
          <div class="status-row">
            <div>
              <n-text depth="3">
                {{ hasSavedDatasource ? '已保存 MySQL 数据源' : '未保存 MySQL 数据源' }}
                <template v-if="hasSavedPassword"> · 密码已保存</template>
              </n-text>
              <n-text
                v-if="connectionMessage"
                depth="3"
                class="connection-message"
              >
                {{ connectionMessage }}
              </n-text>
            </div>
            <n-tag :type="dbStatusTagType" size="small">
              {{ dbStatusLabel }}
            </n-tag>
          </div>

          <div class="datasource-grid">
            <label class="field-block">
              <span>地址</span>
              <n-input v-model:value="datasourceForm.host" placeholder="127.0.0.1" clearable />
            </label>
            <label class="field-block">
              <span>端口</span>
              <n-input-number
                v-model:value="datasourceForm.port"
                :min="1"
                :max="65535"
                placeholder="3306"
              />
            </label>
            <label class="field-block">
              <span>数据库</span>
              <n-input v-model:value="datasourceForm.database" placeholder="database" clearable />
            </label>
            <label class="field-block">
              <span>账号</span>
              <n-input v-model:value="datasourceForm.username" placeholder="username" clearable />
            </label>
            <label class="field-block password-field">
              <span>密码</span>
              <n-input
                v-model:value="datasourceForm.password"
                type="password"
                show-password-on="click"
                :placeholder="hasSavedPassword ? '留空沿用已保存密码' : 'password'"
                clearable
              />
            </label>
            <label class="field-block">
              <span>超时秒数</span>
              <n-input-number
                v-model:value="datasourceForm.connectTimeoutSeconds"
                :min="1"
                :max="60"
                placeholder="8"
              />
            </label>
          </div>

          <div class="modal-actions">
            <n-button
              secondary
              :disabled="!hasSavedDatasource"
              :loading="testingDatasource"
              @click="handleTestDatasource"
            >
              <template #icon>
                <n-icon><ServerOutline /></n-icon>
              </template>
              测试连接
            </n-button>
            <n-button
              type="primary"
              :disabled="!canSaveDatasource"
              :loading="savingDatasource"
              @click="handleSaveDatasource"
            >
              <template #icon>
                <n-icon><SaveOutline /></n-icon>
              </template>
              保存并测试
            </n-button>
          </div>
        </div>
      </n-tab-pane>

      <n-tab-pane name="proxy" tab="网络代理">
        <div class="tab-panel" :class="{ 'is-loading': loadingProxy }">
          <n-text depth="3" class="hint">
            作用于云生、云效流水线监控、OSS 互转等出站 HTTP 请求；保存后立即生效，无需重启。
          </n-text>

          <div class="field-block">
            <span>代理模式</span>
            <n-radio-group v-model:value="proxyForm.mode" name="proxy-mode">
              <n-space vertical :size="10">
                <n-radio
                  v-for="item in MODE_OPTIONS"
                  :key="item.value"
                  :value="item.value"
                >
                  <div class="mode-item">
                    <strong>{{ item.label }}</strong>
                    <n-text depth="3">{{ item.hint }}</n-text>
                  </div>
                </n-radio>
              </n-space>
            </n-radio-group>
          </div>

          <label class="field-block">
            <span>代理地址</span>
            <n-input
              v-model:value="proxyForm.url"
              :disabled="!isCustom"
              placeholder="http://127.0.0.1:7890"
              clearable
            />
            <n-text depth="3" class="field-hint">
              仅支持 HTTP/HTTPS 代理；如需账号可写在地址中：http://user:pass@host:port
            </n-text>
          </label>

          <div class="modal-actions">
            <n-button
              type="primary"
              :disabled="!canSaveProxy"
              :loading="savingProxy"
              @click="handleSaveProxy"
            >
              <template #icon>
                <n-icon><SaveOutline /></n-icon>
              </template>
              保存并生效
            </n-button>
          </div>
        </div>
      </n-tab-pane>

      <n-tab-pane name="browser" tab="浏览器插件">
        <div class="tab-panel" :class="{ 'is-loading': loadingBridge }">
          <div class="status-row">
            <n-text depth="3">
              {{ hasSavedBridgeConfig ? '已保存插件 ID' : '未配置插件 ID' }}
              · 用于凭证查询打开网站与云生 Cookie 桥接
            </n-text>
            <n-tag :type="hasSavedBridgeConfig ? 'success' : 'warning'" size="small">
              {{ hasSavedBridgeConfig ? '已配置' : '待配置' }}
            </n-tag>
          </div>

          <label class="field-block">
            <span>插件 ID</span>
            <n-input
              v-model:value="bridgeForm.extensionId"
              placeholder="Chrome/Edge 扩展页面中的 32 位插件 ID（a-p）"
              clearable
            />
          </label>

          <div class="modal-actions">
            <n-button
              type="primary"
              :disabled="!canSaveBridgeConfig"
              :loading="savingBridge"
              @click="handleSaveBridge"
            >
              <template #icon>
                <n-icon><SaveOutline /></n-icon>
              </template>
              保存插件 ID
            </n-button>
          </div>
        </div>
      </n-tab-pane>

      <n-tab-pane name="yunsheng" tab="云生">
        <div class="tab-panel" :class="{ 'is-loading': loadingYunsheng }">
          <n-text depth="3" class="hint">
            可配置账号密码自动登录，或随时手动粘贴 Cookie。保存时按勾选同步到本地文件且不会自动打开浏览器；登录成功则按勾选写文件，并在勾选「登录后打开浏览器」时桥接打开 shebaorobot。
          </n-text>

          <div class="yunsheng-grid">
            <label class="field-block">
              <span>账号</span>
              <n-input v-model:value="yunshengForm.account" placeholder="云生账号" clearable />
            </label>
            <label class="field-block">
              <span>密码</span>
              <n-input
                v-model:value="yunshengForm.password"
                type="password"
                show-password-on="click"
                placeholder="明文保存于本机配置"
                clearable
              />
            </label>
          </div>

          <label class="field-block">
            <span>完整 Cookie</span>
            <n-input
              v-model:value="yunshengForm.cookies"
              type="textarea"
              :autosize="{ minRows: 3, maxRows: 8 }"
              placeholder="token_inner=eyJ..."
            />
          </label>

          <div class="field-block">
            <div class="cookie-files-heading">
              <span>Cookie 同步文件</span>
              <n-button size="tiny" quaternary @click="addCookieFile">
                <template #icon>
                  <n-icon><AddOutline /></n-icon>
                </template>
                添加路径
              </n-button>
            </div>
            <div
              v-for="(item, index) in yunshengForm.cookieFiles"
              :key="index"
              class="cookie-file-row"
            >
              <n-checkbox v-model:checked="item.enabled">启用</n-checkbox>
              <n-input
                v-model:value="item.path"
                placeholder="本机绝对路径，如 .../.boss_cookie"
                clearable
              />
              <n-button quaternary circle size="small" @click="removeCookieFile(index)">
                <template #icon>
                  <n-icon><RemoveOutline /></n-icon>
                </template>
              </n-button>
            </div>
          </div>

          <n-checkbox v-model:checked="yunshengForm.openBrowserOnLogin">
            登录成功后打开浏览器并写入 Cookie（shebaorobot）
          </n-checkbox>

          <div class="modal-actions yunsheng-actions">
            <n-button
              type="primary"
              secondary
              :loading="loggingInYunsheng"
              :disabled="!yunshengForm.account.trim() || !yunshengForm.password"
              @click="handleLoginYunsheng"
            >
              立即登录
            </n-button>
            <n-button
              secondary
              :loading="openingBrowser"
              :disabled="!yunshengForm.cookies.trim()"
              @click="handleOpenYunshengBrowser"
            >
              <template #icon>
                <n-icon><OpenOutline /></n-icon>
              </template>
              用当前 Cookie 打开浏览器
            </n-button>
            <n-button
              type="primary"
              :loading="savingYunsheng"
              @click="handleSaveYunsheng"
            >
              <template #icon>
                <n-icon><SaveOutline /></n-icon>
              </template>
              保存
            </n-button>
          </div>
        </div>
      </n-tab-pane>
    </n-tabs>

    <template #footer>
      <div class="footer-actions">
        <n-button @click="closeSettings">关闭</n-button>
      </div>
    </template>
  </n-modal>
</template>

<style scoped>
.tab-panel {
  display: grid;
  gap: 16px;
  padding-top: 8px;
  min-height: 220px;
}

.tab-panel.is-loading {
  opacity: 0.65;
  pointer-events: none;
}

.status-row {
  min-width: 0;
  padding: 12px;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  border-radius: 8px;
  background-color: var(--n-action-color, rgba(0, 0, 0, 0.04));
}

.connection-message {
  display: block;
  margin-top: 4px;
  overflow-wrap: anywhere;
}

.datasource-grid,
.yunsheng-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 132px;
  gap: 14px;
}

.yunsheng-grid {
  grid-template-columns: 1fr 1fr;
}

.password-field {
  grid-column: 1 / -1;
}

.field-block {
  min-width: 0;
  display: grid;
  gap: 7px;
}

.field-block > span,
.cookie-files-heading > span {
  font-size: 13px;
  font-weight: 600;
  color: var(--n-text-color-2, #666666);
}

.field-block :deep(.n-input-number) {
  width: 100%;
}

.hint,
.field-hint {
  display: block;
  line-height: 1.5;
  font-size: 12px;
}

.mode-item {
  display: grid;
  gap: 2px;
}

.mode-item strong {
  font-weight: 600;
}

.cookie-files-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.cookie-file-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 8px;
  align-items: center;
}

.modal-actions,
.footer-actions {
  display: flex;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 8px;
}

.yunsheng-actions {
  margin-top: 4px;
}

@media (max-width: 560px) {
  .datasource-grid,
  .yunsheng-grid {
    grid-template-columns: 1fr;
  }

  .cookie-file-row {
    grid-template-columns: 1fr;
  }

  .status-row {
    flex-direction: column;
  }

  .modal-actions :deep(.n-button) {
    width: 100%;
  }
}
</style>
