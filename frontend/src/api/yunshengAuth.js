import { invokeApi } from './tauriClient'

export const loadYunshengAuthToken = () => invokeApi('load_yunsheng_auth_token')

export const saveYunshengAuthToken = (config) =>
  invokeApi('save_yunsheng_auth_token', { config })
