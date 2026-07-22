import { invokeApi } from './tauriClient'

export const loadHttpProxyConfig = () => {
  return invokeApi('load_http_proxy_config')
}

export const saveHttpProxyConfig = (config) => {
  return invokeApi('save_http_proxy_config', { config })
}
