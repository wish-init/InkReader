<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { NAlert, NButton, NCard, NEmpty, NEllipsis, NPageHeader, NSpace, NSpin, NTag, NText, useMessage } from 'naive-ui'
import { toArchiveUrl } from '@/api/archive'
import { getBook, setBookFavorite } from '@/api/library'
import { toAssetUrl, type Book, type Chapter } from '@/api/tauri'

const props = defineProps<{ bookId: string }>()

const router = useRouter()
const message = useMessage()
const book = ref<Book | null>(null)
const loading = ref(true)
const favoriteLoading = ref(false)
const error = ref('')

const coverUrl = computed(() => {
  if (!book.value?.coverPath) return undefined
  if (book.value.kind !== 'folder') {
    return toArchiveUrl(book.value.path, book.value.coverPath)
  }
  return toAssetUrl(book.value.coverPath)
})

const progressPercent = computed(() => {
  if (!book.value?.totalPages) return 0
  return Math.min(100, Math.round(((book.value.lastPage + 1) / book.value.totalPages) * 100))
})

const hasProgress = computed(() => Boolean(book.value?.lastReadAt || (book.value?.lastPage ?? 0) > 0))

async function loadBook() {
  loading.value = true
  error.value = ''
  try {
    book.value = await getBook(props.bookId)
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

function openReader() {
  if (!book.value) return
  router.push(`/reader/${book.value.id}`)
}

async function toggleFavorite() {
  if (!book.value || favoriteLoading.value) return
  favoriteLoading.value = true
  error.value = ''
  try {
    const nextFavorite = !book.value.isFavorite
    await setBookFavorite(book.value.path, nextFavorite)
    book.value.isFavorite = nextFavorite
    message.success(nextFavorite ? '已加入收藏' : '已移出收藏')
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    favoriteLoading.value = false
  }
}

function openChapter(chapter: Chapter) {
  if (!book.value) return
  const index = book.value.chapters.findIndex((item) => item.id === chapter.id)
  router.push(`/reader/${book.value.id}?chapter=${Math.max(index, 0)}`)
}

onMounted(loadBook)
</script>

<template>
  <section class="page-section">
    <NPageHeader @back="router.push('/library')">
      <template #title>漫画详情</template>
      <template #subtitle>
        <NEllipsis v-if="book" style="max-width: min(720px, 70vw)">{{ book.path }}</NEllipsis>
      </template>
      <template #extra>
        <NSpace>
          <NButton @click="router.push('/library')">返回书架</NButton>
          <NButton v-if="book" :loading="favoriteLoading" @click="toggleFavorite">
            {{ book.isFavorite ? '取消收藏' : '加入收藏' }}
          </NButton>
          <NButton v-if="book" type="primary" @click="openReader">
            {{ hasProgress ? '继续阅读' : '开始阅读' }}
          </NButton>
        </NSpace>
      </template>
    </NPageHeader>

    <NAlert v-if="error" type="error" class="state-block" :show-icon="false">
      {{ error }}
    </NAlert>

    <NSpin v-if="loading" class="state-block" description="正在加载漫画详情..." />

    <template v-else-if="book">
      <div class="book-detail-layout">
        <div class="book-detail-cover">
          <img v-if="coverUrl" :src="coverUrl" :alt="book.title" loading="eager" decoding="async" />
          <div v-else class="cover-placeholder">无封面</div>
        </div>

        <section class="book-detail-main">
          <NCard :bordered="false">
            <NSpace vertical size="medium">
              <div>
                <h1 class="book-detail-title">{{ book.title }}</h1>
                <NText v-if="book.titleOverride" depth="3">默认标题：{{ book.scannedTitle }}</NText>
              </div>

              <NSpace :wrap="true">
                <NTag round>{{ book.chapterCount }} 章</NTag>
                <NTag round>{{ book.totalPages }} 页</NTag>
                <NTag v-if="book.kind !== 'folder'" round>{{ book.kind.toUpperCase() }}</NTag>
                <NTag v-if="book.isFavorite" type="warning" round>已收藏</NTag>
              </NSpace>

              <div class="book-progress">
                <span class="book-progress-bar">
                  <span :style="{ width: `${progressPercent}%` }" />
                </span>
                <NText depth="3">
                  {{ hasProgress ? `阅读进度 ${progressPercent}% · 第 ${book.lastPage + 1} 页` : '还没有阅读进度' }}
                </NText>
              </div>

              <NText v-if="book.authors.length" depth="2">作者：{{ book.authors.join(' / ') }}</NText>
              <NText v-if="book.description" depth="2">{{ book.description }}</NText>

              <NSpace v-if="book.tags.length" :wrap="true">
                <NTag v-for="tag in book.tags" :key="tag" size="small" round>{{ tag }}</NTag>
              </NSpace>
            </NSpace>
          </NCard>

          <NCard title="章节" :bordered="false">
            <div class="chapter-list">
              <button
                v-for="chapter in book.chapters"
                :key="chapter.id"
                class="chapter-row"
                type="button"
                @click="openChapter(chapter)"
              >
                <span>{{ chapter.title }}</span>
                <NText depth="3">{{ chapter.pageCount }} 页</NText>
              </button>
            </div>
          </NCard>
        </section>
      </div>
    </template>

    <NEmpty v-else class="state-block" description="没有找到漫画" />
  </section>
</template>
