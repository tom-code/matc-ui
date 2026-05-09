<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { RouterView, useRouter, useRoute } from 'vue-router'
import type { MenuOption } from 'naive-ui'
import { getVersion } from '@tauri-apps/api/app'
import { useThemeStore } from './stores/theme'

const version = ref('')
onMounted(async () => {
  try {
    version.value = await getVersion()
  } catch {
    // not running inside Tauri (e.g. plain vite dev server)
  }
})

const router = useRouter()
const route = useRoute()
const themeStore = useThemeStore()

const menuOptions: MenuOption[] = [
  { label: 'Devices', key: '/devices' },
  { label: 'Commission', key: '/commission' },
  { label: 'Logs', key: '/logs' },
  { label: 'Settings', key: '/settings' },
]

function handleMenuSelect(key: string) {
  router.push(key)
}

function activeKey() {
  if (route.path.startsWith('/devices')) return '/devices'
  return route.path
}
</script>

<template>
  <n-config-provider :theme="themeStore.theme" :theme-overrides="themeStore.themeOverrides">
    <n-message-provider>
      <n-layout has-sider style="height: 100vh">
        <n-layout-sider
          bordered
          :width="200"
          :collapsed-width="0"
          show-trigger="arrow-circle"
          content-style="padding: 16px 0"
        >
          <div class="logo">matc-ui</div>
          <div v-if="version" class="logo-version">v{{ version }}</div>
          <n-menu
            :options="menuOptions"
            :value="activeKey()"
            @update:value="handleMenuSelect"
          />
        </n-layout-sider>

        <n-layout content-style="padding: 0">
          <n-layout-header bordered style="padding: 0 24px; height: 48px; display: flex; align-items: center">
            <n-breadcrumb>
              <n-breadcrumb-item>Matter Controller</n-breadcrumb-item>
            </n-breadcrumb>
          </n-layout-header>

          <n-layout-content content-style="padding: 24px; overflow: auto; height: calc(100vh - 48px)">
            <RouterView />
          </n-layout-content>
        </n-layout>
      </n-layout>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body, #app {
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.logo {
  padding: 16px 16px 8px;
  font-size: 16px;
  font-weight: 700;
  color: var(--n-text-color);
  letter-spacing: -0.3px;
}

.view-container {
  max-width: 1200px;
}

.logo-version {
  padding: 0 16px 12px;
  font-size: 11px;
  color: var(--n-text-color-3);
  letter-spacing: 0.2px;
}
</style>
