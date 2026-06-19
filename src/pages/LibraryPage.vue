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
  NTag,
  NText,
  useDialog,
  useMessage,
  type SelectOption,
} from 'naive-ui'
import {
  addBookToFavoriteCollection,
  createFavoriteCollection,
  listBookAuthors,
  listBookFavoriteCollections,
  listBooks,
  listBookTags,
  listFavoriteCollections,
  removeBookFromFavoriteCollection,
  resetBookTitle,
  updateBookAuthors,
  updateBookTags,
} from '@/api/library'
import { listRepositories } from '@/api/repositories'
import { markBookRead, markBookUnread } from '@/api/reader'
import BookList from '@/components/library/BookList.vue'
import LibraryViewSettingsPanel from '@/components/library/LibraryViewSettingsPanel.vue'
import { useBookListController } from '@/composables/library/useBookListController'
import { useBookContextMenu } from '@/composables/library/useBookContextMenu'
import { useBookRenameDialog } from '@/composables/library/useBookRenameDialog'
import { useBookSelection } from '@/composables/library/useBookSelection'
import { useBookThumbnailHydration } from '@/composables/library/useBookThumbnailHydration'
import { useLibraryViewSettings } from '@/composables/library/useLibraryViewSettings'
import type { BookSummary, FavoriteCollection, FavoriteStatus, MetadataFilter, ReadingStatus, Repository } from '@/api/tauri'
import {
  batchConfirmationMessage,
  batchResultSummary,
  createBatchPartialResult,
  sourceFilesSafeNotice,
  type BatchOperationFailure,
  type BatchOperationItem,
  type BatchOperationResult,
} from '@/utils/batchOperations'
import type { BookSortKey, SortDirection } from '@/utils/bookSort'
import { getReadingStatus } from '@/utils/readingStatus'
import {
  createSavedLibraryView,
  deleteSavedLibraryView,
  loadSavedLibraryViews,
  renameSavedLibraryView,
  type SavedLibraryView,
} from '@/utils/savedLibraryViews'
import {
  loadRecentLibraryFilters,
  saveRecentLibraryFilter,
  type RecentLibraryFilter,
  type RecentLibraryFilterState,
} from '@/utils/recentLibraryFilters'

const libraryStateKey = 'inkreader:library-list-state'
const libraryScrollStateKey = 'inkreader:library-scroll-state'
const defaultPageSize = 80
const initialRenderedBookCount = 24
const renderedBookBatchSize = 12
const renderedBookBatchDelayMs = 80
const pageSizeOptions: SelectOption[] = [
  { label: '每页 40 本', value: 40 },
  { label: '每页 80 本', value: 80 },
  { label: '每页 120 本', value: 120 },
  { label: '每页 200 本', value: 200 },
]
const searchDebounceMs = 250

type LibraryListState = {
  query: string
  repositoryId: string | null
  selectedAuthors: string[]
  selectedTags: string[]
  excludedTags: string[]
  metadataFilters: MetadataFilter[]
  readingStatus: ReadingStatus
  favoriteStatus: FavoriteStatus
  sortKey: BookSortKey
  sortDirection: SortDirection
  pageSize: number
  currentPage: number
}

type LibraryScrollState = {
  scrollTop: number
  signature: string
}

const defaultLibraryState: LibraryListState = {
  query: '',
  repositoryId: null,
  selectedAuthors: [],
  selectedTags: [],
  excludedTags: [],
  metadataFilters: [],
  readingStatus: 'all',
  favoriteStatus: 'all',
  sortKey: 'createdAt',
  sortDirection: 'desc',
  pageSize: defaultPageSize,
  currentPage: 1,
}

