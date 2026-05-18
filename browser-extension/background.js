function cookieUrl(targetUrl, cookie) {
 const target = new URL(targetUrl)
 const path = cookie.path || '/'
 const normalizedPath = path.startsWith('/') ? path : `/${path}`

 return `${cookie.secure ? 'https' : target.protocol}//${target.host}${normalizedPath}`
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

async function writeCookiesAndOpen(payload, sender) {
 if (payload.type !== 'openWithCookies') {
 throw new Error('消息类型不支持')
 }

 const targetUrl = String(payload.targetUrl || '')
 const cookies = Array.isArray(payload.cookies) ? payload.cookies : []

 if (!targetUrl.startsWith('http://') && !targetUrl.startsWith('https://')) {
 throw new Error('目标地址不合法')
 }

 if (cookies.length === 0) {
 throw new Error('Cookie 列表为空')
 }

 const errors = []
 let written = 0
 let cleared = 0

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

 if (written === 0) {
 throw new Error(errors.length ? errors.join('; ') : '没有 Cookie 写入成功')
 }

 const tabId = sender?.tab?.id

 if (typeof tabId !== 'number') {
 throw new Error('无法定位当前浏览器标签页')
 }

 await chrome.tabs.update(tabId, { url: targetUrl })

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
