<script setup>
import { onBeforeUnmount, onMounted } from 'vue'
import { useNotification } from 'naive-ui'
import { listen } from '@tauri-apps/api/event'

const notification = useNotification()
let unlisten = null

onMounted(async () => {
  try {
    unlisten = await listen('system-notify', (event) => {
      const payload = event.payload || {}
      const title = payload.title || '系统通知'
      const body = payload.body || ''
      const failed = payload.ok === false
      notification.create({
        title,
        content: body,
        type: failed ? 'warning' : 'success',
        duration: 10000,
        keepAliveOnHover: true
      })
    })
  } catch (error) {
    console.warn('listen system-notify failed', error)
  }
})

onBeforeUnmount(() => {
  if (typeof unlisten === 'function') {
    unlisten()
  }
})
</script>

<template></template>
