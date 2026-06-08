<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { NAlert, NButton, NCard, NEmpty, NEllipsis, NForm, NInput, NModal, NPageHeader, NSpace, NSpin, NTag, NText, useMessage } from 'naive-ui'
import { toArchiveUrl } from '@/api/archive'
import { getBook, setBookFavorite, updateBookMetadata } from '@/api/library'
import { markBookRead, markBookUnread } from '@/api/reader'
import { toAssetUrl, type Book, type Chapter } from '@/api/tauri'
import { getReadingStatus, getReadingStatusLabel } from '@/utils/readingStatus'
import { getReadingProgressPercent } from '@/utils/readingProgress'

const props = defineProps<{ bookId: string }>()

const router = useRouter()
const message = useMessage()
const book = ref<Book | null>(null)
const loading = ref(true)
const favoriteLoading = ref(false)
const progressActionLoading = ref(false)
const metadataSubmitting = ref(false)
const metadataDialogVisible = ref(false)
const metadataTitle = ref('')
const metadataAuthors = ref('')
const metadataTags = ref('')
const metadataDescription = ref('')
const error = ref('')

const coverUrl = computed(() => {
  if (!book.value?.coverPath) return undefined
  if (book.value.kind !== 'folder') {
    return toArchiveUrl(book.value.path, book.value.coverPath)
  }
  return toAssetUrl(book.value.coverPath)
})

const progressPercent = computed(() => {
  return book.value ? getReadingProgressPercent(book.value) : 0
})

const readingStatus = computed(() => book.value ? getReadingStatus(book.value) : 'unread')
const readingStatusLabel = computed(() => getReadingStatusLabel(readingStatus.value))
const hasProgress = computed(() => readingStatus.value !== 'unread')

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

async function markRead() {
  if (!book.value || progressActionLoading.value) return
  progressActionLoading.value = true
  error.value = ''
  try {
    book.value = await markBookRead(book.value.id)
    message.success('已标记为已读完')
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    progressActionLoading.value = false
  }
}

async function markUnread() {
  if (!book.value || progressActionLoading.value) return
  progressActionLoading.value = true
  error.value = ''
  try {
    book.value = await markBookUnread(book.value.id)
    message.success('已标记为未阅读')
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    progressActionLoading.value = false
  }
}

function openMetadataDialog() {
  if (!book.value) return
  metadataTitle.value = book.value.title
  metadataAuthors.value = book.value.authors.join(', ')
  metadataTags.value = book.value.tags.join(', ')
  metadataDescription.value = book.value.description ?? ''
  metadataDialogVisible.value = true
}

function splitCommaValues(value: string) {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item)
}

async function submitMetadata() {
  if (!book.value || metadataSubmitting.value) return
  const title = metadataTitle.value.trim()
  if (!title) {
    error.value = '漫画标题不能为空'
    return
  }

  metadataSubmitting.value = true
  error.value = ''
  try {
    book.value = await updateBookMetadata({
      bookPath: book.value.path,
      title,
      description: metadataDescription.value.trim() || null,
      authors: splitCommaValues(metadataAuthors.value),
      tags: splitCommaValues(metadataTags.value),
    })
    metadataDialogVisible.value = false
    message.success('元数据已保存')
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    metadataSubmitting.value = false
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
          <NButton v-if="book" @click="openMetadataDialog">编辑元数据</NButton>
          <NButton v-if="book && readingStatus !== 'read'" :loading="progressActionLoading" @click="markRead">
            标记已读
          </NButton>
          <NButton v-if="book && readingStatus !== 'unread'" :loading="progressActionLoading" @click="markUnread">
            标记未读
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
                <NTag
                  round
                  :type="readingStatus === 'read' ? 'success' : readingStatus === 'reading' ? 'info' : 'default'"
                >
                  {{ readingStatusLabel }}
                </NTag>
              </NSpace>

              <div class="book-progress">
                <span class="book-progress-bar">
                  <span :style="{ width: `${progressPercent}%` }" />
                </span>
                <NText depth="3">
                  {{ hasProgress ? `${readingStatusLabel} ${progressPercent}% · 第 ${book.lastPage + 1} 页` : readingStatusLabel }}
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

    <NModal
      v-model:show="metadataDialogVisible"
      preset="card"
      title="编辑元数据"
      class="favorite-modal"
      :style="{ width: 'min(560px, calc(100vw - 32px))' }"
    >
      <NForm @submit.prevent="submitMetadata">
        <NSpace vertical size="large">
          <NInput v-model:value="metadataTitle" placeholder="标题" />
          <NInput v-model:value="metadataAuthors" placeholder="作者，多个用英文逗号分隔" />
          <NInput v-model:value="metadataTags" placeholder="标签，多个用英文逗号分隔" />
          <NInput
            v-model:value="metadataDescription"
            type="textarea"
            placeholder="描述"
            :autosize="{ minRows: 3, maxRows: 8 }"
          />
          <NSpace justify="end">
            <NButton :disabled="metadataSubmitting" @click="metadataDialogVisible = false">取消</NButton>
            <NButton
              type="primary"
              attr-type="submit"
              :loading="metadataSubmitting"
              :disabled="!metadataTitle.trim()"
            >
              保存
            </NButton>
          </NSpace>
        </NSpace>
      </NForm>
    </NModal>
  </section>
</template>
