<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import {
  NAlert,
  NButton,
  NCard,
  NCheckbox,
  NDropdown,
  NEmpty,
  NForm,
  NInput,
  NList,
  NListItem,
  NModal,
  NPageHeader,
  NPagination,
  NSelect,
  NSpace,
  NSpin,
  NText,
  useMessage,
  type SelectOption,
} from 'naive-ui'
import {
  addBookToFavoriteCollection,
  createFavoriteCollection,
  ensureBookThumbnails,
  listBookFavoriteCollections,
  listBooks,
  listBookTags,
  listFavoriteCollections,
  removeBookFromFavoriteCollection,
  renameBookTitle,
  resetBookTitle,
} from '@/api/library'
import { getLibraryViewSettings, saveLibraryViewSettings } from '@/api/settings'
import BookList from '@/components/library/BookList.vue'
import LibraryViewSettingsPanel from '@/components/library/LibraryViewSettingsPanel.vue'
import type { BookSummary, FavoriteCollection, LibraryViewSettings } from '@/api/tauri'
import type { BookSortKey, SortDirection } from '@/utils/bookSort'

const defaultViewSettings: LibraryViewSettings = {
  layout: 'grid',
  coverSize: 'medium',
  showAuthors: true,
  showTags: true,
  tagLimit: 4,
}

const libraryStateKey = 'inkreader:library-list-state'
const defaultPageSize = 80
const pageSizeOptions: SelectOption[] = [
  { label: '每页 40 本', value: 40 },
  { label: '每页 80 本', value: 80 },
  { label: '每页 120 本', value: 120 },
  { label: '每页 200 本', value: 200 },
]
const searchDebounceMs = 250

type LibraryListState = {
  query: string
  selectedTags: string[]
  sortKey: BookSortKey
  sortDirection: SortDirection
  pageSize: number
  currentPage: number
}

const defaultLibraryState: LibraryListState = {
  query: '',
  selectedTags: [],
  sortKey: 'createdAt',
  sortDirection: 'desc',
  pageSize: defaultPageSize,
  currentPage: 1,
}

const router = useRouter()
const message = useMessage()
const libraryState = loadLibraryState()
const books = ref<BookSummary[]>([])
const totalBooks = ref(0)
const favoriteCollections = ref<FavoriteCollection[]>([])
const loading = ref(true)
const pageLoading = ref(false)
const error = ref('')
const query = ref(libraryState.query)
const debouncedQuery = ref(libraryState.query)
const selectedTags = ref<string[]>(libraryState.selectedTags)
const sortKey = ref<BookSortKey>(libraryState.sortKey)
const sortDirection = ref<SortDirection>(libraryState.sortDirection)
const currentPage = ref(libraryState.currentPage)
const pageSize = ref(libraryState.pageSize)
const allTags = ref<string[]>([])
const showViewSettings = ref(false)
const viewSettings = ref<LibraryViewSettings>({ ...defaultViewSettings })
const favoriteDialogBook = ref<BookSummary | null>(null)
const favoriteDialogCollections = ref<Set<string>>(new Set())
const favoriteDialogLoading = ref(false)
const newCollectionName = ref('')
const contextMenuBook = ref<BookSummary | null>(null)
const contextMenuVisible = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const renameDialogBook = ref<BookSummary | null>(null)
const renameTitleValue = ref('')
const renameSubmitting = ref(false)
let searchTimer: number | undefined
let requestToken = 0
let thumbnailRequestToken = 0
let initialized = false

const sortKeyOptions: SelectOption[] = [
  { label: '最近阅读', value: 'lastReadAt' },
  { label: '创建时间', value: 'createdAt' },
  { label: '名称', value: 'title' },
  { label: '页数', value: 'totalPages' },
]

const sortDirectionOptions: SelectOption[] = [
  { label: '降序', value: 'desc' },
  { label: '升序', value: 'asc' },
]

const tagOptions = computed<SelectOption[]>(() => allTags.value.map((tag) => ({ label: tag, value: tag })))
const bookContextMenuOptions = computed(() => [
  { label: '重命名标题', key: 'rename' },
  { label: '恢复默认标题', key: 'reset', disabled: !contextMenuBook.value?.titleOverride },
])
const pageCount = computed(() => Math.max(1, Math.ceil(totalBooks.value / pageSize.value)))
const hasFilters = computed(() => Boolean(debouncedQuery.value.trim() || selectedTags.value.length))
const shouldShowToolbar = computed(() => totalBooks.value > 0 || books.value.length > 0 || hasFilters.value)