const router = useRouter()
const dialog = useDialog()
const message = useMessage()
const libraryState = loadLibraryState()
const renderedBookCount = ref(0)
const favoriteCollections = ref<FavoriteCollection[]>([])
const loading = ref(true)
const error = ref('')
const selectedRepositoryId = ref<string | null>(libraryState.repositoryId)
const selectedAuthors = ref<string[]>(libraryState.selectedAuthors)
const selectedTags = ref<string[]>(libraryState.selectedTags)
const excludedTags = ref<string[]>(libraryState.excludedTags)
const metadataFilters = ref<MetadataFilter[]>(libraryState.metadataFilters)
const readingStatus = ref<ReadingStatus>(libraryState.readingStatus)
const favoriteStatus = ref<FavoriteStatus>(libraryState.favoriteStatus)
const repositories = ref<Repository[]>([])
const allAuthors = ref<string[]>([])
const allTags = ref<string[]>([])
const showViewSettings = ref(false)
const showAdvancedFilters = ref(false)
const savedViews = ref<SavedLibraryView[]>(loadSavedLibraryViews('library'))
const recentFilters = ref<RecentLibraryFilter[]>(loadRecentLibraryFilters('library'))
const selectedSavedViewId = ref<string | null>(null)
const savedViewModalVisible = ref(false)
const savedViewName = ref('')
const renameSavedViewId = ref<string | null>(null)
const renameSavedViewName = ref('')
const {
  viewSettings,
  loadLibraryViewSettings,
  saveViewSettings: persistViewSettings,
} = useLibraryViewSettings({
  error,
  onSaveSuccess: (value) => message.success(value),
})
const favoriteDialogBook = ref<BookSummary | null>(null)
const favoriteDialogCollections = ref<Set<string>>(new Set())
const favoriteDialogLoading = ref(false)
const newCollectionName = ref('')
const batchActionLoading = ref(false)
const batchResult = ref<BatchOperationResult | null>(null)
const batchMetadataModalVisible = ref(false)
const batchMetadataMode = ref<'tags' | 'authors'>('tags')
const batchMetadataText = ref('')
const {
  contextMenuBook,
  contextMenuVisible,
  contextMenuX,
  contextMenuY,
  openBookContextMenu,
  closeBookContextMenu,
} = useBookContextMenu()
const {
  renameDialogBook,
  renameTitleValue,
  renameSubmitting,
  openRenameDialog,
  closeRenameDialog,
  submitRenameTitle,
  resetBookTitleFromMenu,
} = useBookRenameDialog({
  error,
  replaceBook,
  onSuccess: (value) => message.success(value),
})
const {
  books,
  totalBooks,
  pageLoading,
  query,
  debouncedQuery,
  sortKey,
  sortDirection,
  currentPage,
  pageSize,
  pageCount,
  loadBooks,
  loadFirstPage,
  markInitialized: markBookListInitialized,
  setQueryNow,
} = useBookListController({
  defaultPageSize,
  searchDebounceMs,
  initialQuery: libraryState.query,
  initialSortKey: libraryState.sortKey,
  initialSortDirection: libraryState.sortDirection,
  initialPageSize: libraryState.pageSize,
  initialCurrentPage: libraryState.currentPage,
  error,
  load: (request) => listBooks({
    repositoryId: selectedRepositoryId.value,
    query: request.query,
    authors: selectedAuthors.value,
    tags: selectedTags.value,
    excludeTags: excludedTags.value,
    metadataFilters: metadataFilters.value,
    readingStatus: readingStatus.value,
    favoriteStatus: favoriteStatus.value,
    sortKey: request.sortKey,
    sortDirection: request.sortDirection,
    limit: request.limit,
    offset: request.offset,
  }),
  onStateChanged: saveLibraryState,
  onItemsLoaded: (response) => {
    pruneSelectionToCurrentBooks()
    scheduleBookRendering(response.books.length, loadLibraryScrollTop() > 0)
    void hydrateBookThumbnails(response.books)
  },
  onPageChanged: scrollListToTop,
})
const {
  selectedBookPaths,
  selectedBooks,
  selectedCount,
  allCurrentPageSelected,
  clearSelection,
  toggleBookSelection,
  toggleSelectAllCurrentPage,
  selectedItems,
  pruneSelectionToCurrentBooks,
} = useBookSelection(books)
const { hydrateBookThumbnails } = useBookThumbnailHydration(books)
let renderBatchTimer: number | undefined
let initialized = false

const sortKeyOptions: SelectOption[] = [
  { label: '最近阅读', value: 'lastReadAt' },
  { label: '漫画发布时间', value: 'publishedAt' },
  { label: '创建时间', value: 'createdAt' },
  { label: '名称', value: 'title' },
  { label: '页数', value: 'totalPages' },
]

const sortDirectionOptions: SelectOption[] = [
  { label: '降序', value: 'desc' },
  { label: '升序', value: 'asc' },
]

const readingStatusOptions: SelectOption[] = [
  { label: '全部', value: 'all' },
  { label: '未阅读', value: 'unread' },
  { label: '阅读中', value: 'reading' },
  { label: '已读完', value: 'read' },
]

const favoriteStatusOptions: SelectOption[] = [
  { label: '全部收藏状态', value: 'all' },
  { label: '已收藏', value: 'favorited' },
  { label: '未收藏', value: 'notFavorited' },
]

