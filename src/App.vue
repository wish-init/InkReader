<script setup lang="ts">
import { NConfigProvider, NDialogProvider, NLoadingBarProvider, NMessageProvider, type GlobalThemeOverrides } from 'naive-ui'
import AppShell from '@/components/common/AppShell.vue'

const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#2f6f4e',
    primaryColorHover: '#3f855f',
    primaryColorPressed: '#245c3f',
    primaryColorSuppl: '#3f855f',
    borderRadius: '10px',
    borderColor: '#dfe6dc',
    bodyColor: '#f5f7f2',
    cardColor: '#ffffff',
    textColorBase: '#1f2a22',
    textColor1: '#1f2a22',
    textColor2: '#526057',
    textColor3: '#748077',
    fontFamily: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  },
  Card: {
    borderRadius: '14px',
    paddingMedium: '18px',
  },
  Button: {
    borderRadiusMedium: '10px',
  },
  Menu: {
    itemBorderRadius: '10px',
  },
}
</script>

<template>
  <NConfigProvider :theme-overrides="themeOverrides">
    <NDialogProvider>
      <NMessageProvider>
        <NLoadingBarProvider>
          <AppShell />
        </NLoadingBarProvider>
      </NMessageProvider>
    </NDialogProvider>
  </NConfigProvider>
</template>
