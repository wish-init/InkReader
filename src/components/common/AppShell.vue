<script setup lang="ts">
import { computed, h, onMounted, ref, watch } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import { NButton, NLayout, NLayoutContent, NLayoutSider, NMenu, NText, type MenuOption } from 'naive-ui'
import { autoScanRepositories, listRepositories } from '@/api/repositories'

const route = useRoute()
const router = useRouter()
const siderCollapsedKey = 'inkreader:sider-collapsed'

const navItems = [
  { key: 'library', label: '书架', path: '/library', icon: '书' },
  { key: 'favorites', label: '收藏', path: '/favorites', icon: '藏' },
  { key: 'history', label: '阅读记录', path: '/history', icon: '史' },
  { key: 'repositories', label: '仓库', path: '/repositories', icon: '仓' },
  { key: 'settings', label: '设置', path: '/settings', icon: '设' },
]
const siderCollapsed = ref(loadSiderCollapsed())

const menuOptions: MenuOption[] = navItems.map((item) => ({
  key: item.key,
  label: () => h('span', item.label),
  icon: () => h('span', { class: 'nav-icon' }, item.icon),
}))

const selectedKey = computed(() => {
  const path = route.path === '/' ? '/library' : route.path
  if (path.startsWith('/reader')) return 'library'
  return navItems.find((item) => path.startsWith(item.path))?.key ?? 'library'
})

function handleMenuUpdate(key: string) {
  const item = navItems.find((value) => value.key === key)
  if (item && route.path !== item.path) router.push(item.path)
}

function loadSiderCollapsed() {
  return window.localStorage.getItem(siderCollapsedKey) === 'true'
}

function toggleSider() {
  siderCollapsed.value = !siderCollapsed.value
}

async function autoScanOnStartup() {
  if (window.sessionStorage.getItem('inkreader:auto-scan-started') === 'true') return
  window.sessionStorage.setItem('inkreader:auto-scan-started', 'true')
  try {
    const repositories = await listRepositories()
    if (!repositories.length) return
    await autoScanRepositories()
    window.dispatchEvent(new CustomEvent('inkreader:auto-scan-complete'))
  } catch {
    window.dispatchEvent(new CustomEvent('inkreader:auto-scan-failed'))
  }
}

watch(siderCollapsed, (value) => {
  window.localStorage.setItem(siderCollapsedKey, String(value))
})

onMounted(() => {
  void autoScanOnStartup()
})
</script>

<template>
  <NLayout has-sider class="app-shell">
    <NLayoutSider
      bordered
      :width="236"
      :collapsed-width="72"
      collapse-mode="width"
      :collapsed="siderCollapsed"
      :native-scrollbar="false"
      class="app-sider"
      :class="{ 'app-sider-collapsed': siderCollapsed }"
    >
      <div class="brand">
        <div class="brand-mark">IR</div>
        <div v-if="!siderCollapsed" class="brand-copy">
          <div class="brand-title">InkReader</div>
          <NText depth="3" class="brand-subtitle">本地漫画仓库</NText>
        </div>
        <NButton
          quaternary
          circle
          size="small"
          class="sider-toggle"
          :title="siderCollapsed ? '展开侧边栏' : '收起侧边栏'"
          :aria-label="siderCollapsed ? '展开侧边栏' : '收起侧边栏'"
          @click="toggleSider"
        >
          {{ siderCollapsed ? '>' : '<' }}
        </NButton>
      </div>

      <NMenu
        :value="selectedKey"
        :options="menuOptions"
        :collapsed="siderCollapsed"
        :collapsed-width="72"
        :indent="16"
        @update:value="handleMenuUpdate"
      />
    </NLayoutSider>

    <NLayoutContent class="main-panel">
      <RouterView />
    </NLayoutContent>
  </NLayout>
</template>