const repositoryOptions = computed<SelectOption[]>(() => [
  { label: '全部仓库', value: 'all' },
  ...repositories.value.map((repository) => ({
    label: repository.name,
    value: repository.id,
  })),
])
const authorOptions = computed<SelectOption[]>(() => allAuthors.value.map((author) => ({ label: author, value: author })))
const tagOptions = computed<SelectOption[]>(() => allTags.value.map((tag) => ({ label: tag, value: tag })))
const metadataFilterOptions: SelectOption[] = [
  { label: '缺简介', value: 'missingDescription' },
  { label: '缺作者', value: 'missingAuthors' },
  { label: '缺标签', value: 'missingTags' },
  { label: '缺封面', value: 'missingCover' },
  { label: '缺发布时间', value: 'missingPublishedAt' },
]
const savedViewOptions = computed<SelectOption[]>(() => [
  { label: '不使用保存视图', value: 'none' },
  ...savedViews.value.map((view) => ({ label: view.name, value: view.id })),
])
const metadataModalTitle = computed(() => (
  batchMetadataMode.value === 'tags' ? '批量设置标签' : '批量设置作者'
))
const batchMetadataValueCount = computed(() => normalizeBatchTextValues(batchMetadataText.value).length)
const bookContextMenuOptions = computed(() => [
  { label: '重命名标题', key: 'rename' },
  { label: '恢复默认标题', key: 'reset', disabled: !contextMenuBook.value?.titleOverride },
  { label: '标记已读', key: 'mark-read', disabled: !contextMenuBook.value || getReadingStatus(contextMenuBook.value) === 'read' },
  { label: '标记未读', key: 'mark-unread', disabled: !contextMenuBook.value || getReadingStatus(contextMenuBook.value) === 'unread' },
])
const hasLibraryFilters = computed(() => Boolean(
  debouncedQuery.value.trim()
    || selectedRepositoryId.value
    || selectedAuthors.value.length
    || selectedTags.value.length
    || excludedTags.value.length
    || metadataFilters.value.length
    || readingStatus.value !== 'all'
    || favoriteStatus.value !== 'all',
))
const shouldShowToolbar = computed(() => totalBooks.value > 0 || books.value.length > 0 || hasLibraryFilters.value)
const visibleBooks = computed(() => books.value.slice(0, renderedBookCount.value))

async function loadInitialData() {
  loading.value = true
  error.value = ''
  let shouldRestoreScroll = false
  try {
    const [, nextRepositories] = await Promise.all([
      loadLibraryViewSettings(),
      listRepositories(),
      loadBooks(),
    ])
    repositories.value = nextRepositories
    initialized = true
    markBookListInitialized()
    shouldRestoreScroll = true
    void loadDeferredLibraryData()
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
  if (shouldRestoreScroll) await restoreLibraryScrollPosition()
}

async function loadDeferredLibraryData() {
  const [nextCollections, nextAuthors, nextTags] = await Promise.all([
    listFavoriteCollections().catch(() => favoriteCollections.value),
    listBookAuthors(selectedRepositoryId.value ?? undefined).catch(() => allAuthors.value),
    listBookTags(selectedRepositoryId.value ?? undefined).catch(() => allTags.value),
  ])
  favoriteCollections.value = nextCollections
  allAuthors.value = nextAuthors
  allTags.value = nextTags
}

function scheduleBookRendering(total: number, renderAll = false) {
  if (renderBatchTimer) window.clearTimeout(renderBatchTimer)
  if (renderAll) {
    renderedBookCount.value = total
    return
  }

  renderedBookCount.value = Math.min(total, initialRenderedBookCount)
  if (renderedBookCount.value < total) scheduleNextBookRenderBatch()
}

function scheduleNextBookRenderBatch() {
  renderBatchTimer = window.setTimeout(() => {
    renderedBookCount.value = Math.min(
      books.value.length,
      renderedBookCount.value + renderedBookBatchSize,
    )
    if (renderedBookCount.value < books.value.length) {
      scheduleNextBookRenderBatch()
    }
  }, renderedBookBatchDelayMs)
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
      repositoryId: typeof value.repositoryId === 'string' && value.repositoryId
        ? value.repositoryId
        : defaultLibraryState.repositoryId,
      selectedAuthors: normalizeStringArray(value.selectedAuthors),
      selectedTags: Array.isArray(value.selectedTags)
        ? value.selectedTags.filter((tag): tag is string => typeof tag === 'string')
        : typeof legacyValue.selectedTag === 'string' && legacyValue.selectedTag
          ? [legacyValue.selectedTag]
          : defaultLibraryState.selectedTags,
      excludedTags: normalizeStringArray(value.excludedTags),
      metadataFilters: normalizeMetadataFilters(value.metadataFilters),
      readingStatus: isReadingStatus(value.readingStatus) ? value.readingStatus : defaultLibraryState.readingStatus,
      favoriteStatus: isFavoriteStatus(value.favoriteStatus) ? value.favoriteStatus : defaultLibraryState.favoriteStatus,
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
    repositoryId: selectedRepositoryId.value,
    selectedAuthors: selectedAuthors.value,
    selectedTags: selectedTags.value,
    excludedTags: excludedTags.value,
    metadataFilters: metadataFilters.value,
    readingStatus: readingStatus.value,
    favoriteStatus: favoriteStatus.value,
    sortKey: sortKey.value,
    sortDirection: sortDirection.value,
    pageSize: pageSize.value,
    currentPage: currentPage.value,
  }))
}

