import { getVersion } from '@tauri-apps/api/app'
import { getCurrentWindow } from '@tauri-apps/api/window'

const TITLE_PREFIX = '加解密工具'

const buildVersion =
  typeof __APP_VERSION__ === 'string' && __APP_VERSION__ ? __APP_VERSION__ : ''

function titleWithVersion(version) {
  const v = String(version || '').trim()
  return v ? `${TITLE_PREFIX} v${v}` : TITLE_PREFIX
}

/** 用构建期版本立刻写 document.title（不依赖异步 IPC）。 */
export function applyBuildWindowTitle() {
  if (!buildVersion) return
  document.title = titleWithVersion(buildVersion)
}

/**
 * 同步窗口标题为「加解密工具 vX.Y.Z」。
 * 同时更新 document.title 与原生窗口标题。
 */
export async function syncAppWindowTitle() {
  applyBuildWindowTitle()
  let version = buildVersion
  try {
    version = (await getVersion()) || buildVersion
  } catch {
    // 保留构建期版本
  }
  const title = titleWithVersion(version)
  document.title = title
  try {
    await getCurrentWindow().setTitle(title)
  } catch {
    // 忽略原生标题失败；document.title 仍会生效
  }
}
