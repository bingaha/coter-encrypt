import { computed, ref } from 'vue'
import { loadHttpProxyConfig, saveHttpProxyConfig } from '@/api/httpProxy'

const MODE_OPTIONS = [
  { value: 'direct', label: '直连', hint: '不使用任何代理' },
  { value: 'system', label: '系统代理', hint: '跟随环境变量 HTTP_PROXY / HTTPS_PROXY' },
  { value: 'custom', label: '指定代理', hint: '使用下方 HTTP/HTTPS 代理地址' }
]

const defaultForm = () => ({
  mode: 'system',
  url: ''
})

const modalVisible = ref(false)
const form = ref(defaultForm())
const loading = ref(false)
const saving = ref(false)
const modeLabel = ref('系统代理')

let loadedOnce = false
let loadingPromise = null

const isCustom = computed(() => form.value.mode === 'custom')

const canSave = computed(() => {
  if (form.value.mode !== 'custom') return true
  return Boolean(String(form.value.url || '').trim())
})

const statusLabel = computed(() => modeLabel.value || '系统代理')

const applyConfig = (config) => {
  form.value = {
    mode: config?.mode || 'system',
    url: config?.url || ''
  }
  const matched = MODE_OPTIONS.find((item) => item.value === form.value.mode)
  modeLabel.value = matched?.label || '系统代理'
}

const loadConfig = async ({ force = false } = {}) => {
  if (loadingPromise) return loadingPromise
  if (loadedOnce && !force) return true

  loading.value = true
  loadingPromise = loadHttpProxyConfig()
    .then((response) => {
      applyConfig(response.data || defaultForm())
      loadedOnce = true
      return true
    })
    .finally(() => {
      loading.value = false
      loadingPromise = null
    })

  return loadingPromise
}

const saveConfig = async () => {
  if (!canSave.value) {
    throw new Error('指定代理模式下请填写代理地址')
  }
  saving.value = true
  try {
    const payload = {
      mode: form.value.mode,
      url: String(form.value.url || '').trim()
    }
    const response = await saveHttpProxyConfig(payload)
    applyConfig(response.data || payload)
    loadedOnce = true
    return response.data
  } finally {
    saving.value = false
  }
}

const openModal = async () => {
  modalVisible.value = true
  await loadConfig({ force: true })
}

const closeModal = () => {
  modalVisible.value = false
}

export const useHttpProxyConfig = () => ({
  MODE_OPTIONS,
  modalVisible,
  form,
  loading,
  saving,
  isCustom,
  canSave,
  statusLabel,
  loadConfig,
  saveConfig,
  openModal,
  closeModal
})
