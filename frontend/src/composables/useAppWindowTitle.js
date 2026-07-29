import { getVersion } from '@tauri-apps/api/app'
import { getCurrentWindow } from '@tauri-apps/api/window'

const TITLE_PREFIX = '加解密工具'

/**
 * 同步窗口标题为「加解密工具 vX.Y.Z」。
 * 同时更新 document.title 与原生窗口标题，保证桌面端显示一致。
 */
export async function syncAppWindowTitle() {
  let version = ''
  try {
    version = await getVersion()
  } catch {
    return
  }
  const title = version ? `${TITLE_PREFIX} v${version}` : TITLE_PREFIX
  document.title = title
  try {
    await getCurrentWindow().setTitle(title)
  } catch {
    // 非桌面预览或权限不足时忽略原生标题
  }
}
