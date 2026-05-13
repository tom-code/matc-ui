import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { darkTheme } from 'naive-ui'
import type { GlobalTheme, GlobalThemeOverrides } from 'naive-ui'

export type ThemeName = 'light' | 'dark' | 'orange'

const compactCommon: GlobalThemeOverrides['common'] = {
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
}

const orangeCommon: GlobalThemeOverrides['common'] = {
  primaryColor: '#e07628',
  primaryColorHover: '#f08a3a',
  primaryColorPressed: '#c06018',
  primaryColorSuppl: '#f0a060',
}

function loadTheme(): ThemeName {
  const v = localStorage.getItem('theme')
  if (v === 'dark' || v === 'orange') return v
  return 'light'
}

export const useThemeStore = defineStore('theme', () => {
  const themeName = ref<ThemeName>(loadTheme())
  const isCompact = ref(localStorage.getItem('compact') === 'true')

  const isDark = computed(() => themeName.value === 'dark')

  const theme = computed<GlobalTheme | null>(() => themeName.value === 'dark' ? darkTheme : null)

  const themeOverrides = computed<GlobalThemeOverrides | undefined>(() => {
    const common: GlobalThemeOverrides['common'] = {
      ...(themeName.value === 'orange' ? orangeCommon : {}),
      ...(isCompact.value ? compactCommon : {}),
    }
    return Object.keys(common).length ? { common } : undefined
  })

  function setTheme(name: ThemeName) {
    themeName.value = name
    localStorage.setItem('theme', name)
  }

  function toggle() {
    setTheme(isDark.value ? 'light' : 'dark')
  }

  function toggleCompact() {
    isCompact.value = !isCompact.value
    localStorage.setItem('compact', isCompact.value ? 'true' : 'false')
  }

  return { themeName, isDark, isCompact, theme, themeOverrides, setTheme, toggle, toggleCompact }
})