function currentSavedLibraryViewState() {
  return {
    query: query.value,
    repositoryId: selectedRepositoryId.value,
    authors: selectedAuthors.value,
    selectedTags: selectedTags.value,
    excludeTags: excludedTags.value,
    metadataFilters: metadataFilters.value,
    readingStatus: readingStatus.value,
    favoriteStatus: favoriteStatus.value,
    sortKey: sortKey.value,
    sortDirection: sortDirection.value,
    pageSize: pageSize.value,
    viewSettings: viewSettings.value,
  }
}

function openSaveViewModal() {
  savedViewName.value = ''
  savedViewModalVisible.value = true
}

function saveCurrentView() {
  savedViews.value = createSavedLibraryView('library', savedViewName.value, currentSavedLibraryViewState())
  selectedSavedViewId.value = savedViews.value.at(-1)?.id ?? null
  savedViewModalVisible.value = false
  message.success('已保存视图')
}

async function applySavedView(value: string | null) {
  const id = value && value !== 'none' ? value : null
  selectedSavedViewId.value = id
  if (!id) return
  const savedView = savedViews.value.find((view) => view.id === id)
  if (!savedView) return

  const state = savedView.state
  setQueryNow(state.query)
  selectedRepositoryId.value = state.repositoryId ?? null
  selectedAuthors.value = state.authors ?? []
  selectedTags.value = state.selectedTags ?? []
  excludedTags.value = state.excludeTags ?? []
  metadataFilters.value = state.metadataFilters ?? []
  readingStatus.value = state.readingStatus ?? 'all'
  favoriteStatus.value = state.favoriteStatus ?? 'all'
  sortKey.value = state.sortKey
  sortDirection.value = state.sortDirection
  pageSize.value = state.pageSize
  viewSettings.value = state.viewSettings
  saveLibraryState()
  allAuthors.value = await listBookAuthors(selectedRepositoryId.value ?? undefined).catch(() => [])
  allTags.value = await listBookTags(selectedRepositoryId.value ?? undefined).catch(() => [])
  if (initialized) await loadFirstPage()
}

function beginRenameSavedView(view: SavedLibraryView) {
  renameSavedViewId.value = view.id
  renameSavedViewName.value = view.name
}

function submitRenameSavedView() {
  const id = renameSavedViewId.value
  if (!id) return
  savedViews.value = renameSavedLibraryView('library', id, renameSavedViewName.value)
  renameSavedViewId.value = null
  renameSavedViewName.value = ''
}

function removeSavedView(id: string) {
  savedViews.value = deleteSavedLibraryView('library', id)
  if (selectedSavedViewId.value === id) selectedSavedViewId.value = null
}

function getLibraryScrollElements() {
  const mainPanel = document.querySelector<HTMLElement>('.main-panel')
  if (!mainPanel) return []

  return [
    mainPanel,
    ...Array.from(mainPanel.querySelectorAll<HTMLElement>('.n-layout-scroll-container')),
  ]
}

function getLibraryScrollTop() {
  return Math.max(0, ...getLibraryScrollElements().map((element) => element.scrollTop))
}

function getLibraryScrollSignature() {
  return JSON.stringify({
    query: debouncedQuery.value,
    repositoryId: selectedRepositoryId.value,
    selectedAuthors: selectedAuthors.value,
    selectedTags: selectedTags.value,
    excludedTags: excludedTags.value,
    metadataFilters: metadataFilters.value,
    readingStatus: readingStatus.value,
    favoriteStatus: favoriteStatus.value,
    sortKey: sortKey.value,
    sortDirection: sortDirection.value,
    pageSize: pageSize.value,
    currentPage: currentPage.value,
  })
}

function saveLibraryScrollTop(scrollTop: number) {
  const scrollState: LibraryScrollState = {
    scrollTop: Math.max(0, Math.round(scrollTop)),
    signature: getLibraryScrollSignature(),
  }
  window.sessionStorage.setItem(libraryScrollStateKey, JSON.stringify(scrollState))
}

function saveLibraryScrollPosition() {
  saveLibraryScrollTop(getLibraryScrollTop())
}

function loadLibraryScrollTop() {
  try {
    const rawValue = window.sessionStorage.getItem(libraryScrollStateKey)
    if (!rawValue) return 0

    const value = JSON.parse(rawValue) as Partial<LibraryScrollState>
    if (value.signature !== getLibraryScrollSignature()) return 0
    return typeof value.scrollTop === 'number' && Number.isFinite(value.scrollTop)
      ? Math.max(0, value.scrollTop)
      : 0
  } catch {
    return 0
  }
}

async function restoreLibraryScrollPosition() {
  const scrollTop = loadLibraryScrollTop()
  if (scrollTop <= 0) return

  await nextTick()
  await new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => resolve())
  })
  getLibraryScrollElements().forEach((element) => {
    element.scrollTo({ top: scrollTop, behavior: 'auto' })
  })
}

