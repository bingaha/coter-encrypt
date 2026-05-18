import { invoke } from '@tauri-apps/api/core'

const toMessage = (error) => {
 if (!error) return '未知错误'
 if (typeof error === 'string') return error
 if (error.message) return error.message

 try {
 return JSON.stringify(error)
 } catch {
 return String(error)
 }
}

const normalizeTauriError = (error) => {
 if (error instanceof Error) return error

 const normalized = new Error(toMessage(error))
 const status = error?.status ?? error?.response?.status
 const data = error?.data ?? error?.response?.data ?? error

 if (status) {
 normalized.response = { status, data }
 }

 return normalized
}

export const invokeApi = async (command, args) => {
 try {
 const data = args === undefined
 ? await invoke(command)
 : await invoke(command, args)

 return { data }
 } catch (error) {
 throw normalizeTauriError(error)
 }
}
