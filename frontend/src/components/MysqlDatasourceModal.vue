<script setup>
import {
 NButton,
 NIcon,
 NInput,
 NInputNumber,
 NModal,
 NTag,
 NText,
 useMessage
} from 'naive-ui'
import {
 SaveOutline,
 ServerOutline
} from '@vicons/ionicons5'
import { useMysqlDatasourceConfig } from '@/composables/useMysqlDatasourceConfig'

const message = useMessage()

const {
 modalVisible,
 datasourceForm,
 hasSavedDatasource,
 hasSavedPassword,
 loadingConfig,
 savingDatasource,
 testingDatasource,
 connectionMessage,
 canSaveDatasource,
 statusLabel,
 statusTagType,
 checkConnection,
 saveConfig,
 closeModal
} = useMysqlDatasourceConfig()

const handleSave = async () => {
 if (!canSaveDatasource.value) {
 message.warning('请补全数据源配置')
 return
 }

 try {
 const connected = await saveConfig()
 message.success(connected ? '数据源配置已保存，连接正常' : '数据源配置已保存，但连接失败')
 } catch (error) {
 message.error(error?.message || '保存数据源配置失败')
 }
}

const handleTest = async () => {
 const connected = await checkConnection()

 if (connected) {
 message.success('MySQL 连接正常')
 } else {
 message.error(connectionMessage.value || '测试 MySQL 连接失败')
 }
}
</script>

<template>
 <n-modal
 v-model:show="modalVisible"
 preset="card"
 title="数据库配置"
 style="width: min(680px, calc(100vw - 32px))"
 >
 <div class="datasource-modal">
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
 <n-tag :type="statusTagType" size="small">
 {{ statusLabel }}
 </n-tag>
 </div>

 <div class="datasource-grid" :class="{ 'is-loading': loadingConfig }">
 <label class="field-block">
 <span>地址</span>
 <n-input
 v-model:value="datasourceForm.host"
 placeholder="127.0.0.1"
 clearable
 />
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
 <n-input
 v-model:value="datasourceForm.database"
 placeholder="database"
 clearable
 />
 </label>

 <label class="field-block">
 <span>账号</span>
 <n-input
 v-model:value="datasourceForm.username"
 placeholder="username"
 clearable
 />
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
 <n-button @click="closeModal">
 关闭
 </n-button>
 <n-button
 secondary
 :disabled="!hasSavedDatasource"
 :loading="testingDatasource"
 @click="handleTest"
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
 @click="handleSave"
 >
 <template #icon>
 <n-icon><SaveOutline /></n-icon>
 </template>
 保存并测试
 </n-button>
 </div>
 </div>
 </n-modal>
</template>

<style scoped>
.datasource-modal {
 display: grid;
 gap: 16px;
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

.datasource-grid {
 display: grid;
 grid-template-columns: minmax(0, 1fr) 132px;
 gap: 14px;
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

.modal-actions {
 display: flex;
 justify-content: flex-end;
 flex-wrap: wrap;
 gap: 8px;
}

@media (max-width: 560px) {
 .datasource-grid {
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
