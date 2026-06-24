<script setup>
import { ref, computed, provide } from 'vue'
import {
 NConfigProvider,
 NMessageProvider,
 NDialogProvider,
 NNotificationProvider,
 zhCN,
 dateZhCN,
 darkTheme
} from 'naive-ui'
import MysqlDatasourceModal from '@/components/MysqlDatasourceModal.vue'

// 主题状态管理，从 localStorage 读取初始值
const isDarkMode = ref(localStorage.getItem('theme') === 'dark')

// 计算当前主题
const theme = computed(() => isDarkMode.value ? darkTheme : null)

// 主题切换方法
const toggleTheme = () => {
 isDarkMode.value = !isDarkMode.value
 localStorage.setItem('theme', isDarkMode.value ? 'dark' : 'light')
}

// 提供给子组件使用
provide('isDarkMode', isDarkMode)
provide('toggleTheme', toggleTheme)
</script>

<template>
 <n-config-provider :theme="theme" :locale="zhCN" :date-locale="dateZhCN">
 <n-message-provider>
 <n-dialog-provider>
 <n-notification-provider>
 <div class="app-container">
 <router-view v-slot="{ Component }">
 <keep-alive>
 <component :is="Component" />
 </keep-alive>
 </router-view>
 <mysql-datasource-modal />
 </div>
 </n-notification-provider>
 </n-dialog-provider>
 </n-message-provider>
 </n-config-provider>
</template>

<style>
.app-container {
 position: fixed;
 inset: 0;
 width: 100%;
 height: 100%;
 min-height: 0;
 display: flex;
 flex-direction: column;
 overflow: hidden;
}

.app-container > * {
 flex: 1 1 auto;
 min-height: 0;
}
</style>