async function loadInitialData() {
  loading.value = true
  error.value = ''
  try {
    const nextSettings = await getLibraryViewSettings()
    viewSettings.value = { ...defaultViewSettings, ...nextSettings }
    await loadBooks()
    initialized = true
    void loadDeferredLibraryData()
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

async function loadDeferredLibraryData() {
  const [nextCollections, nextTags] = await Promise.all([
    listFavoriteCollections().catch(() => favoriteCollections.value),
    listBookTags().catch(() => allTags.value),
  ])
  favoriteCollections.value = nextCollections
  allTags.value = nextTags
}

async function loadBooks() {
  const token = ++requestToken
  pageLoading.value = true
  error.value = ''
  try {
    const response = await listBooks({
      query: debouncedQuery.value,
      tags: selectedTags.value,
      sortKey: sortKey.value,
      sortDirection: sortDirection.value,
      limit: pageSize.value,
      offset: (currentPage.value - 1) * pageSize.value,
    })
    if (token !== requestToken) return
    books.value = response.books
    totalBooks.value = response.total
    void hydrateVisibleBookThumbnails(response.books)
    if (currentPage.value > pageCount.value) {
      currentPage.value = pageCount.value
      saveLibraryState()
      await loadBooks()
    }
  } catch (innerError) {
    if (token === requestToken) error.value = String(innerError)
  } finally {
    if (token === requestToken) pageLoading.value = false
  }
}

async function hydrateVisibleBookThumbnails(sourceBooks: BookSummary[]) {
  const missingThumbnailIds = sourceBooks
    .filter((book) => book.coverPath && !book.thumbnailPath)
    .map((book) => book.id)
  if (!missingThumbnailIds.length) return

  const token = ++thumbnailRequestToken
  const thumbnails = await ensureBookThumbnails(missingThumbnailIds).catch(() => [])
  if (token !== thumbnailRequestToken || !thumbnails.length) return

  const thumbnailByBookId = new Map(
    thumbnails
      .filter((thumbnail) => thumbnail.thumbnailPath)
      .map((thumbnail) => [thumbnail.bookId, thumbnail.thumbnailPath]),
  )
  if (!thumbnailByBookId.size) return

  books.value = books.value.map((book) => {
    const thumbnailPath = thumbnailByBookId.get(book.id)
    return thumbnailPath ? { ...book, thumbnailPath } : book
  })
}

function loadLibraryState(): LibraryListState {
  try {
    const rawValue = window.localStorage.getItem(libraryStateKey)
      ?? window.sessionStorage.getItem(libraryStateKey)
    if (!rawValue) return { ...defaultLibraryState }

    const value = JSON.parse(rawValue) as Partial<LibraryListState>
    const legacyValue = value as Partial<LibraryListState> & { selectedTag?: unknown }
    return {
      query: typeof value.query === 'string' ? value.query : defaultLibraryState.query,
      selectedTags: Array.isArray(value.selectedTags)
        ? value.selectedTags.filter((tag): tag is string => typeof tag === 'string')
        : typeof legacyValue.selectedTag === 'string' && legacyValue.selectedTag
          ? [legacyValue.selectedTag]
          : defaultLibraryState.selectedTags,
      sortKey: isBookSortKey(value.sortKey) ? value.sortKey : defaultLibraryState.sortKey,
      sortDirection: isSortDirection(value.sortDirection) ? value.sortDirection : defaultLibraryState.sortDirection,
      pageSize: isPageSize(value.pageSize) ? value.pageSize : defaultLibraryState.pageSize,
      currentPage: isPositiveInteger(value.currentPage) ? value.currentPage : defaultLibraryState.currentPage,
    }
  } catch {
    return { ...defaultLibraryState }
  }
}

function saveLibraryState() {
  window.localStorage.setItem(libraryStateKey, JSON.stringify({
    query: query.value,
    selectedTags: selectedTags.value,
    sortKey: sortKey.value,
    sortDirection: sortDirection.value,
    pageSize: pageSize.value,
    currentPage: currentPage.value,
  }))
}

function isBookSortKey(value: unknown): value is BookSortKey {
  return value === 'title' || value === 'totalPages' || value === 'createdAt' || value === 'lastReadAt'
}

function isSortDirection(value: unknown): value is SortDirection {
  return value === 'asc' || value === 'desc'
}

function isPageSize(value: unknown): value is number {
  return pageSizeOptions.some((option) => option.value === value)
}

function isPositiveInteger(value: unknown): value is number {
  return Number.isInteger(value) && Number(value) > 0
}

function updateSelectedTags(value: string[] | null) {
  selectedTags.value = value ?? []
}

function selectCardTag(tag: string) {
  if (!selectedTags.value.includes(tag)) {
    selectedTags.value = [...selectedTags.value, tag]
  }
}

function openBookContextMenu(payload: { book: BookSummary, x: number, y: number }) {
  contextMenuBook.value = payload.book
  contextMenuX.value = payload.x
  contextMenuY.value = payload.y
  contextMenuVisible.value = false
  requestAnimationFrame(() => {
    contextMenuVisible.value = true
  })
}

function handleBookContextMenuSelect(key: string | number) {
  const book = contextMenuBook.value
  contextMenuVisible.value = false
  if (!book) return

  if (key === 'rename') {
    renameDialogBook.value = book
    renameTitleValue.value = book.title
  } else if (key === 'reset') {
    void resetBookTitleFromMenu(book)
  }
}

function closeRenameDialog() {
  if (renameSubmitting.value) return
  renameDialogBook.value = null
  renameTitleValue.value = ''
}

function replaceBook(updated: BookSummary) {
  books.value = books.value.map((book) => book.path === updated.path ? { ...book, ...updated } : book)
  if (favoriteDialogBook.value?.path === updated.path) {
    favoriteDialogBook.value = { ...favoriteDialogBook.value, ...updated }
  }
  if (contextMenuBook.value?.path === updated.path) {
    contextMenuBook.value = { ...contextMenuBook.value, ...updated }
  }
}

async function submitRenameTitle() {
  const book = renameDialogBook.value
  const title = renameTitleValue.value.trim()
  if (!book || !title || renameSubmitting.value) return

  renameSubmitting.value = true
  error.value = ''
  try {
    const updated = await renameBookTitle(book.path, title)
    replaceBook(updated)
    renameDialogBook.value = null
    renameTitleValue.value = ''
    message.success('漫画标题已重命名')
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    renameSubmitting.value = false
  }
}

async function resetBookTitleFromMenu(book: BookSummary) {
  error.value = ''
  try {
    const updated = await resetBookTitle(book.path)
    replaceBook(updated)
    message.success('已恢复默认标题')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

function filterTagOption(pattern: string, option: SelectOption) {
  return fuzzyMatch(String(option.label ?? ''), normalizeText(pattern))
}

function openBook(book: BookSummary) {
  router.push(`/reader/${book.id}`)
}

function openBookDetail(book: BookSummary) {
  router.push(`/books/${book.id}`)
}

async function scrollListToTop() {
  await nextTick()
  document.querySelector<HTMLElement>('.main-panel')?.scrollTo({ top: 0, behavior: 'smooth' })
}

async function toggleFavorite(book: BookSummary) {
  await openFavoriteDialog(book)
}

async function openFavoriteDialog(book: BookSummary) {
  favoriteDialogBook.value = book
  favoriteDialogLoading.value = true
  error.value = ''
  try {
    const [collections, memberships] = await Promise.all([
      listFavoriteCollections(),
      listBookFavoriteCollections(book.path),
    ])
    favoriteCollections.value = collections
    favoriteDialogCollections.value = new Set(memberships.map((collection) => collection.id))
  } catch (innerError) {
    error.value = String(innerError)
    favoriteDialogBook.value = null
  } finally {
    favoriteDialogLoading.value = false
  }
}

async function toggleCollectionMembership(collection: FavoriteCollection) {
  const book = favoriteDialogBook.value
  if (!book) return

  const previousMemberships = new Set(favoriteDialogCollections.value)
  const nextMemberships = new Set(favoriteDialogCollections.value)
  const isMember = nextMemberships.has(collection.id)
  if (isMember) {
    nextMemberships.delete(collection.id)
  } else {
    nextMemberships.add(collection.id)
  }
  favoriteDialogCollections.value = nextMemberships
  error.value = ''

  try {
    if (isMember) {
      await removeBookFromFavoriteCollection(book.path, collection.id)
    } else {
      await addBookToFavoriteCollection(book.path, collection.id)
    }
    book.isFavorite = nextMemberships.size > 0
    books.value = books.value.map((item) => item.path === book.path ? { ...item, isFavorite: book.isFavorite } : item)
    await refreshFavoriteCollections()
  } catch (innerError) {
    favoriteDialogCollections.value = previousMemberships
    book.isFavorite = previousMemberships.size > 0
    books.value = books.value.map((item) => item.path === book.path ? { ...item, isFavorite: book.isFavorite } : item)
    error.value = String(innerError)
  }
}

async function createCollectionFromDialog() {
  const name = newCollectionName.value.trim()
  if (!name) return
  error.value = ''
  try {
    const collection = await createFavoriteCollection(name)
    favoriteCollections.value = [...favoriteCollections.value, collection]
    newCollectionName.value = ''
    await toggleCollectionMembership(collection)
    message.success(`已新建收藏夹「${collection.name}」`)
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function refreshFavoriteCollections() {
  favoriteCollections.value = await listFavoriteCollections()
}

function closeFavoriteDialog() {
  favoriteDialogBook.value = null
  favoriteDialogCollections.value = new Set()
  newCollectionName.value = ''
}

async function saveViewSettings() {
  error.value = ''
  try {
    await saveLibraryViewSettings(viewSettings.value)
    showViewSettings.value = false
    message.success('显示设置已保存')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

function normalizeText(value: string) {
  return value.trim().toLocaleLowerCase()
}

function fuzzyMatch(value: string, normalizedQuery: string) {
  const normalizedValue = normalizeText(value)
  if (!normalizedQuery || normalizedValue.includes(normalizedQuery)) return true

  let queryIndex = 0
  for (const char of normalizedValue) {
    if (char === normalizedQuery[queryIndex]) queryIndex += 1
    if (queryIndex >= normalizedQuery.length) return true
  }
  return false
}

watch(query, () => {
  saveLibraryState()
  if (searchTimer) window.clearTimeout(searchTimer)
  searchTimer = window.setTimeout(() => {
    debouncedQuery.value = query.value
  }, searchDebounceMs)
})

watch([selectedTags, sortKey, sortDirection, pageSize], () => {
  saveLibraryState()
  if (!initialized) return
  currentPage.value = 1
  void loadBooks()
})

watch(debouncedQuery, () => {
  if (!initialized) return
  currentPage.value = 1
  saveLibraryState()
  void loadBooks()
})

watch(currentPage, async () => {
  saveLibraryState()
  if (!initialized) return
  await loadBooks()
  await scrollListToTop()
})

onMounted(loadInitialData)
onMounted(() => {
  window.addEventListener('inkreader:auto-scan-complete', handleAutoScanComplete)
})
onBeforeUnmount(() => {
  if (searchTimer) window.clearTimeout(searchTimer)
  window.removeEventListener('inkreader:auto-scan-complete', handleAutoScanComplete)
})

function handleAutoScanComplete() {
  if (!initialized) return
  void loadBooks()
}
</script>

<template>
  <section class="page-section">
    <div class="page-sticky-region library-sticky-region">
      <NPageHeader>
        <template #title>书架</template>
        <template #subtitle>从已添加仓库中扫描出的漫画会显示在这里。</template>
        <template #extra>
          <NSpace>
            <NButton @click="showViewSettings = !showViewSettings">显示设置</NButton>
            <RouterLink to="/repositories" custom v-slot="{ navigate }">
              <NButton @click="navigate">管理仓库</NButton>
            </RouterLink>
          </NSpace>
        </template>
      </NPageHeader>

      <NCard v-if="showViewSettings" class="view-settings-wrap" title="显示设置">
        <LibraryViewSettingsPanel v-model="viewSettings" />
        <template #action>
          <NSpace justify="end">
            <NButton @click="showViewSettings = false">取消</NButton>
            <NButton type="primary" @click="saveViewSettings">保存设置</NButton>
          </NSpace>
        </template>
      </NCard>

      <NCard v-if="shouldShowToolbar" class="toolbar-card" :bordered="false">
        <NSpace align="center" :wrap="true">
          <NInput v-model:value="query" clearable placeholder="搜索标题、作者、标签" class="search-input" />
          <NSelect
            :value="selectedTags"
            clearable
            filterable
            multiple
            placeholder="筛选标签"
            :options="tagOptions"
            :filter="filterTagOption"
            class="tag-select"
            @update:value="updateSelectedTags"
          />
          <NSelect v-model:value="sortKey" :options="sortKeyOptions" class="sort-select" />
          <NSelect v-model:value="sortDirection" :options="sortDirectionOptions" class="sort-select" />
          <NSelect v-model:value="pageSize" :options="pageSizeOptions" class="sort-select" />
          <NText depth="3">第 {{ currentPage }} / {{ pageCount }} 页，{{ books.length }} / {{ totalBooks }} 本</NText>
        </NSpace>
      </NCard>

      <NCard v-if="!loading && totalBooks > pageSize" class="toolbar-card" :bordered="false">
        <NSpace justify="center" align="center">
          <NPagination v-model:page="currentPage" :page-count="pageCount" />
        </NSpace>
      </NCard>
    </div>

    <NAlert v-if="error" type="error" class="state-block" :show-icon="false">
      {{ error }}
    </NAlert>

    <NSpin v-if="loading" class="state-block" description="正在加载书架..." />

    <template v-else-if="books.length">
      <NSpin :show="pageLoading" description="正在加载当前页...">
        <BookList
          :books="books"
          :settings="viewSettings"
          @open="openBook"
          @detail="openBookDetail"
          @toggle-favorite="toggleFavorite"
          @select-tag="selectCardTag"
          @book-context-menu="openBookContextMenu"
        />
      </NSpin>
    </template>

    <NEmpty v-else-if="hasFilters" class="state-block" description="没有匹配结果">
      <template #extra>
        <NText depth="3">调整搜索词或标签筛选。</NText>
      </template>
    </NEmpty>

    <NEmpty v-else class="state-block" description="书架是空的">
      <template #extra>
        <NSpace vertical align="center">
          <NText depth="3">先添加一个漫画仓库，InkReader 会读取元数据、封面和章节。</NText>
          <RouterLink to="/repositories" custom v-slot="{ navigate }">
            <NButton type="primary" @click="navigate">添加仓库</NButton>
          </RouterLink>
        </NSpace>
      </template>
    </NEmpty>

    <NDropdown
      placement="bottom-start"
      trigger="manual"
      :x="contextMenuX"
      :y="contextMenuY"
      :show="contextMenuVisible"
      :options="bookContextMenuOptions"
      @select="handleBookContextMenuSelect"
      @clickoutside="contextMenuVisible = false"
    />

    <NModal
      :show="Boolean(renameDialogBook)"
      preset="card"
      title="重命名漫画标题"
      class="favorite-modal"
      :style="{ width: 'min(480px, calc(100vw - 32px))' }"
      @update:show="(value) => { if (!value) closeRenameDialog() }"
    >
      <form v-if="renameDialogBook" @submit.prevent="submitRenameTitle">
        <NSpace vertical size="large">
          <NText depth="3">默认标题：{{ renameDialogBook.scannedTitle }}</NText>
          <NInput v-model:value="renameTitleValue" placeholder="输入新的漫画标题" autofocus />
          <NSpace justify="end">
            <NButton :disabled="renameSubmitting" @click="closeRenameDialog">取消</NButton>
            <NButton type="primary" attr-type="submit" :loading="renameSubmitting" :disabled="!renameTitleValue.trim()">
              保存
            </NButton>
          </NSpace>
        </NSpace>
      </form>
    </NModal>

    <NModal
      :show="Boolean(favoriteDialogBook)"
      preset="card"
      title="管理收藏夹"
      class="favorite-modal"
      :style="{ width: 'min(520px, calc(100vw - 32px))' }"
      @update:show="(value) => { if (!value) closeFavoriteDialog() }"
    >
      <NSpace v-if="favoriteDialogBook" vertical size="large">
        <NText depth="3">{{ favoriteDialogBook.title }}</NText>

        <NSpin v-if="favoriteDialogLoading" description="正在加载收藏夹..." />
        <NList v-else bordered hoverable>
          <NListItem v-for="collection in favoriteCollections" :key="collection.id">
            <NSpace align="center" justify="space-between">
              <NCheckbox
                :checked="favoriteDialogCollections.has(collection.id)"
                @update:checked="() => toggleCollectionMembership(collection)"
              >
                {{ collection.name }}
              </NCheckbox>
              <NText depth="3">{{ collection.bookCount }} 本</NText>
            </NSpace>
          </NListItem>
        </NList>

        <NForm @submit.prevent="createCollectionFromDialog">
          <NSpace>
            <NInput v-model:value="newCollectionName" placeholder="新收藏夹名称" />
            <NButton type="primary" attr-type="submit">新建并加入</NButton>
          </NSpace>
        </NForm>
      </NSpace>
    </NModal>
  </section>
</template>
