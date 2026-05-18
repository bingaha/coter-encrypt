import { invokeApi } from './tauriClient'

/**
 * 使用指定项目配置处理本地 HAR 文件。
 * 入参必须是本地路径，不再走浏览器上传或 HTTP multipart。
 */
export const processHarWithProject = (options) => {
 return invokeApi('process_har', {
 request: {
 inputPath: options.inputPath,
 outputPath: options.outputPath,
 projectId: options.projectId ? Number(options.projectId) : null,
 projectName: options.projectName || null,
 inputOriginalRef: options.inputOriginalRef || '',
 finalOutputMappingId: options.finalOutputMappingId || '',
 regexPreset: options.regexPreset || 'BASE64',
 inputValues: options.inputValues || {}
 }
 })
}
