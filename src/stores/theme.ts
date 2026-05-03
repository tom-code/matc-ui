import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { darkTheme } from 'naive-ui'
import type { GlobalTheme, GlobalThemeOverrides } from 'naive-ui'

const compactOverrides: GlobalThemeOverrides = {
  common: {
    fontSize: '12px',
    fontSizeMini: '10px',
    fontSizeSmall: '11px',
    fontSizeMedium: '12px',
    fontSizeLarge: '13px',
    fontSizeHuge: '14px',
    heightTiny: '20px',
    heightSmall: '24px',
    heightMedium: '28px',
    heightLarge: '32px',
    heightHuge: '38px',
    borderRadius: '3px',
    borderRadiusSmall: '2px',
  },
}

export const useThemeStore = defineStore('theme', () => {
  const isDark = ref(localStorage.getItem('theme') === 'dark')
  const isCompact = ref(localStorage.getItem('compact') === 'true')

  const theme = computed<GlobalTheme | null>(() => isDark.value ? darkTheme : null)
  const themeOverrides = computed<GlobalThemeOverrides | undefined>(
    () => isCompact.value ? compactOverrides : undefined
  )

  function toggle() {
    isDark.value = !isDark.value
    localStorage.setItem('theme', isDark.value ? 'dark' : 'light')
  }

  function toggleCompact() {
    isCompact.value = !isCompact.value
    localStorage.setItem('compact', isCompact.value ? 'true' : 'false')
  }

  return { isDark, isCompact, theme, themeOverrides, toggle, toggleCompact }
})