function isBookSortKey(value: unknown): value is BookSortKey {
  return value === 'title'
    || value === 'totalPages'
    || value === 'createdAt'
    || value === 'lastReadAt'
    || value === 'publishedAt'
}

function isSortDirection(value: unknown): value is SortDirection {
  return value === 'asc' || value === 'desc'
}

function isReadingStatus(value: unknown): value is ReadingStatus {
  return value === 'all' || value === 'unread' || value === 'reading' || value === 'read'
}

function isFavoriteStatus(value: unknown): value is FavoriteStatus {
  return value === 'all' || value === 'favorited' || value === 'notFavorited'
}

function isPageSize(value: unknown): value is number {
  return pageSizeOptions.some((option) => option.value === value)
}

function isPositiveInteger(value: unknown): value is number {
  return Number.isInteger(value) && Number(value) > 0
}

function normalizeStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return Array.from(new Set(value.filter((item): item is string => typeof item === 'string').map((item) => item.trim()).filter(Boolean)))
}

function normalizeMetadataFilters(value: unknown): MetadataFilter[] {
  if (!Array.isArray(value)) return []
  return value.filter((item): item is MetadataFilter => (
    item === 'missingDescription'
      || item === 'missingAuthors'
      || item === 'missingTags'
      || item === 'missingCover'
      || item === 'missingPublishedAt'
  ))
}

function updateSelectedAuthors(value: string[] | null) {
  selectedAuthors.value = value ?? []
}

function updateSelectedTags(value: string[] | null) {
  const nextTags = value ?? []
  selectedTags.value = nextTags
  excludedTags.value = excludedTags.value.filter((tag) => !nextTags.includes(tag))
}

function updateExcludedTags(value: string[] | null) {
  const nextTags = value ?? []
  excludedTags.value = nextTags
  selectedTags.value = selectedTags.value.filter((tag) => !nextTags.includes(tag))
}

function updateSelectedRepository(value: string | null) {
  selectedRepositoryId.value = value && value !== 'all' ? value : null
}

function selectCardTag(tag: string) {
  if (!selectedTags.value.includes(tag)) {
    selectedTags.value = [...selectedTags.value, tag]
  }
}

function selectCardAuthor(author: string) {
  const value = author.trim()
  if (!value) return
  if (!selectedAuthors.value.includes(value)) {
    selectedAuthors.value = [...selectedAuthors.value, value]
  }
}

function currentRecentFilterState(): RecentLibraryFilterState {
  return {
    query: query.value,
    authors: selectedAuthors.value,
    tags: selectedTags.value,
    excludeTags: excludedTags.value,
    metadataFilters: metadataFilters.value,
  }
}

function saveCurrentRecentFilter() {
  recentFilters.value = saveRecentLibraryFilter('library', currentRecentFilterState())
}

async function applyRecentFilter(filter: RecentLibraryFilter) {
  setQueryNow(filter.state.query)
  selectedAuthors.value = filter.state.authors
  selectedTags.value = filter.state.tags
  excludedTags.value = filter.state.excludeTags
  metadataFilters.value = filter.state.metadataFilters
  saveLibraryState()
  if (initialized) await loadFirstPage()
}

