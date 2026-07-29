import { invokeApi } from './tauriClient'

/** 云生 shebaorobot 固定桥接目标（与 Rust 侧 SHEBAOROBOT_URL 一致） */
export const YUNSHENG_SHEBAOROBOT_URL = 'https://work.yunsheng.cn/shebaorobot/'

export const loadYunshengAuthToken = () => invokeApi('load_yunsheng_auth_token')

export const saveYunshengAuthToken = (config) =>
  invokeApi('save_yunsheng_auth_token', { config })

/** SM2 全自动登录：写回 Cookie，并按勾选同步文件 / 打开浏览器 */
export const loginYunsheng = () => invokeApi('login_yunsheng')

/** 用 Cookie 请求头桥接打开指定 URL（需已配置浏览器插件 ID） */
export const openBrowserWithUrlCookieHeader = (request) =>
  invokeApi('open_browser_with_url_cookie_header', { request })

/** 用当前 Cookie 打开 shebaorobot */
export const openYunshengBrowserWithCookies = (cookieHeader) =>
  openBrowserWithUrlCookieHeader({
    targetUrl: YUNSHENG_SHEBAOROBOT_URL,
    cookieHeader: String(cookieHeader || '').trim()
  })
