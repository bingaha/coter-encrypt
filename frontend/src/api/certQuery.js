import { invokeApi } from './tauriClient'

export const loadMysqlDatasourceConfig = () => {
 return invokeApi('load_mysql_datasource_config')
}

export const saveMysqlDatasourceConfig = (config) => {
 return invokeApi('save_mysql_datasource_config', { config })
}

export const testMysqlDatasource = (config = null) => {
 return invokeApi('test_mysql_datasource', { config })
}

export const queryCertInfo = (request) => {
 return invokeApi('query_cert_info', { request })
}

export const queryRobotTaskFeedbackData = (request) => {
 return invokeApi('query_robot_task_feedback_data', { request })
}

export const loadBrowserBridgeConfig = () => {
 return invokeApi('load_browser_bridge_config')
}

export const saveBrowserBridgeConfig = (config) => {
 return invokeApi('save_browser_bridge_config', { config })
}

export const loadWebsiteUrlMappings = () => {
 return invokeApi('load_website_url_mappings')
}

export const saveWebsiteUrlMapping = (mapping) => {
 return invokeApi('save_website_url_mapping', { mapping })
}

export const openDefaultBrowserWithCookies = (request) => {
 return invokeApi('open_default_browser_with_cookies', { request })
}
