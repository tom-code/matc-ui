<script setup lang="ts">
import { useThemeStore } from '../stores/theme'
import type { ThemeName } from '../stores/theme'

const themeStore = useThemeStore()

const themeOptions: { label: string; value: ThemeName }[] = [
  { label: 'Light', value: 'light' },
  { label: 'Dark', value: 'dark' },
  { label: 'Orange', value: 'orange' },
]
</script>

<template>
  <div class="view-container">
    <n-h2>Settings</n-h2>
    <n-card title="Appearance" style="max-width: 400px">
      <n-form label-placement="left" label-width="120">
        <n-form-item label="Theme">
          <n-radio-group
            :value="themeStore.themeName"
            name="theme"
            @update:value="themeStore.setTheme"
          >
            <n-radio-button
              v-for="opt in themeOptions"
              :key="opt.value"
              :value="opt.value"
              :label="opt.label"
            />
          </n-radio-group>
        </n-form-item>
        <n-form-item label="Compact mode">
          <n-switch :value="themeStore.isCompact" @update:value="themeStore.toggleCompact" />
        </n-form-item>
      </n-form>
    </n-card>
  </div>
</template>
