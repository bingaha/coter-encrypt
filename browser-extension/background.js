function cookieUrl(targetUrl, cookie) {
 const target = new URL(targetUrl)
 const path = cookie.path || '/'
 const normalizedPath = path.startsWith('/') ? path : `/${path}`

 return `${cookie.secure ? 'https:' : target.protocol}//${target.host}${normalizedPath}`
}

function normalizeHostname(hostname) {
 return String(hostname || '').trim().toLowerCase().replace(/^\./, '')
}

function isIpAddress(hostname) {
 return /^(\d{1,3}\.){3}\d{1,3}$/.test(hostname) || hostname.includes(':')
}

function domainMatchesHostname(cookieDomain, targetHostname) {
 const domain = normalizeHostname(cookieDomain)
 const hostname = normalizeHostname(targetHostname)

 if (!domain || !hostname) {
 return false
 }

 return domain === hostname || hostname.endsWith(`.${domain}`)
}

function hostnameMatchesCookieDomain(hostname, cookieDomain) {
 const normalizedHostname = normalizeHostname(hostname)
 const domain = normalizeHostname(cookieDomain)

 if (!normalizedHostname || !domain) {
 return false
 }

 return normalizedHostname === domain || normalizedHostname.endsWith(`.${domain}`)
}

function candidateCookieUrls(targetUrl) {
 const target = new URL(targetUrl)
 const base = `${target.hostname}${target.port ? `:${target.port}` : ''}`

 return [
 `https://${base}/`,
 `http://${base}/`
 ]
}

function removeCookie(details) {
 return new Promise((resolve, reject) => {
 chrome.cookies.remove(details, (removed) => {
 const runtimeError = chrome.runtime.lastError

 if (runtimeError) {
 reject(new Error(runtimeError.message))
 return
 }

 resolve(removed)
 })
 })
}

async function clearCookiesForTarget(targetUrl) {
 const target = new URL(targetUrl)
 const hostname = normalizeHostname(target.hostname)
 const relatedCookies = new Map()

 for (const url of candidateCookieUrls(targetUrl)) {
 const cookies = await chrome.cookies.getAll({ url })

 for (const cookie of cookies) {
 if (!domainMatchesHostname(cookie.domain, hostname)) {
 continue
 }

 const key = [
 cookie.storeId || '',
 cookie.name,
 cookie.domain,
 cookie.path,
 cookie.secure ? 'secure' : 'plain'
 ].join('\n')
 relatedCookies.set(key, cookie)
 }
 }

 const targetParts = hostname.split('.').filter(Boolean)
 const domainFilters = [hostname]

 if (!isIpAddress(hostname) && targetParts.length > 1) {
 for (let index = 1; index < targetParts.length - 1; index += 1) {
 domainFilters.push(targetParts.slice(index).join('.'))
 }
 }

 for (const domain of domainFilters) {
 const cookies = await chrome.cookies.getAll({ domain })

 for (const cookie of cookies) {
 if (!hostnameMatchesCookieDomain(hostname, cookie.domain)) {
 continue
 }

 const key = [
 cookie.storeId || '',
 cookie.name,
 cookie.domain,
 cookie.path,
 cookie.secure ? 'secure' : 'plain'
 ].join('\n')
 relatedCookies.set(key, cookie)
 }
 }

 const errors = []
 let cleared = 0

 for (const cookie of relatedCookies.values()) {
 try {
 const protocol = cookie.secure ? 'https:' : target.protocol
 const path = cookie.path || '/'
 const normalizedPath = path.startsWith('/') ? path : `/${path}`
 const url = `${protocol}//${target.host}${normalizedPath}`
 const removed = await removeCookie({
 url,
 name: cookie.name,
 storeId: cookie.storeId
 })

 if (removed) {
 cleared += 1
 }
 } catch (error) {
 errors.push(`${cookie.name}: ${error?.message || String(error)}`)
 }
 }

 return { cleared, errors }
}

