<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { NAlert, NButton, NCard, NEmpty, NEllipsis, NPageHeader, NPopconfirm, NProgress, NSpace, NTag, NText, useMessage } from 'naive-ui'
import { autoScanRepositories, listRepositories, listRepositoryScanHistory, removeRepository, scanRepository } from '@/api/repositories'
import type {
  Repository,
  RepositoryDuplicateBook,
  RepositoryScanHistoryRecord,
  RepositoryScanIssue,
  RepositoryScanIssueCode,
  RepositoryScanIssueSeverity,
  RepositoryScanProgress,
  RepositoryScanResult,
} from '@/api/tauri'

type ScanDiagnostic = {
  path: string
  reason: string
  code: RepositoryScanIssueCode
  severity: RepositoryScanIssueSeverity
  suggestion?: string
  duplicateOf?: string
  title?: string
}

type DisplayScanResult = {
  summary: RepositoryScanResult['summary']
}

type ScanDiagnosticGroup = {
  code: RepositoryScanIssueCode
  label: string
  severity: RepositoryScanIssueSeverity
  items: ScanDiagnostic[]
}

const scanIssueCodeLabels: Record<RepositoryScanIssueCode, string> = {
  unchangedBook: '未变化',
  noImages: '无可阅读图片',
  readFailed: '读取失败',
  duplicateBook: '疑似重复',
  unknown: '其他问题',
}

const scanSeverityAlertTypes: Record<RepositoryScanIssueSeverity, 'default' | 'info' | 'success' | 'warning' | 'error'> = {
  info: 'info',
  warning: 'warning',
  error: 'error',
}

const repositories = ref<Repository[]>([])
const loading = ref(false)
const autoScanning = ref(false)
const error = ref('')
const scanProgress = ref<RepositoryScanProgress | null>(null)
const scanResults = ref<RepositoryScanResult[]>([])
const scanHistory = ref<RepositoryScanHistoryRecord[]>([])
const expandedScanResultKeys = ref<Set<string>>(new Set())
const message = useMessage()
let unlistenProgress: UnlistenFn | undefined

const progressPercentage = computed(() => {
  const progress = scanProgress.value
  if (!progress || !progress.total) return 0
  return Math.round((progress.current / progress.total) * 100)
})

const scanProgressTitle = computed(() => {
  switch (scanProgress.value?.phase) {
    case 'scanComplete':
      return '扫描完成，准备保存'
    case 'persist':
      return '正在保存扫描结果'
    case 'finish':
      return '扫描完成'
    default:
      return '正在扫描仓库'
  }
})

