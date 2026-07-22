<script setup>
import {
  NButton,
  NIcon,
  NInput,
  NModal,
  NRadio,
  NRadioGroup,
  NSpace,
  NText,
  useMessage
} from 'naive-ui'
import { SaveOutline } from '@vicons/ionicons5'
import { useHttpProxyConfig } from '@/composables/useHttpProxyConfig'

const message = useMessage()

const {
  MODE_OPTIONS,
  modalVisible,
  form,
  loading,
  saving,
  isCustom,
  canSave,
  closeModal,
  saveConfig
} = useHttpProxyConfig()

const handleSave = async () => {
  if (!canSave.value) {
    message.warning('指定代理模式下请填写代理地址')
    return
  }
  try {
    await saveConfig()
    message.success('代理配置已保存并生效')
    closeModal()
  } catch (error) {
    message.error(error?.message || '保存代理配置失败')
  }
}
</script>

<template>
  <n-modal
    v-model:show="modalVisible"
    preset="card"
    title="网络代理"
    style="width: min(560px, calc(100vw - 32px))"
  >
    <div class="proxy-modal" :class="{ 'is-loading': loading }">
      <n-text depth="3" class="hint">
        作用于云效流水线监控、OSS 互转等出站 HTTP 请求；保存后立即生效，无需重启。
      </n-text>

      <div class="field-block">
        <span>代理模式</span>
        <n-radio-group v-model:value="form.mode" name="proxy-mode">
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
          v-model:value="form.url"
          :disabled="!isCustom"
          placeholder="http://127.0.0.1:7890"
          clearable
        />
        <n-text depth="3" class="field-hint">
          仅支持 HTTP/HTTPS 代理；如需账号可写在地址中：http://user:pass@host:port
        </n-text>
      </label>

      <div class="modal-actions">
        <n-button @click="closeModal">关闭</n-button>
        <n-button
          type="primary"
          :disabled="!canSave"
          :loading="saving"
          @click="handleSave"
        >
          <template #icon>
            <n-icon><SaveOutline /></n-icon>
          </template>
          保存并生效
        </n-button>
      </div>
    </div>
  </n-modal>
</template>

<style scoped>
.proxy-modal {
  display: grid;
  gap: 16px;
}

.proxy-modal.is-loading {
  opacity: 0.7;
  pointer-events: none;
}

.hint {
  display: block;
  line-height: 1.5;
}

.field-block {
  display: grid;
  gap: 8px;
  font-size: 13px;
}

.mode-item {
  display: grid;
  gap: 2px;
}

.mode-item strong {
  font-weight: 600;
}

.field-hint {
  font-size: 12px;
  line-height: 1.4;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}
</style>
