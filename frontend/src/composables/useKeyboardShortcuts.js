import { onMounted, onUnmounted } from 'vue'
import { useConfigStore, useExecutionStore } from '../store'

/**
 * 键盘快捷键 composable
 * 实现 Delete 键删除选中组件和 Ctrl+Enter 执行工作流
 * Requirements: 5.3, 6.6
 */
export function useKeyboardShortcuts(options = {}) {
 const {
 onExecute = null,
 enabled = true
 } = options

 const configStore = useConfigStore()
 const executionStore = useExecutionStore()

 /**
 * 检查当前焦点是否在输入元素上
 * 如果在输入元素上，不应该触发快捷键
 */
 const isInputFocused = () => {
 const activeElement = document.activeElement
 if (!activeElement) return false
 
 const tagName = activeElement.tagName.toLowerCase()
 const isInput = tagName === 'input' || tagName === 'textarea' || tagName === 'select'
 const isContentEditable = activeElement.getAttribute('contenteditable') === 'true'
 
 return isInput || isContentEditable
 }

 /**
 * 处理 Delete 键删除选中组件
 * Requirements: 5.3
 */
 const handleDeleteKey = () => {
 // 如果焦点在输入框中，不处理
 if (isInputFocused()) return false

 const selectedComponent = configStore.selectedComponent
 if (selectedComponent) {
 configStore.removeComponent(selectedComponent.id)
 return true
 }
 return false
 }

 /**
 * 处理 Ctrl+Enter 执行工作流
 * Requirements: 6.6
 */
 const handleCtrlEnter = () => {
 // 检查是否正在执行
 if (executionStore.isExecuting) return false

 // 检查是否有组件
 if (configStore.addedComponents.length === 0) return false

 // 检查是否有验证错误
 const validation = configStore.validateConfig()
 if (!validation.isValid) return false

 // 调用执行回调
 if (onExecute && typeof onExecute === 'function') {
 onExecute()
 return true
 }
 return false
 }

 /**
 * 键盘事件处理器
 */
 const handleKeydown = (event) => {
 if (!enabled) return

 // Ctrl+Enter 执行工作流
 if (event.ctrlKey && event.key === 'Enter') {
 event.preventDefault()
 handleCtrlEnter()
 return
 }

 // Delete 键删除选中组件
 if (event.key === 'Delete' || event.key === 'Backspace') {
 // Backspace 只在非输入状态下处理
 if (event.key === 'Backspace' && isInputFocused()) return
 
 if (event.key === 'Delete' && !isInputFocused()) {
 event.preventDefault()
 handleDeleteKey()
 }
 }
 }

 /**
 * 注册键盘事件监听
 */
 const registerShortcuts = () => {
 window.addEventListener('keydown', handleKeydown)
 }

 /**
 * 注销键盘事件监听
 */
 const unregisterShortcuts = () => {
 window.removeEventListener('keydown', handleKeydown)
 }

 // 组件挂载时自动注册
 onMounted(() => {
 registerShortcuts()
 })

 // 组件卸载时自动注销
 onUnmounted(() => {
 unregisterShortcuts()
 })

 return {
 handleDeleteKey,
 handleCtrlEnter,
 registerShortcuts,
 unregisterShortcuts,
 isInputFocused
 }
}
