<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { NAlert, NButton, NCard, NEmpty, NInput, NPageHeader, NPagination, NSpace, NSpin, NTag, NText } from 'naive-ui'
import {
  listBookAuthorAggregations,
  listBooks,
  listBookTagAggregations,
} from '@/api/library'
import BookList from '@/components/library/BookList.vue'
import { useBookListController } from '@/composables/library/useBookListController'
import { useBookThumbnailHydration } from '@/composables/library/useBookThumbnailHydration'
import { useLibraryViewSettings } from '@/composables/library/useLibraryViewSettings'
import type { BookAggregationItem, BookSummary } from '@/api/tauri'

const props = defineProps<{
  mode: 'authors' | 'tags'
}>()

const router = useRouter()
const error = ref('')
const loading = ref(true)
const aggregations = ref<BookAggregationItem[]>([])
const aggregationQuery = ref('')
const debouncedAggregationQuery = ref('')
const selectedName = ref('')
const defaultPageSize = 80
const searchDebounceMs = 250
const { viewSettings, loadLibraryViewSettings } = useLibraryViewSettings({ error })
const {
  books,
  totalBooks,
  pageLoading,
  sortKey,
  sortDirection,
  currentPage,
  pageSize,
  pageCount,
  loadBooks,
  loadFirstPage,
  markInitialized,
} = useBookListController({
  defaultPageSize,
  searchDebounceMs,
  initialSortKey: 'createdAt',
  initialSortDirection: 'desc',
  error,
  load: (request) => listBooks({
    query: '',
    author: props.mode === 'authors' ? selectedName.value : null,
    tag: props.mode === 'tags' ? selectedName.value : null,
    sortKey: request.sortKey,
    sortDirection: request.sortDirection,
    limit: request.limit,
    offset: request.offset,
  }),
  onPageChanged: scrollListToTop,
  onItemsLoaded: (response) => {
    void hydrateBookThumbnails(response.books)
  },
})
const { hydrateBookThumbnails } = useBookThumbnailHydration(books)

const title = computed(() => (props.mode === 'authors' ? '作者' : '标签'))
const emptyDescription = computed(() => `暂无${title.value}`)
const selectedAggregation = computed(() => aggregations.value.find((item) => item.name === selectedName.value) ?? null)
const filteredAggregations = computed(() => aggregations.value)
let aggregationSearchTimer: number | undefined

async function loadPage() {
  loading.value = true
  error.value = ''
  try {
    const [items] = await Promise.all([
      props.mode === 'authors'
        ? listBookAuthorAggregations(debouncedAggregationQuery.value)
        : listBookTagAggregations(debouncedAggregationQuery.value),
      loadLibraryViewSettings(),
    ])
    aggregations.value = items
    selectedName.value = items[0]?.name ?? ''
    if (selectedName.value) await loadBooks()
    markInitialized()
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

async function selectAggregation(name: string) {
  selectedName.value = name
  await loadFirstPage()
}

function openBook(book: BookSummary) {
  router.push(`/reader/${book.id}`)
}

function openBookDetail(book: BookSummary) {
  router.push(`/books/${book.id}`)
}

function selectCardAuthor(author: string) {
  if (props.mode === 'authors') {
    void selectAggregation(author)
  }
}

function selectCardTag(tag: string) {
  if (props.mode === 'tags') {
    void selectAggregation(tag)
  }
}

async function scrollListToTop() {
  document.querySelector<HTMLElement>('.main-panel')?.scrollTo({ top: 0, behavior: 'smooth' })
}

watch(() => props.mode, loadPage)
watch(aggregationQuery, () => {
  if (aggregationSearchTimer) window.clearTimeout(aggregationSearchTimer)
  aggregationSearchTimer = window.setTimeout(() => {
    debouncedAggregationQuery.value = aggregationQuery.value
  }, searchDebounceMs)
})
watch(debouncedAggregationQuery, loadPage)

onMounted(loadPage)
onBeforeUnmount(() => {
  if (aggregationSearchTimer) window.clearTimeout(aggregationSearchTimer)
})
</script>

<template>
  <section class="page-section">
    <NPageHeader>
      <template #title>{{ title }}</template>
      <template #subtitle>按{{ title }}浏览本地书库</template>
    </NPageHeader>

    <NAlert v-if="error" type="error" class="state-block" :show-icon="false">
      {{ error }}
    </NAlert>

    <div class="aggregation-layout">
      <NCard class="aggregation-sidebar" :bordered="false">
        <NInput v-model:value="aggregationQuery" clearable :placeholder="`搜索${title}`" />
        <NSpin v-if="loading" class="state-block" description="正在加载..." />
        <NEmpty v-else-if="!filteredAggregations.length" :description="emptyDescription" />
        <div v-else class="aggregation-list">
          <button
            v-for="item in filteredAggregations"
            :key="item.name"
            class="aggregation-item"
            :class="{ active: item.name === selectedName }"
            type="button"
            @click="selectAggregation(item.name)"
          >
            <span>{{ item.name }}</span>
            <NTag size="small" round>{{ item.count }}</NTag>
          </button>
        </div>
      </NCard>

      <div class="aggregation-main">
        <NCard v-if="selectedAggregation" :bordered="false" class="toolbar-card">
          <NSpace justify="space-between" align="center" :wrap="true">
            <div>
              <h2 class="section-title">{{ selectedAggregation.name }}</h2>
              <NText depth="3">{{ totalBooks }} / {{ selectedAggregation.count }} 本</NText>
            </div>
            <NSpace>
              <NButton @click="router.push('/library')">返回书架</NButton>
              <NTag round type="success">后端筛选</NTag>
            </NSpace>
          </NSpace>
        </NCard>

        <NCard v-if="selectedAggregation && totalBooks > pageSize" :bordered="false" class="toolbar-card">
          <NSpace justify="center" align="center">
            <NPagination v-model:page="currentPage" :page-count="pageCount" />
          </NSpace>
        </NCard>

        <NSpin v-if="loading" class="state-block" description="正在加载..." />
        <NEmpty v-else-if="!selectedAggregation" class="state-block" :description="emptyDescription" />
        <NSpin v-else :show="pageLoading" description="正在加载当前页...">
          <BookList
            :books="books"
            :settings="viewSettings"
            highlight-query=""
            @open="openBook"
            @detail="openBookDetail"
            @select-author="selectCardAuthor"
            @select-tag="selectCardTag"
          />
        </NSpin>
      </div>
    </div>
  </section>
</template>
