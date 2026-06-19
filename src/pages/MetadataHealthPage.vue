<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  NAlert,
  NButton,
  NCard,
  NEmpty,
  NEllipsis,
  NGrid,
  NGridItem,
  NPageHeader,
  NSpace,
  NSpin,
  NTag,
  NText,
} from 'naive-ui'
import { listMetadataHealth } from '@/api/metadataHealth'
import type { MetadataHealthBookIssue, MetadataHealthSummary } from '@/api/tauri'

const emptySummary: MetadataHealthSummary = {
  missingMetadata: [],
  missingCovers: [],
  noImageIssues: [],
  duplicateIssues: [],
}

const router = useRouter()
const loading = ref(false)
const error = ref('')
const summary = ref<MetadataHealthSummary>(emptySummary)

const totalIssues = computed(
  () =>
    summary.value.missingMetadata.length +
    summary.value.missingCovers.length +
    summary.value.noImageIssues.length +
    summary.value.duplicateIssues.length,
)

const healthCards = computed(() => [
  { label: '元数据缺失', count: summary.value.missingMetadata.length, type: 'warning' as const },
  { label: '封面缺失', count: summary.value.missingCovers.length, type: 'warning' as const },
  { label: '无图片结果', count: summary.value.noImageIssues.length, type: 'error' as const },
  { label: '疑似重复', count: summary.value.duplicateIssues.length, type: 'warning' as const },
])

async function loadHealth() {
  loading.value = true
  error.value = ''
  try {
    summary.value = await listMetadataHealth()
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

function openBook(issue: MetadataHealthBookIssue) {
  router.push(`/books/${issue.book.id}`)
}

function openRepositories() {
  router.push('/repositories')
}

function fileName(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() || path
}

function formatDateTime(value: string) {
  const date = new Date(value)
  if (!Number.isFinite(date.getTime())) return value
  return date.toLocaleString('zh-CN')
}

onMounted(loadHealth)
</script>

<template>
  <section class="page-section">
    <NPageHeader>
      <template #title>健康检查</template>
      <template #subtitle>按本地记录和最近扫描结果列出需要人工确认的项目。</template>
      <template #extra>
        <NButton :loading="loading" @click="loadHealth">刷新</NButton>
      </template>
    </NPageHeader>

    <NAlert v-if="error" type="error" class="state-block" :show-icon="false">
      {{ error }}
    </NAlert>

    <NSpin v-if="loading && !totalIssues" class="state-block" description="正在检查书库健康状态..." />

    <NGrid v-if="!loading || totalIssues" :cols="4" :x-gap="14" :y-gap="14" responsive="screen" class="health-grid">
      <NGridItem v-for="card in healthCards" :key="card.label">
        <NCard :bordered="false" class="health-stat-card">
          <NSpace vertical size="small">
            <NText depth="3">{{ card.label }}</NText>
            <NSpace align="center" size="small">
              <NText class="health-stat-number">{{ card.count }}</NText>
              <NTag :type="card.count ? card.type : 'success'" round>
                {{ card.count ? '待处理' : '正常' }}
              </NTag>
            </NSpace>
          </NSpace>
        </NCard>
      </NGridItem>
    </NGrid>

    <NEmpty v-if="!loading && !error && !totalIssues" class="state-block" description="没有发现健康问题" />

    <NCard v-if="summary.missingMetadata.length" :bordered="false" class="toolbar-card">
      <NSpace vertical size="small">
        <h2 class="section-title">元数据缺失</h2>
        <div v-for="issue in summary.missingMetadata" :key="issue.book.path" class="health-row">
          <div class="health-row-main">
            <NText strong>{{ issue.book.title }}</NText>
            <NEllipsis class="path-text">{{ issue.book.path }}</NEllipsis>
          </div>
          <NSpace align="center" :wrap="true">
            <NTag v-for="reason in issue.reasons" :key="reason" size="small" type="warning" round>
              {{ reason }}
            </NTag>
            <NButton size="small" secondary @click="openBook(issue)">详情</NButton>
          </NSpace>
        </div>
      </NSpace>
    </NCard>

    <NCard v-if="summary.missingCovers.length" :bordered="false" class="toolbar-card">
      <NSpace vertical size="small">
        <h2 class="section-title">封面缺失</h2>
        <div v-for="issue in summary.missingCovers" :key="issue.book.path" class="health-row">
          <div class="health-row-main">
            <NText strong>{{ issue.book.title }}</NText>
            <NEllipsis class="path-text">{{ issue.book.path }}</NEllipsis>
          </div>
          <NSpace align="center" :wrap="true">
            <NTag v-for="reason in issue.reasons" :key="reason" size="small" type="warning" round>
              {{ reason }}
            </NTag>
            <NButton size="small" secondary @click="openBook(issue)">详情</NButton>
          </NSpace>
        </div>
      </NSpace>
    </NCard>

    <NCard v-if="summary.noImageIssues.length" :bordered="false" class="toolbar-card">
      <NSpace vertical size="small">
        <h2 class="section-title">无图片扫描结果</h2>
        <div v-for="issue in summary.noImageIssues" :key="`${issue.repositoryId}-${issue.path}`" class="health-row">
          <div class="health-row-main">
            <NText strong>{{ fileName(issue.path) }}</NText>
            <NEllipsis class="path-text">{{ issue.path }}</NEllipsis>
            <NText depth="3">{{ issue.repositoryName }} · {{ formatDateTime(issue.scannedAt) }}</NText>
            <NText depth="3">{{ issue.reason }}</NText>
            <NText v-if="issue.suggestion" depth="3">{{ issue.suggestion }}</NText>
          </div>
          <NSpace align="center" :wrap="true">
            <NTag size="small" type="error" round>{{ issue.code }}</NTag>
            <NButton size="small" secondary @click="openRepositories">仓库</NButton>
          </NSpace>
        </div>
      </NSpace>
    </NCard>

    <NCard v-if="summary.duplicateIssues.length" :bordered="false" class="toolbar-card">
      <NSpace vertical size="small">
        <h2 class="section-title">疑似重复</h2>
        <div
          v-for="issue in summary.duplicateIssues"
          :key="`${issue.repositoryId}-${issue.path}-${issue.duplicateOf}`"
          class="health-row"
        >
          <div class="health-row-main">
            <NText strong>{{ issue.title || fileName(issue.path) }}</NText>
            <NEllipsis class="path-text">{{ issue.path }}</NEllipsis>
            <NEllipsis class="path-text">重复对象：{{ issue.duplicateOf }}</NEllipsis>
            <NText depth="3">{{ issue.repositoryName }} · {{ formatDateTime(issue.scannedAt) }}</NText>
          </div>
          <NSpace align="center" :wrap="true">
            <NTag size="small" type="warning" round>duplicateBook</NTag>
            <NButton size="small" secondary @click="openRepositories">仓库</NButton>
          </NSpace>
        </div>
      </NSpace>
    </NCard>
  </section>
</template>

<style scoped>
.health-grid {
  margin-bottom: 18px;
}

.health-stat-card {
  box-shadow: var(--shadow-soft);
}

.health-stat-number {
  color: var(--ink);
  font-size: 30px;
  font-weight: 800;
  line-height: 1;
}

.health-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 12px 0;
  border-top: 1px solid rgb(223 230 220 / 72%);
}

.health-row:first-of-type {
  border-top: 0;
}

.health-row-main {
  min-width: 0;
  display: grid;
  gap: 4px;
}

@media (max-width: 720px) {
  .health-row {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
