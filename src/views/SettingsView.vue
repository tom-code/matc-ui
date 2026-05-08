<script setup lang="ts">
import { useThemeStore } from '../stores/theme'

const themeStore = useThemeStore()
</script>

<template>
  <div class="view-container">
    <n-h2>Settings</n-h2>
    <n-card title="Appearance" style="max-width: 400px">
      <n-form label-placement="left" label-width="120">
        <n-form-item label="Dark mode">
          <n-switch :value="themeStore.isDark" @update:value="themeStore.toggle" />
        </n-form-item>
        <n-form-item label="Compact mode">
          <n-switch :value="themeStore.isCompact" @update:value="themeStore.toggleCompact" />
        </n-form-item>
      </n-form>
    </n-card>
  </div>
</template>
