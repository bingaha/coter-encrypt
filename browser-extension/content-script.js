function setStatus(message) {
 const statusEl = document.getElementById('status')

 if (statusEl) {
 statusEl.textContent = message
 }
}

function getBridgeParams() {
 const url = new URL(window.location.href)

 if (url.hostname !== '127.0.0.1' || url.pathname !== '/bridge') {
 return null
 }

 const token = url.searchParams.get('token') || ''
 const extensionId = url.searchParams.get('extensionId') || ''
 const port = Number(url.port)

 if (!token) {
 throw new Error('缺少连接令牌')
 }

 if (!Number.isInteger(port) || port <= 0 || port > 65535) {
 throw new Error('本地端口不合法')
 }

 return {
 expectedExtensionId: extensionId,
 port,
 token
 }
}

function sendRuntimeMessage(payload) {
 return new Promise((resolve, reject) => {
 chrome.runtime.sendMessage(payload, (response) => {
 const runtimeError = chrome.runtime.lastError

 if (runtimeError) {
 reject(new Error(runtimeError.message))
 return
 }

 if (!response) {
 reject(new Error('浏览器插件后台未返回执行结果'))
 return
 }

 resolve(response)
 })
 })
}

function sendSocketResult(socket, result) {
 if (socket.readyState === WebSocket.OPEN) {
 socket.send(JSON.stringify(result))
 }
}

async function run() {
 try {
 const bridgeParams = getBridgeParams()

 if (!bridgeParams) {
 return
 }

 const { expectedExtensionId, port, token } = bridgeParams
 const socket = new WebSocket(`ws://127.0.0.1:${port}/ws?token=${encodeURIComponent(token)}`)
 let finished = false

 socket.addEventListener('open', () => {
 setStatus('已连接桌面程序，正在接收 Cookie...')
 })

 socket.addEventListener('message', async (event) => {
 try {
 const payload = JSON.parse(event.data)
 setStatus('正在写入浏览器 Cookie...')

 if (expectedExtensionId && expectedExtensionId !== chrome.runtime.id) {
 const result = {
 type: 'openWithCookiesResult',
 requestId: payload.requestId,
 ok: false,
 written: 0,
 errors: ['当前浏览器插件 ID 与桌面应用配置不一致']
 }
 finished = true
 sendSocketResult(socket, result)
 setStatus(result.errors[0])
 return
 }

 const result = await sendRuntimeMessage(payload)
 finished = true
 sendSocketResult(socket, result)
 } catch (error) {
 const result = {
 type: 'openWithCookiesResult',
 ok: false,
 written: 0,
 errors: [error?.message || String(error)]
 }
 finished = true
 sendSocketResult(socket, result)
 setStatus(result.errors[0])
 }
 })

 socket.addEventListener('error', () => {
 if (!finished) {
 setStatus('连接桌面程序失败')
 }
 })

 socket.addEventListener('close', () => {
 if (!finished) {
 setStatus('连接已关闭')
 }
 })
 } catch (error) {
 setStatus(error?.message || String(error))
 }
}

run()