async function loadRepositories() {
  try {
    repositories.value = await listRepositories()
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function loadScanHistory() {
  try {
    scanHistory.value = await listRepositoryScanHistory()
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
    await loadScanHistory()
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
    await loadScanHistory()
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
    await loadScanHistory()
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

function hasScanDetails(result: DisplayScanResult) {
  return scanDiagnostics(result).length > 0
}

function visibleScanEntries<T>(entries: T[], expanded: boolean) {
  return expanded ? entries : entries.slice(0, 3)
}

function scanDiagnosticGroups(result: DisplayScanResult): ScanDiagnosticGroup[] {
  const groups = new Map<RepositoryScanIssueCode, ScanDiagnosticGroup>()

  for (const diagnostic of scanDiagnostics(result)) {
    const existing = groups.get(diagnostic.code)
    if (existing) {
      existing.items.push(diagnostic)
      continue
    }

    groups.set(diagnostic.code, {
      code: diagnostic.code,
      label: scanIssueCodeLabels[diagnostic.code],
      severity: diagnostic.severity,
      items: [diagnostic],
    })
  }

  return Array.from(groups.values())
}

function scanDiagnostics(result: DisplayScanResult): ScanDiagnostic[] {
  return [
    ...result.summary.skippedEntries.map(normalizeScanIssue),
    ...result.summary.failedEntries.map(normalizeScanIssue),
    ...result.summary.duplicateBooks.map(duplicateBookDiagnostic),
  ]
}

function normalizeScanIssue(issue: RepositoryScanIssue): ScanDiagnostic {
  return {
    path: issue.path,
    reason: issue.reason,
    code: normalizeScanIssueCode(issue.code),
    severity: normalizeScanIssueSeverity(issue.severity, issue.code),
    suggestion: issue.suggestion,
  }
}

function duplicateBookDiagnostic(book: RepositoryDuplicateBook): ScanDiagnostic {
  return {
    path: book.path,
    reason: `${fileName(book.path)} 与 ${fileName(book.duplicateOf)} 疑似重复`,
    code: 'duplicateBook',
    severity: 'warning',
    suggestion: '检查两本书是否为同一内容后再整理。',
    duplicateOf: book.duplicateOf,
    title: book.title,
  }
}

function normalizeScanIssueCode(code?: RepositoryScanIssueCode): RepositoryScanIssueCode {
  return code && code in scanIssueCodeLabels ? code : 'unknown'
}

function normalizeScanIssueSeverity(
  severity?: RepositoryScanIssueSeverity,
  code?: RepositoryScanIssueCode,
): RepositoryScanIssueSeverity {
  if (severity === 'info' || severity === 'warning' || severity === 'error') return severity
  return code === 'unchangedBook' ? 'info' : 'warning'
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
  await Promise.all([loadRepositories(), loadScanHistory()])
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
          <NText strong>{{ scanProgressTitle }}</NText>
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
          <NAlert
            v-for="group in scanDiagnosticGroups(result)"
            :key="group.code"
            :type="scanSeverityAlertTypes[group.severity]"
            :show-icon="false"
          >
            <NSpace vertical size="small">
              <NSpace align="center" size="small">
                <NText strong>{{ group.label }}</NText>
                <NTag size="small" round>{{ group.items.length }}</NTag>
              </NSpace>
              <NText
                v-for="item in visibleScanEntries(group.items, isScanResultExpanded(result))"
                :key="`${item.code}-${item.path}-${item.reason}`"
              >
                <template v-if="item.code === 'duplicateBook' && item.duplicateOf">
                  {{ item.title || fileName(item.path) }}：{{ fileName(item.path) }} 与 {{ fileName(item.duplicateOf) }}
                </template>
                <template v-else>
                  {{ fileName(item.path) }}：{{ item.reason }}
                </template>
                <NText v-if="item.suggestion" depth="3">（{{ item.suggestion }}）</NText>
              </NText>
            </NSpace>
          </NAlert>
        </NSpace>
      </NCard>
    </NSpace>

    <NSpace v-if="scanHistory.length" vertical size="small" class="repository-list">
      <NText strong>最近扫描历史</NText>
      <NCard v-for="record in scanHistory" :key="record.id" :bordered="false" class="toolbar-card">
        <NSpace vertical size="small">
          <NSpace align="center" :wrap="true">
            <NText strong>{{ record.repositoryName }}</NText>
            <NText depth="3">{{ formatDateTime(record.scannedAt) }}</NText>
            <NTag size="small" round>更新 {{ record.summary.scannedBooks }}</NTag>
            <NTag size="small" round>未变化 {{ record.summary.unchangedBooks }}</NTag>
            <NTag v-if="record.summary.failedEntries.length" size="small" type="error" round>
              失败 {{ record.summary.failedEntries.length }}
            </NTag>
            <NTag v-if="record.summary.duplicateBooks.length" size="small" type="warning" round>
              疑似重复 {{ record.summary.duplicateBooks.length }}
            </NTag>
          </NSpace>
          <NEllipsis class="path-text">{{ record.repositoryPath }}</NEllipsis>
          <template v-if="hasScanDetails(record)">
            <NAlert
              v-for="group in scanDiagnosticGroups(record)"
              :key="`${record.id}-${group.code}`"
              :type="scanSeverityAlertTypes[group.severity]"
              :show-icon="false"
            >
              <NSpace vertical size="small">
                <NSpace align="center" size="small">
                  <NText strong>{{ group.label }}</NText>
                  <NTag size="small" round>{{ group.items.length }}</NTag>
                </NSpace>
                <NText
                  v-for="item in visibleScanEntries(group.items, false)"
                  :key="`${item.code}-${item.path}-${item.reason}`"
                >
                  <template v-if="item.code === 'duplicateBook' && item.duplicateOf">
                    {{ item.title || fileName(item.path) }}：{{ fileName(item.path) }} 与 {{ fileName(item.duplicateOf) }}
                  </template>
                  <template v-else>
                    {{ fileName(item.path) }}：{{ item.reason }}
                  </template>
                  <NText v-if="item.suggestion" depth="3">（{{ item.suggestion }}）</NText>
                </NText>
              </NSpace>
            </NAlert>
          </template>
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
