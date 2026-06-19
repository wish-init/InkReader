<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import {
  NAlert,
  NButton,
  NCard,
  NEmpty,
  NEllipsis,
  NPageHeader,
  NRadioButton,
  NRadioGroup,
  NSelect,
  NSpace,
  NSpin,
  NText,
  type SelectOption,
} from 'naive-ui'
import { toArchiveUrl } from '@/api/archive'
import { listReadingHistory, listReadingHistoryByBook } from '@/api/reader'
import { toAssetUrl, type ReadingHistoryRecord } from '@/api/tauri'

type ViewMode = 'books' | 'raw'
type GroupMode = 'day' | 'week' | 'month'

type ViewModeOption = {
  label: string
  value: ViewMode
}

type HistoryGroup = {
  key: string
  label: string
  records: ReadingHistoryRecord[]
}

const router = useRouter()
const bookRecords = ref<ReadingHistoryRecord[]>([])
const rawRecords = ref<ReadingHistoryRecord[]>([])
const loading = ref(true)
const error = ref('')
const viewMode = ref<ViewMode>('books')
const groupMode = ref<GroupMode>('day')

const viewModeOptions: ViewModeOption[] = [
  { label: '按书', value: 'books' },
  { label: '原始记录', value: 'raw' },
]

const groupModeOptions: SelectOption[] = [
  { label: '按日', value: 'day' },
  { label: '按周', value: 'week' },
  { label: '按月', value: 'month' },
]

const rawGroups = computed<HistoryGroup[]>(() => {
  const values = new Map<string, HistoryGroup>()
  for (const record of rawRecords.value) {
    const group = historyGroup(record.readAt, groupMode.value)
    const existing = values.get(group.key)
    if (existing) {
      existing.records.push(record)
    } else {
      values.set(group.key, { ...group, records: [record] })
    }
  }
  return [...values.values()]
})

async function loadHistory() {
  loading.value = true
  error.value = ''
  try {
    const [groupedHistory, rawHistory] = await Promise.all([
      listReadingHistoryByBook(),
      listReadingHistory(),
    ])
    bookRecords.value = groupedHistory
    rawRecords.value = rawHistory
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

function openRecord(record: ReadingHistoryRecord) {
  router.push(`/reader/${record.bookId}`)
}

function getRecordCoverUrl(record: ReadingHistoryRecord): string | undefined {
  if (record.bookKind !== 'folder' && record.coverPath) {
    return toArchiveUrl(record.bookPath, record.coverPath)
  }
  return toAssetUrl(record.coverPath)
}

function historyGroup(value: string, mode: GroupMode): Omit<HistoryGroup, 'records'> {
  const date = new Date(value)
  if (mode === 'month') {
    const key = `${date.getFullYear()}-${pad(date.getMonth() + 1)}`
    return { key, label: `${date.getFullYear()}年${date.getMonth() + 1}月` }
  }

  if (mode === 'week') {
    const start = startOfLocalWeek(date)
    const end = new Date(start)
    end.setDate(start.getDate() + 6)
    const key = formatDateKey(start)
    return { key, label: `${formatDateLabel(start)} 至 ${formatDateLabel(end)}` }
  }

  const key = formatDateKey(date)
  return { key, label: formatDateLabel(date) }
}

function startOfLocalWeek(date: Date) {
  const value = new Date(date.getFullYear(), date.getMonth(), date.getDate())
  const day = value.getDay() || 7
  value.setDate(value.getDate() - day + 1)
  return value
}

function formatDateKey(date: Date) {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

function formatDateLabel(date: Date) {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

function formatReadTime(value: string) {
  return new Date(value).toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function progressLabel(record: ReadingHistoryRecord) {
  return `${record.chapterTitle || '未知章节'} · 读到第 ${record.page + 1} 页`
}

function pad(value: number) {
  return String(value).padStart(2, '0')
}

onMounted(loadHistory)
</script>

<template>
  <section class="page-section">
    <NPageHeader>
      <template #title>阅读记录</template>
      <template #subtitle>按书查看最近阅读进度，也可以切换到原始阅读记录。</template>
      <template #extra>
        <div class="history-toolbar">
          <NRadioGroup v-model:value="viewMode" size="small">
            <NRadioButton
              v-for="option in viewModeOptions"
              :key="String(option.value)"
              :value="option.value"
              :label="option.label"
            />
          </NRadioGroup>
          <NSelect
            v-if="viewMode === 'raw'"
            v-model:value="groupMode"
            :options="groupModeOptions"
            class="sort-select"
          />
        </div>
      </template>
    </NPageHeader>

    <NAlert v-if="error" type="error" class="state-block" :show-icon="false">
      {{ error }}
    </NAlert>
    <NSpin v-if="loading" class="state-block" description="正在加载阅读记录..." />

    <NSpace v-else-if="viewMode === 'books' && bookRecords.length" vertical size="small" class="history-groups">
      <NCard
        v-for="record in bookRecords"
        :key="record.bookId"
        embedded
        :bordered="false"
        class="history-card"
        role="button"
        tabindex="0"
        @click="openRecord(record)"
        @keydown.enter="openRecord(record)"
        @keydown.space.prevent="openRecord(record)"
      >
        <div class="history-card-body">
          <div class="history-cover">
            <img v-if="getRecordCoverUrl(record)" :src="getRecordCoverUrl(record)" :alt="record.bookTitle" />
            <div v-else class="cover-placeholder">无封面</div>
          </div>
          <div class="history-info">
            <NEllipsis class="history-title">{{ record.bookTitle }}</NEllipsis>
            <NText depth="3">{{ progressLabel(record) }}</NText>
            <NText depth="3">最近阅读 {{ formatReadTime(record.readAt) }}</NText>
          </div>
        </div>
      </NCard>
    </NSpace>

    <NSpace v-else-if="viewMode === 'raw' && rawGroups.length" vertical size="large" class="history-groups">
      <section v-for="group in rawGroups" :key="group.key" class="history-group">
        <h2 class="section-title">{{ group.label }}</h2>
        <NSpace vertical size="small">
          <NCard
            v-for="record in group.records"
            :key="record.id"
            embedded
            :bordered="false"
            class="history-card"
            role="button"
            tabindex="0"
            @click="openRecord(record)"
            @keydown.enter="openRecord(record)"
            @keydown.space.prevent="openRecord(record)"
          >
            <div class="history-card-body">
              <div class="history-cover">
                <img v-if="getRecordCoverUrl(record)" :src="getRecordCoverUrl(record)" :alt="record.bookTitle" />
                <div v-else class="cover-placeholder">无封面</div>
              </div>
              <div class="history-info">
                <NEllipsis class="history-title">{{ record.bookTitle }}</NEllipsis>
                <NText depth="3">{{ progressLabel(record) }}</NText>
                <NText depth="3">{{ formatReadTime(record.readAt) }}</NText>
              </div>
            </div>
          </NCard>
        </NSpace>
      </section>
    </NSpace>

    <NEmpty v-else class="state-block" description="还没有阅读记录">
      <template #extra>
        <NSpace vertical align="center">
          <NText depth="3">进入漫画并翻页后，阅读记录会显示在这里。</NText>
          <RouterLink to="/library" custom v-slot="{ navigate }">
            <NButton type="primary" @click="navigate">去书架看看</NButton>
          </RouterLink>
        </NSpace>
      </template>
    </NEmpty>
  </section>
</template>