async function setCookie(targetUrl, cookie) {
 if (!cookie || !cookie.name || cookie.value === undefined) {
 throw new Error('Cookie 缺少 name 或 value')
 }

 const details = {
 url: cookieUrl(targetUrl, cookie),
 name: String(cookie.name),
 value: String(cookie.value),
 path: cookie.path || '/',
 secure: Boolean(cookie.secure),
 httpOnly: Boolean(cookie.httpOnly)
 }

 if (cookie.domain) {
 details.domain = String(cookie.domain)
 }

 if (cookie.expirationDate !== undefined && cookie.expirationDate !== null) {
 details.expirationDate = Number(cookie.expirationDate)
 }

 if (cookie.sameSite) {
 details.sameSite = cookie.sameSite
 }

 return chrome.cookies.set(details)
}

function waitForTabLoad(tabId) {
 return new Promise((resolve, reject) => {
 const timeout = setTimeout(() => {
 chrome.tabs.onUpdated.removeListener(listener)
 reject(new Error('等待页面加载超时'))
 }, 30000)

 function listener(updatedTabId, info) {
 if (updatedTabId === tabId && info.status === 'complete') {
 chrome.tabs.onUpdated.removeListener(listener)
 clearTimeout(timeout)
 resolve()
 }
 }

 chrome.tabs.onUpdated.addListener(listener)
 })
}

async function injectStorageItems(tabId, storageItems) {
 if (!storageItems || storageItems.length === 0) {
 return false
 }

 await chrome.scripting.executeScript({
 target: { tabId },
 func: (items) => {
 for (const item of items) {
 if (item.storage === 'sessionStorage') {
 sessionStorage.setItem(item.key, item.value)
 } else if (item.storage === 'localStorage') {
 localStorage.setItem(item.key, item.value)
 }
 }
 },
 args: [storageItems]
 })

 return true
}

async function writeCookiesAndOpen(payload, sender) {
 if (payload.type !== 'openWithCookies') {
 throw new Error('消息类型不支持')
 }

 const targetUrl = String(payload.targetUrl || '')
 const cookies = Array.isArray(payload.cookies) ? payload.cookies : []
 const storageItems = Array.isArray(payload.storageItems) ? payload.storageItems : []

 if (!targetUrl.startsWith('http://') && !targetUrl.startsWith('https://')) {
 throw new Error('目标地址不合法')
 }

 if (cookies.length === 0 && storageItems.length === 0) {
 throw new Error('Cookie 列表和存储规则均为空')
 }

 const errors = []
 let written = 0
 let cleared = 0

 // 1. 清除并写入 Cookie
 try {
 const clearResult = await clearCookiesForTarget(targetUrl)
 cleared = clearResult.cleared
 errors.push(...clearResult.errors.map((message) => `清理 Cookie 失败: ${message}`))
 } catch (error) {
 errors.push(`清理 Cookie 失败: ${error?.message || String(error)}`)
 }

 for (const cookie of cookies) {
 try {
 await setCookie(targetUrl, cookie)
 written += 1
 } catch (error) {
 errors.push(`${cookie?.name || 'unknown'}: ${error?.message || String(error)}`)
 }
 }

 if (cookies.length > 0 && written === 0) {
 throw new Error(errors.length ? errors.join('; ') : '没有 Cookie 写入成功')
 }

 // 2. 创建新标签页打开目标页面
 const tab = await chrome.tabs.create({ url: targetUrl, active: true })

 // 3. 等待页面加载完成
 await waitForTabLoad(tab.id)

 // 4. 注入 sessionStorage/localStorage
 let storageInjected = false
 if (storageItems.length > 0) {
 try {
 storageInjected = await injectStorageItems(tab.id, storageItems)
 } catch (error) {
 errors.push(`写入存储失败: ${error?.message || String(error)}`)
 }
 }

 // 5. 如果注入了存储，刷新页面让页面读取预设值
 if (storageInjected) {
 await chrome.tabs.reload(tab.id)
 }

 return {
 type: 'openWithCookiesResult',
 requestId: payload.requestId,
 ok: errors.length === 0,
 written,
 cleared,
 errors
 }
}

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
 if (message?.type !== 'openWithCookies') {
 return false
 }

 writeCookiesAndOpen(message, sender)
 .then((result) => sendResponse(result))
 .catch((error) => {
 sendResponse({
 type: 'openWithCookiesResult',
 requestId: message.requestId,
 ok: false,
 written: 0,
 cleared: 0,
 errors: [error?.message || String(error)]
 })
 })

 return true
})
