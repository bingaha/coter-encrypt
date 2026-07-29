import { ref } from 'vue'

/** @typedef {'database' | 'proxy' | 'browser' | 'yunsheng'} SettingsTab */

const settingsVisible = ref(false)
/** @type {import('vue').Ref<SettingsTab>} */
const settingsTab = ref('database')

/**
 * 打开统一设置弹窗并定位到指定 Tab。
 * @param {SettingsTab} [tab='database']
 */
const openSettings = (tab = 'database') => {
  settingsTab.value = tab
  settingsVisible.value = true
}

const closeSettings = () => {
  settingsVisible.value = false
}

export const useAppSettings = () => ({
  settingsVisible,
  settingsTab,
  openSettings,
  closeSettings
})