function handleBookContextMenuSelect(key: string | number) {
  const book = contextMenuBook.value
  closeBookContextMenu()
  if (!book) return

  if (key === 'rename') {
    openRenameDialog(book)
  } else if (key === 'reset') {
    void resetBookTitleFromMenu(book)
  } else if (key === 'mark-read') {
    void markBookReadFromMenu(book)
  } else if (key === 'mark-unread') {
    void markBookUnreadFromMenu(book)
  }
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

async function markBookReadFromMenu(book: BookSummary) {
  error.value = ''
  try {
    const updated = await markBookRead(book.id)
    await applyBookStatusUpdate(updated)
    message.success('已标记为已读完')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function markBookUnreadFromMenu(book: BookSummary) {
  error.value = ''
  try {
    const updated = await markBookUnread(book.id)
    await applyBookStatusUpdate(updated)
    message.success('已标记为未阅读')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function applyBookStatusUpdate(updated: BookSummary) {
  replaceBook(updated)
  if (readingStatus.value !== 'all' || favoriteStatus.value !== 'all') {
    await loadBooks()
  }
}

function confirmDialog(title: string, content: string) {
  return new Promise<boolean>((resolve) => {
    let settled = false
    const settle = (value: boolean) => {
      if (settled) return
      settled = true
      resolve(value)
    }

    dialog.warning({
      title,
      content,
      positiveText: '确定',
      negativeText: '取消',
      onPositiveClick: () => settle(true),
      onNegativeClick: () => settle(false),
      onClose: () => settle(false),
    })
  })
}

function openBatchMetadataModal(mode: 'tags' | 'authors') {
  if (!selectedCount.value || batchActionLoading.value) return
  batchMetadataMode.value = mode
  batchMetadataText.value = ''
  batchMetadataModalVisible.value = true
  batchResult.value = null
}

function closeBatchMetadataModal() {
  batchMetadataModalVisible.value = false
  batchMetadataText.value = ''
}

async function submitBatchMetadataEdit() {
  const values = normalizeBatchTextValues(batchMetadataText.value)
  if (!selectedCount.value || !values.length) return

  const label = batchMetadataMode.value === 'tags' ? '标签' : '作者'
  const confirmed = await confirmDialog(
    `批量设置${label}`,
    batchConfirmationMessage(`设置${label}`, selectedCount.value),
  )
  if (!confirmed) return

  const operation = batchMetadataMode.value === 'tags' ? '已设置标签' : '已设置作者'
  await runSelectedBookBatch(operation, async (book) => {
    return batchMetadataMode.value === 'tags'
      ? updateBookTags(book.path, values)
      : updateBookAuthors(book.path, values)
  })
  closeBatchMetadataModal()
  if (batchMetadataMode.value === 'tags') {
    allTags.value = await listBookTags(selectedRepositoryId.value ?? undefined).catch(() => allTags.value)
  }
}

async function markSelectedBooksRead() {
  if (!selectedCount.value) return
  const confirmed = await confirmDialog(
    '批量标记已读',
    batchConfirmationMessage('标记已读', selectedCount.value),
  )
  if (!confirmed) return

  await runSelectedBookBatch('已标记已读', (book) => markBookRead(book.id))
}

async function markSelectedBooksUnread() {
  if (!selectedCount.value) return
  const confirmed = await confirmDialog(
    '批量标记未读',
    batchConfirmationMessage('标记未读', selectedCount.value),
  )
  if (!confirmed) return

  await runSelectedBookBatch('已标记未读', (book) => markBookUnread(book.id))
}

async function resetSelectedBookTitles() {
  if (!selectedCount.value) return
  const confirmed = await confirmDialog(
    '批量恢复扫描标题',
    batchConfirmationMessage('恢复扫描标题', selectedCount.value),
  )
  if (!confirmed) return

  await runSelectedBookBatch('已恢复扫描标题', (book) => resetBookTitle(book.path))
}

async function runSelectedBookBatch(
  operation: string,
  mutateBook: (book: BookSummary) => Promise<BookSummary>,
) {
  const sourceBooks = [...selectedBooks.value]
  const items = selectedItems()
  if (!sourceBooks.length) return

  error.value = ''
  batchResult.value = null
  batchActionLoading.value = true
  const failed: BatchOperationFailure[] = []
  const updatedBooks: BookSummary[] = []

  try {
    for (const book of sourceBooks) {
      try {
        updatedBooks.push(await mutateBook(book))
      } catch (innerError) {
        failed.push({
          path: book.path,
          title: book.title,
          reason: String(innerError),
        })
      }
    }

    updatedBooks.forEach(replaceBook)
    batchResult.value = createBatchPartialResult(operation, items, failed)
    if (failed.length) {
      message.warning(batchResultSummary(batchResult.value))
    } else {
      clearSelection()
      message.success(batchResultSummary(batchResult.value))
    }

    if (updatedBooks.length && (readingStatus.value !== 'all' || favoriteStatus.value !== 'all')) {
      await loadBooks()
    }
  } finally {
    batchActionLoading.value = false
  }
}

function normalizeBatchTextValues(value: string) {
  return [...new Set(value.split(/[,，\n]/).map((item) => item.trim()).filter(Boolean))]
}

function filterTagOption(pattern: string, option: SelectOption) {
  return fuzzyMatch(String(option.label ?? ''), normalizeText(pattern))
}

function openBook(book: BookSummary) {
  saveLibraryScrollPosition()
  router.push(`/reader/${book.id}`)
}

function openBookDetail(book: BookSummary) {
  saveLibraryScrollPosition()
  router.push(`/books/${book.id}`)
}

async function scrollListToTop() {
  await nextTick()
  getLibraryScrollElements().forEach((element) => {
    element.scrollTo({ top: 0, behavior: 'auto' })
  })
  saveLibraryScrollTop(0)
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
  try {
    await persistViewSettings()
    showViewSettings.value = false
  } catch {
    // Error state is owned by useLibraryViewSettings.
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

watch(selectedRepositoryId, async () => {
  selectedAuthors.value = []
  selectedTags.value = []
  excludedTags.value = []
  saveLibraryState()
  const [nextAuthors, nextTags] = await Promise.all([
    listBookAuthors(selectedRepositoryId.value ?? undefined).catch(() => []),
    listBookTags(selectedRepositoryId.value ?? undefined).catch(() => []),
  ])
  allAuthors.value = nextAuthors
  allTags.value = nextTags
  if (!initialized) return
  void loadFirstPage()
})

watch([selectedAuthors, selectedTags, excludedTags, metadataFilters, readingStatus, favoriteStatus], () => {
  saveLibraryState()
  if (!initialized) return
  void loadFirstPage()
})

onMounted(loadInitialData)
onMounted(() => {
  window.addEventListener('inkreader:auto-scan-complete', handleAutoScanComplete)
})
onBeforeUnmount(() => {
  saveLibraryScrollPosition()
  if (renderBatchTimer) window.clearTimeout(renderBatchTimer)
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
          <NSelect
            :value="selectedSavedViewId ?? 'none'"
            :options="savedViewOptions"
            class="saved-view-select"
            @update:value="applySavedView"
          />
          <NButton @click="openSaveViewModal">保存当前视图</NButton>
          <NTag v-if="selectedSavedViewId" round type="success">已应用保存视图</NTag>
        </NSpace>
        <div v-if="savedViews.length" class="saved-view-list">
          <div v-for="view in savedViews" :key="view.id" class="saved-view-item">
            <template v-if="renameSavedViewId === view.id">
              <NInput v-model:value="renameSavedViewName" size="small" class="saved-view-name-input" />
              <NButton size="small" type="primary" @click="submitRenameSavedView">保存</NButton>
              <NButton size="small" @click="renameSavedViewId = null">取消</NButton>
            </template>
            <template v-else>
              <NText>{{ view.name }}</NText>
              <NButton size="small" text @click="beginRenameSavedView(view)">重命名</NButton>
              <NButton size="small" text type="error" @click="removeSavedView(view.id)">删除</NButton>
            </template>
          </div>
        </div>
      </NCard>

      <NCard v-if="shouldShowToolbar" class="toolbar-card" :bordered="false">
        <NSpace align="center" :wrap="true">
          <NInput v-model:value="query" clearable placeholder="搜索标题、作者、标签" class="search-input" />
          <NSelect
            :value="selectedRepositoryId ?? 'all'"
            :options="repositoryOptions"
            class="sort-select"
            @update:value="updateSelectedRepository"
          />
          <NButton @click="showAdvancedFilters = !showAdvancedFilters">
            {{ showAdvancedFilters ? '收起筛选' : '高级筛选' }}
          </NButton>
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
          <NSelect v-model:value="readingStatus" :options="readingStatusOptions" class="sort-select" />
          <NSelect v-model:value="favoriteStatus" :options="favoriteStatusOptions" class="sort-select" />
          <NSelect v-model:value="sortKey" :options="sortKeyOptions" class="sort-select" />
          <NSelect v-model:value="sortDirection" :options="sortDirectionOptions" class="sort-select" />
          <NSelect v-model:value="pageSize" :options="pageSizeOptions" class="sort-select" />
          <NText depth="3">第 {{ currentPage }} / {{ pageCount }} 页，{{ books.length }} / {{ totalBooks }} 本</NText>
        </NSpace>
        <div v-if="showAdvancedFilters" class="advanced-filter-panel">
          <NSpace align="center" :wrap="true">
            <NSelect
              :value="selectedAuthors"
              clearable
              filterable
              multiple
              placeholder="筛选作者"
              :options="authorOptions"
              class="tag-select"
              @update:value="updateSelectedAuthors"
            />
            <NSelect
              :value="excludedTags"
              clearable
              filterable
              multiple
              placeholder="排除标签"
              :options="tagOptions"
              :filter="filterTagOption"
              class="tag-select"
              @update:value="updateExcludedTags"
            />
            <NSelect
              v-model:value="metadataFilters"
              clearable
              multiple
              placeholder="缺元数据"
              :options="metadataFilterOptions"
              class="tag-select"
            />
            <NButton secondary @click="saveCurrentRecentFilter">保存最近筛选</NButton>
          </NSpace>
          <NSpace v-if="recentFilters.length" class="recent-filter-list" align="center" :wrap="true">
            <NText depth="3">最近筛选</NText>
            <NButton
              v-for="filter in recentFilters"
              :key="filter.id"
              size="small"
              secondary
              @click="applyRecentFilter(filter)"
            >
              {{ filter.label }}
            </NButton>
          </NSpace>
        </div>
      </NCard>

      <NCard v-if="!loading && totalBooks > pageSize" class="toolbar-card" :bordered="false">
        <NSpace justify="center" align="center">
          <NPagination v-model:page="currentPage" :page-count="pageCount" />
        </NSpace>
      </NCard>

      <NAlert
        v-if="batchResult"
        :type="batchResult.failed.length ? 'warning' : 'success'"
        class="state-block"
        :show-icon="false"
      >
        <NSpace vertical size="small">
          <NText strong>{{ batchResultSummary(batchResult) }}</NText>
          <NText depth="3">{{ sourceFilesSafeNotice }}</NText>
          <NList v-if="batchResult.failed.length" bordered>
            <NListItem v-for="failure in batchResult.failed" :key="failure.path">
              <NSpace vertical size="small">
                <NText>{{ failure.title }}</NText>
                <NText depth="3">{{ failure.reason }}</NText>
              </NSpace>
            </NListItem>
          </NList>
        </NSpace>
      </NAlert>

      <NCard v-if="books.length" :bordered="false" class="toolbar-card">
        <NSpace justify="space-between" align="center" :wrap="true">
          <NSpace align="center">
            <NTag round type="success">已选择 {{ selectedCount }} 本</NTag>
            <NButton secondary size="small" @click="toggleSelectAllCurrentPage">
              {{ allCurrentPageSelected ? '取消选择' : '全选当前页' }}
            </NButton>
            <NButton v-if="selectedCount" text size="small" @click="clearSelection">清空</NButton>
          </NSpace>
          <NSpace>
            <NButton
              secondary
              :disabled="!selectedCount || batchActionLoading"
              @click="openBatchMetadataModal('tags')"
            >
              设置标签
            </NButton>
            <NButton
              secondary
              :disabled="!selectedCount || batchActionLoading"
              @click="openBatchMetadataModal('authors')"
            >
              设置作者
            </NButton>
            <NButton
              secondary
              :disabled="!selectedCount || batchActionLoading"
              :loading="batchActionLoading"
              @click="markSelectedBooksRead"
            >
              标记已读
            </NButton>
            <NButton
              secondary
              :disabled="!selectedCount || batchActionLoading"
              :loading="batchActionLoading"
              @click="markSelectedBooksUnread"
            >
              标记未读
            </NButton>
            <NButton
              secondary
              :disabled="!selectedCount || batchActionLoading"
              :loading="batchActionLoading"
              @click="resetSelectedBookTitles"
            >
              恢复扫描标题
            </NButton>
          </NSpace>
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
          :books="visibleBooks"
          :settings="viewSettings"
          :highlight-query="debouncedQuery"
          selectable
          :selected-book-paths="selectedBookPaths"
          @open="openBook"
          @detail="openBookDetail"
          @toggle-favorite="toggleFavorite"
          @toggle-selection="toggleBookSelection"
          @select-tag="selectCardTag"
          @select-author="selectCardAuthor"
          @book-context-menu="openBookContextMenu"
        />
      </NSpin>
    </template>

    <NEmpty v-else-if="hasLibraryFilters" class="state-block" description="没有匹配结果">
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
      @clickoutside="closeBookContextMenu"
    />

    <NModal
      :show="savedViewModalVisible"
      preset="card"
      title="保存当前视图"
      class="favorite-modal"
      :style="{ width: 'min(420px, calc(100vw - 32px))' }"
      @update:show="(value) => { savedViewModalVisible = value }"
    >
      <form @submit.prevent="saveCurrentView">
        <NSpace vertical size="large">
          <NInput v-model:value="savedViewName" placeholder="视图名称" autofocus />
          <NSpace justify="end">
            <NButton @click="savedViewModalVisible = false">取消</NButton>
            <NButton type="primary" attr-type="submit" :disabled="!savedViewName.trim()">保存</NButton>
          </NSpace>
        </NSpace>
      </form>
    </NModal>

    <NModal
      :show="batchMetadataModalVisible"
      preset="card"
      :title="metadataModalTitle"
      class="favorite-modal"
      :style="{ width: 'min(520px, calc(100vw - 32px))' }"
      @update:show="(value) => { if (!value) closeBatchMetadataModal() }"
    >
      <form @submit.prevent="submitBatchMetadataEdit">
        <NSpace vertical size="large">
          <NText depth="3">
            已选择 {{ selectedCount }} 本漫画。输入多个{{ batchMetadataMode === 'tags' ? '标签' : '作者' }}时可用逗号或换行分隔。{{ sourceFilesSafeNotice }}
          </NText>
          <NInput
            v-model:value="batchMetadataText"
            type="textarea"
            :placeholder="batchMetadataMode === 'tags' ? '例如：热血, 完结, 长篇' : '例如：作者 A, 作者 B'"
            :autosize="{ minRows: 4, maxRows: 8 }"
          />
          <NSpace justify="end">
            <NButton :disabled="batchActionLoading" @click="closeBatchMetadataModal">取消</NButton>
            <NButton
              type="primary"
              attr-type="submit"
              :loading="batchActionLoading"
              :disabled="!batchMetadataValueCount"
            >
              保存 {{ batchMetadataValueCount }} 项
            </NButton>
          </NSpace>
        </NSpace>
      </form>
    </NModal>

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
