<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { NAlert, NButton, NCard, NEmpty, NEllipsis, NPageHeader, NPopconfirm, NProgress, NSpace, NTag, NText, useMessage } from 'naive-ui'
import { autoScanRepositories, listRepositories, removeRepository, scanRepository } from '@/api/repositories'
import type { Repository, RepositoryScanProgress, RepositoryScanResult } from '@/api/tauri'

const repositories = ref<Repository[]>([])
const loading = ref(false)
const autoScanning = ref(false)
const error = ref('')
const scanProgress = ref<RepositoryScanProgress | null>(null)
const scanResults = ref<RepositoryScanResult[]>([])
const expandedScanResultKeys = ref<Set<string>>(new Set())
const message = useMessage()
let unlistenProgress: UnlistenFn | undefined

const progressPercentage = computed(() => {
  const progress = scanProgress.value
  if (!progress || !progress.total) return 0
  return Math.round((progress.current / progress.total) * 100)
})

async function loadRepositories() {
  try {
    repositories.value = await listRepositories()
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function addRepository() {
  error.value = ''
  const selected = await open({ directory: true, multiple: false, title: '选择漫画仓库' })
  if (!selected || Array.isArray(selected)) return

  loading.value = true
  try {
    const result = await scanRepository(selected)
    scanResults.value = [result, ...scanResults.value].slice(0, 8)
    await loadRepositories()
    message.success(scanSummaryMessage(result))
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

async function rescan(repository: Repository) {
  error.value = ''
  loading.value = true
  try {
    const result = await scanRepository(repository.path)
    scanResults.value = [result, ...scanResults.value].slice(0, 8)
    await loadRepositories()
    message.success(`已重新扫描「${repository.name}」：${scanSummaryMessage(result)}`)
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

async function remove(repository: Repository) {
  error.value = ''
  loading.value = true
  try {
    await removeRepository(repository.id)
    await loadRepositories()
    message.success('仓库记录已移除')
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

async function autoUpdateRepositories() {
  if (autoScanning.value) return
  error.value = ''
  autoScanning.value = true
  try {
    const results = await autoScanRepositories()
    scanResults.value = results
    await loadRepositories()
    if (results.length) {
      message.success(`已自动更新 ${results.length} 个仓库`)
    }
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    autoScanning.value = false
  }
}

function scanSummaryMessage(result: RepositoryScanResult) {
  const { scannedBooks, unchangedBooks, failedEntries } = result.summary
  return `更新 ${scannedBooks} 本，跳过 ${unchangedBooks} 本，失败 ${failedEntries.length} 项`
}

function scanResultKey(result: RepositoryScanResult) {
  return `${result.repository.path}-${result.repository.updatedAt}`
}

function isScanResultExpanded(result: RepositoryScanResult) {
  return expandedScanResultKeys.value.has(scanResultKey(result))
}

function toggleScanResultDetails(result: RepositoryScanResult) {
  const key = scanResultKey(result)
  const nextKeys = new Set(expandedScanResultKeys.value)
  if (nextKeys.has(key)) {
    nextKeys.delete(key)
  } else {
    nextKeys.add(key)
  }
  expandedScanResultKeys.value = nextKeys
}

function hasScanDetails(result: RepositoryScanResult) {
  return Boolean(
    result.summary.skippedEntries.length
      || result.summary.failedEntries.length
      || result.summary.duplicateBooks.length,
  )
}

function visibleScanEntries<T>(entries: T[], expanded: boolean) {
  return expanded ? entries : entries.slice(0, 3)
}

function formatDateTime(value?: string | null) {
  if (!value) return '未扫描'
  const date = new Date(value)
  if (!Number.isFinite(date.getTime())) return value
  return date.toLocaleString('zh-CN')
}

function fileName(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() || path
}

onMounted(async () => {
  unlistenProgress = await listen<RepositoryScanProgress>('repository-scan-progress', (event) => {
    scanProgress.value = event.payload
  })
  await loadRepositories()
})

onBeforeUnmount(() => {
  unlistenProgress?.()
})
</script>

<template>
  <section class="page-section">
    <NPageHeader>
      <template #title>漫画仓库</template>
      <template #subtitle>添加包含漫画目录、元数据和章节图片的本地仓库。</template>
      <template #extra>
        <NSpace>
          <NButton :loading="autoScanning" :disabled="loading || !repositories.length" @click="autoUpdateRepositories">
            更新全部
          </NButton>
          <NButton type="primary" :loading="loading" :disabled="autoScanning" @click="addRepository">
            添加仓库
          </NButton>
        </NSpace>
      </template>
    </NPageHeader>

    <NCard v-if="scanProgress && (autoScanning || loading)" class="toolbar-card" :bordered="false">
      <NSpace vertical size="small">
        <NSpace justify="space-between">
          <NText strong>{{ scanProgress.phase === 'finish' ? '扫描完成' : '正在扫描仓库' }}</NText>
          <NText depth="3">{{ scanProgress.current }} / {{ scanProgress.total }}</NText>
        </NSpace>
        <NProgress type="line" :percentage="progressPercentage" :show-indicator="false" />
        <NEllipsis class="path-text">{{ scanProgress.message }}</NEllipsis>
      </NSpace>
    </NCard>

    <NAlert v-if="error" type="error" class="state-block" :show-icon="false">
      {{ error }}
    </NAlert>

    <NSpace v-if="scanResults.length" vertical size="small" class="repository-list">
      <NCard v-for="result in scanResults" :key="scanResultKey(result)" :bordered="false" class="toolbar-card">
        <NSpace vertical size="small">
          <NSpace align="center" :wrap="true">
            <NText strong>{{ result.repository.name }}</NText>
            <NTag size="small" round>更新 {{ result.summary.scannedBooks }}</NTag>
            <NTag size="small" round>未变化 {{ result.summary.unchangedBooks }}</NTag>
            <NTag v-if="result.summary.failedEntries.length" size="small" type="error" round>
              失败 {{ result.summary.failedEntries.length }}
            </NTag>
            <NTag v-if="result.summary.duplicateBooks.length" size="small" type="warning" round>
              疑似重复 {{ result.summary.duplicateBooks.length }}
            </NTag>
            <NButton
              v-if="hasScanDetails(result)"
              size="tiny"
              secondary
              @click="toggleScanResultDetails(result)"
            >
              {{ isScanResultExpanded(result) ? '收起明细' : '查看明细' }}
            </NButton>
          </NSpace>
          <div v-if="result.summary.skippedEntries.length" class="scan-detail-block">
            <NText strong depth="3">跳过</NText>
            <NText
              v-for="item in visibleScanEntries(result.summary.skippedEntries, isScanResultExpanded(result))"
              :key="`${item.path}-${item.reason}`"
              depth="3"
            >
              {{ fileName(item.path) }}：{{ item.reason }}
            </NText>
          </div>
          <NAlert v-if="result.summary.failedEntries.length" type="warning" :show-icon="false">
            <NSpace vertical size="small">
              <NText strong>失败</NText>
              <NText
                v-for="item in visibleScanEntries(result.summary.failedEntries, isScanResultExpanded(result))"
                :key="`${item.path}-${item.reason}`"
              >
                {{ fileName(item.path) }}：{{ item.reason }}
              </NText>
            </NSpace>
          </NAlert>
          <NAlert v-if="result.summary.duplicateBooks.length" type="info" :show-icon="false">
            <NSpace vertical size="small">
              <NText strong>疑似重复</NText>
              <NText
                v-for="item in visibleScanEntries(result.summary.duplicateBooks, isScanResultExpanded(result))"
                :key="`${item.path}-${item.duplicateOf}`"
              >
                {{ item.title }}：{{ fileName(item.path) }} 与 {{ fileName(item.duplicateOf) }}
              </NText>
            </NSpace>
          </NAlert>
        </NSpace>
      </NCard>
    </NSpace>

    <NSpace v-if="repositories.length" vertical size="medium" class="repository-list">
      <NCard v-for="repository in repositories" :key="repository.id" embedded :bordered="false">
        <div class="repository-card-content">
          <div class="repository-main">
            <h2>{{ repository.name }}</h2>
            <NEllipsis class="path-text">{{ repository.path }}</NEllipsis>
          </div>
          <NSpace align="center" :wrap="true">
            <NTag round>{{ repository.bookCount }} 本漫画</NTag>
            <NText depth="3">最近扫描：{{ formatDateTime(repository.lastScannedAt) }}</NText>
            <NButton :disabled="loading || autoScanning" @click="rescan(repository)">重新扫描</NButton>
            <NPopconfirm @positive-click="remove(repository)">
              <template #trigger>
                <NButton type="error" secondary :disabled="loading || autoScanning">移除</NButton>
              </template>
              只移除 InkReader 中的仓库记录，不会删除本地漫画文件。
            </NPopconfirm>
          </NSpace>
        </div>
      </NCard>
    </NSpace>

    <NEmpty v-else class="state-block" description="还没有仓库">
      <template #extra>
        <NText depth="3">选择示例漫画结构所在目录，扫描后会在书架中显示漫画。</NText>
      </template>
    </NEmpty>
  </section>
</template>
