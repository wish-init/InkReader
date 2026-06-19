<script setup lang="ts">
import { open } from '@tauri-apps/plugin-dialog'
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import {
  NAlert,
  NButton,
  NCard,
  NCheckbox,
  NDropdown,
  NEmpty,
  NInput,
  NList,
  NListItem,
  NModal,
  NPageHeader,
  NPagination,
  NPopconfirm,
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
  addBooksToFavoriteCollection,
  createFavoriteCollection,
  deleteFavoriteCollection,
  listBookAuthors,
  listBookTags,
  listFavoriteBooks,
  listFavoriteCollections,
  moveBooksBetweenFavoriteCollections,
  removeBooksFromAllFavoriteCollections,
  removeBooksFromFavoriteCollection,
  renameFavoriteCollection,
  updateFavoriteCollectionMetadata,
} from '@/api/library'
import BookList from '@/components/library/BookList.vue'
import { useBookListController } from '@/composables/library/useBookListController'
import { useBookContextMenu } from '@/composables/library/useBookContextMenu'
import { useBookRenameDialog } from '@/composables/library/useBookRenameDialog'
import { useBookSelection } from '@/composables/library/useBookSelection'
import { useBookThumbnailHydration } from '@/composables/library/useBookThumbnailHydration'
import { useLibraryViewSettings } from '@/composables/library/useLibraryViewSettings'
import { toAssetUrl, type BookSummary, type FavoriteCollection, type MetadataFilter } from '@/api/tauri'
import {
  batchConfirmationMessage,
  batchResultSummary,
  createBatchFailureResult,
  createBatchSuccessResult,
  sourceFilesSafeNotice,
  type BatchOperationItem,
  type BatchOperationResult,
} from '@/utils/batchOperations'
import type { BookSortKey, SortDirection } from '@/utils/bookSort'
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

const defaultPageSize = 80
const searchDebounceMs = 250
const pageSizeOptions: SelectOption[] = [
  { label: '每页 40 本', value: 40 },
  { label: '每页 80 本', value: 80 },
  { label: '每页 120 本', value: 120 },
  { label: '每页 200 本', value: 200 },
]

const router = useRouter()
const dialog = useDialog()
const message = useMessage()
const collections = ref<FavoriteCollection[]>([])
const selectedCollectionId = ref<string | null>(null)
const selectedAuthors = ref<string[]>([])
const selectedTags = ref<string[]>([])
const excludedTags = ref<string[]>([])
const metadataFilters = ref<MetadataFilter[]>([])
const loading = ref(true)
const error = ref('')
const { viewSettings, loadLibraryViewSettings } = useLibraryViewSettings({ error })
const savedViews = ref<SavedLibraryView[]>(loadSavedLibraryViews('favorites'))
const recentFilters = ref<RecentLibraryFilter[]>(loadRecentLibraryFilters('favorites'))
const selectedSavedViewId = ref<string | null>(null)
const showAdvancedFilters = ref(false)
const savedViewModalVisible = ref(false)
const savedViewName = ref('')
const renameSavedViewId = ref<string | null>(null)
const renameSavedViewName = ref('')
const newCollectionName = ref('')
const allAuthors = ref<string[]>([])
const allTags = ref<string[]>([])
const renameValue = ref('')
const metadataModalVisible = ref(false)
const metadataCoverPath = ref('')
const metadataDescription = ref('')
const metadataSubmitting = ref(false)
const batchTargetModalVisible = ref(false)
const batchTargetMode = ref<'move' | 'add'>('move')
const batchActionLoading = ref(false)
const targetCollectionLoading = ref(false)
const targetNewCollectionName = ref('')
const batchResult = ref<BatchOperationResult | null>(null)
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
  hasFilters,
  loadBooks,
  loadFirstPage,
  markInitialized,
  setQueryNow,
} = useBookListController({
  defaultPageSize,
  searchDebounceMs,
  initialSortKey: 'createdAt',
  initialSortDirection: 'desc',
  error,
  load: (request) => listFavoriteBooks({
    collectionId: selectedCollectionId.value,
    query: request.query,
    authors: selectedAuthors.value,
    tags: selectedTags.value,
    excludeTags: excludedTags.value,
    metadataFilters: metadataFilters.value,
    sortKey: request.sortKey,
    sortDirection: request.sortDirection,
    limit: request.limit,
    offset: request.offset,
  }),
  onBeforeListReset: () => clearSelection(),
  onItemsLoaded: (response) => {
    pruneSelectionToCurrentBooks()
    void hydrateBookThumbnails(response.books)
  },
  onPageChanged: scrollListToTop,
})
const {
  selectedBookPaths,
  selectedCount,
  allCurrentPageSelected,
  clearSelection,
  toggleBookSelection,
  toggleSelectAllCurrentPage,
  selectedPaths,
  selectedItems,
  pruneSelectionToCurrentBooks,
  snapshotSelection,
  restoreSelection,
} = useBookSelection(books)
const { hydrateBookThumbnails } = useBookThumbnailHydration(books)

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

const savedViewOptions = computed<SelectOption[]>(() => [
  { label: '不使用保存视图', value: 'none' },
  ...savedViews.value.map((view) => ({ label: view.name, value: view.id })),
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
const selectedCollection = computed(() => (
  selectedCollectionId.value
    ? collections.value.find((collection) => collection.id === selectedCollectionId.value) ?? null
    : null
))
const selectedCollectionDescription = computed(() => selectedCollection.value?.description?.trim() || '暂无描述')
const totalFavoriteCount = computed(() => collections.value.reduce((sum, collection) => sum + collection.bookCount, 0))
const targetCollections = computed(() => (
  batchTargetMode.value === 'move' && selectedCollectionId.value
    ? collections.value.filter((collection) => collection.id !== selectedCollectionId.value)
    : collections.value
))
const batchTargetTitle = computed(() => (batchTargetMode.value === 'move' ? '移动到收藏夹' : '加入收藏夹'))
const bookContextMenuOptions = computed(() => [
  { label: '重命名标题', key: 'rename' },
  { label: '恢复默认标题', key: 'reset', disabled: !contextMenuBook.value?.titleOverride },
])

async function loadPage() {
  loading.value = true
  error.value = ''
  try {
    const [nextCollections, nextAuthors, nextTags] = await Promise.all([
      listFavoriteCollections(),
      listBookAuthors(),
      listBookTags(),
      loadLibraryViewSettings(),
    ])
    collections.value = nextCollections
    allAuthors.value = nextAuthors
    allTags.value = nextTags
    await loadBooks()
    markInitialized()
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

async function refreshCollections() {
  collections.value = await listFavoriteCollections()
}

function openBatchTargetModal(mode: 'move' | 'add') {
  if (!selectedCount.value) return
  batchTargetMode.value = mode
  batchTargetModalVisible.value = true
  targetNewCollectionName.value = ''
}

async function selectCollection(collectionId: string | null) {
  selectedCollectionId.value = collectionId
  renameValue.value = collectionId
    ? collections.value.find((collection) => collection.id === collectionId)?.name ?? ''
    : ''
  loading.value = true
  error.value = ''
  try {
    await loadFirstPage()
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

function currentSavedLibraryViewState() {
  return {
    query: query.value,
    collectionId: selectedCollectionId.value,
    authors: selectedAuthors.value,
    selectedTags: selectedTags.value,
    excludeTags: excludedTags.value,
    metadataFilters: metadataFilters.value,
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
  savedViews.value = createSavedLibraryView('favorites', savedViewName.value, currentSavedLibraryViewState())
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
  selectedCollectionId.value = state.collectionId ?? null
  renameValue.value = selectedCollectionId.value
    ? collections.value.find((collection) => collection.id === selectedCollectionId.value)?.name ?? ''
    : ''
  setQueryNow(state.query)
  selectedAuthors.value = state.authors ?? []
  selectedTags.value = state.selectedTags ?? []
  excludedTags.value = state.excludeTags ?? []
  metadataFilters.value = state.metadataFilters ?? []
  sortKey.value = state.sortKey
  sortDirection.value = state.sortDirection
  pageSize.value = state.pageSize
  viewSettings.value = state.viewSettings
  await loadFirstPage()
}

function beginRenameSavedView(view: SavedLibraryView) {
  renameSavedViewId.value = view.id
  renameSavedViewName.value = view.name
}

function submitRenameSavedView() {
  const id = renameSavedViewId.value
  if (!id) return
  savedViews.value = renameSavedLibraryView('favorites', id, renameSavedViewName.value)
  renameSavedViewId.value = null
  renameSavedViewName.value = ''
}

function removeSavedView(id: string) {
  savedViews.value = deleteSavedLibraryView('favorites', id)
  if (selectedSavedViewId.value === id) selectedSavedViewId.value = null
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
  recentFilters.value = saveRecentLibraryFilter('favorites', currentRecentFilterState())
}

async function applyRecentFilter(filter: RecentLibraryFilter) {
  setQueryNow(filter.state.query)
  selectedAuthors.value = filter.state.authors
  selectedTags.value = filter.state.tags
  excludedTags.value = filter.state.excludeTags
  metadataFilters.value = filter.state.metadataFilters
  await loadFirstPage()
}

function openBook(book: BookSummary) {
  router.push(`/reader/${book.id}`)
}

function openBookDetail(book: BookSummary) {
  router.push(`/books/${book.id}`)
}

function selectCardAuthor(author: string) {
  const value = author.trim()
  if (!value) return
  if (!selectedAuthors.value.includes(value)) {
    selectedAuthors.value = [...selectedAuthors.value, value]
  }
}

function selectCardTag(tag: string) {
  const value = tag.trim()
  if (!value) return
  if (!selectedTags.value.includes(value)) {
    selectedTags.value = [...selectedTags.value, value]
  }
}

async function scrollListToTop() {
  await nextTick()
  document.querySelector<HTMLElement>('.main-panel')?.scrollTo({ top: 0, behavior: 'smooth' })
}

function handleBookContextMenuSelect(key: string | number) {
  const book = contextMenuBook.value
  closeBookContextMenu()
  if (!book) return

  if (key === 'rename') {
    openRenameDialog(book)
  } else if (key === 'reset') {
    void resetBookTitleFromMenu(book)
  }
}

function replaceBook(updated: BookSummary) {
  books.value = books.value.map((book) => book.path === updated.path ? { ...book, ...updated } : book)
  if (contextMenuBook.value?.path === updated.path) {
    contextMenuBook.value = { ...contextMenuBook.value, ...updated }
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

function setBatchSuccess(operation: string, items: BatchOperationItem[]) {
  batchResult.value = createBatchSuccessResult(operation, items)
  message.success(batchResultSummary(batchResult.value))
}

function setBatchFailure(operation: string, items: BatchOperationItem[], innerError: unknown) {
  const reason = String(innerError)
  batchResult.value = createBatchFailureResult(operation, items, reason)
  error.value = reason
}

async function removeFavorite(book: BookSummary) {
  error.value = ''

  if (!selectedCollectionId.value) {
    const confirmed = await confirmDialog('移除收藏', `确定要从所有收藏夹移除《${book.title}》吗？`)
    if (!confirmed) return
  }

  const previousBooks = books.value
  const previousTotal = totalBooks.value
  books.value = books.value.filter((item) => item.path !== book.path)
  totalBooks.value = Math.max(0, totalBooks.value - 1)
  try {
    if (selectedCollectionId.value) {
      await removeBooksFromFavoriteCollection([book.path], selectedCollectionId.value)
    } else {
      await removeBooksFromAllFavoriteCollections([book.path])
    }
    await refreshCollections()
    await loadBooks()
    message.success('已移出收藏')
  } catch (innerError) {
    books.value = previousBooks
    totalBooks.value = previousTotal
    error.value = String(innerError)
  }
}

async function removeSelectedFavorites() {
  const paths = selectedPaths()
  const items = selectedItems()
  if (!paths.length) return

  error.value = ''
  batchResult.value = null
  const content = selectedCollectionId.value
    ? batchConfirmationMessage('从收藏夹移出', paths.length, selectedCollection.value?.name ?? '')
    : batchConfirmationMessage('从所有收藏夹移出', paths.length)
  const confirmed = await confirmDialog('批量移出收藏', content)
  if (!confirmed) return

  const previousBooks = books.value
  const previousTotal = totalBooks.value
  const previousSelection = snapshotSelection()
  books.value = books.value.filter((book) => !previousSelection.has(book.path))
  totalBooks.value = Math.max(0, totalBooks.value - paths.length)
  batchActionLoading.value = true
  try {
    if (selectedCollectionId.value) {
      await removeBooksFromFavoriteCollection(paths, selectedCollectionId.value)
    } else {
      await removeBooksFromAllFavoriteCollections(paths)
    }
    await refreshCollections()
    clearSelection()
    await loadBooks()
    setBatchSuccess('已移出', items)
  } catch (innerError) {
    books.value = previousBooks
    totalBooks.value = previousTotal
    restoreSelection(previousSelection)
    setBatchFailure('批量移出', items, innerError)
  } finally {
    batchActionLoading.value = false
  }
}

async function applyTargetCollection(collection: FavoriteCollection) {
  const paths = selectedPaths()
  const items = selectedItems()
  if (!paths.length) return

  const previousBooks = books.value
  const previousSelection = snapshotSelection()
  error.value = ''
  batchResult.value = null
  batchActionLoading.value = true
  targetCollectionLoading.value = true
  try {
    if (batchTargetMode.value === 'move' && selectedCollectionId.value) {
      await moveBooksBetweenFavoriteCollections(paths, selectedCollectionId.value, collection.id)
      setBatchSuccess(`已移动到「${collection.name}」`, items)
    } else {
      await addBooksToFavoriteCollection(paths, collection.id)
      setBatchSuccess(`已加入「${collection.name}」`, items)
    }
    await refreshCollections()
    await loadBooks()
    clearSelection()
    closeBatchTargetModal()
  } catch (innerError) {
    books.value = previousBooks
    restoreSelection(previousSelection)
    setBatchFailure(
      batchTargetMode.value === 'move' ? '批量移动' : '批量加入',
      items,
      innerError,
    )
  } finally {
    batchActionLoading.value = false
    targetCollectionLoading.value = false
  }
}

async function createCollectionFromTargetModal() {
  const name = targetNewCollectionName.value.trim()
  if (!name || batchActionLoading.value) return

  error.value = ''
  targetCollectionLoading.value = true
  try {
    const collection = await createFavoriteCollection(name)
    collections.value = [...collections.value, collection]
    targetNewCollectionName.value = ''
    await applyTargetCollection(collection)
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    targetCollectionLoading.value = false
  }
}

function closeBatchTargetModal() {
  batchTargetModalVisible.value = false
  targetNewCollectionName.value = ''
}

async function createCollection() {
  const name = newCollectionName.value.trim()
  if (!name) return

  error.value = ''
  try {
    const collection = await createFavoriteCollection(name)
    collections.value = [...collections.value, collection]
    newCollectionName.value = ''
    await selectCollection(collection.id)
    message.success(`已新建收藏夹「${collection.name}」`)
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function renameSelectedCollection() {
  if (!selectedCollection.value || selectedCollection.value.isDefault) return
  const name = renameValue.value.trim()
  if (!name) return

  error.value = ''
  try {
    const renamed = await renameFavoriteCollection(selectedCollection.value.id, name)
    collections.value = collections.value.map((collection) => collection.id === renamed.id ? renamed : collection)
    message.success('收藏夹已重命名')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

function openCollectionMetadataModal() {
  if (!selectedCollection.value) return
  metadataCoverPath.value = selectedCollection.value.coverPath ?? ''
  metadataDescription.value = selectedCollection.value.description ?? ''
  metadataModalVisible.value = true
}

async function chooseCollectionCover() {
  const selected = await open({
    multiple: false,
    title: '选择收藏夹封面',
    filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif'] }],
  })
  if (!selected || Array.isArray(selected)) return
  metadataCoverPath.value = selected
}

function clearCollectionCover() {
  metadataCoverPath.value = ''
}

async function submitCollectionMetadata() {
  if (!selectedCollection.value || metadataSubmitting.value) return

  error.value = ''
  metadataSubmitting.value = true
  try {
    const updated = await updateFavoriteCollectionMetadata(
      selectedCollection.value.id,
      metadataCoverPath.value,
      metadataDescription.value,
    )
    collections.value = collections.value.map((collection) => collection.id === updated.id ? updated : collection)
    metadataModalVisible.value = false
    message.success('收藏夹信息已保存')
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    metadataSubmitting.value = false
  }
}

async function deleteSelectedCollection() {
  if (!selectedCollection.value || selectedCollection.value.isDefault) return

  error.value = ''
  try {
    await deleteFavoriteCollection(selectedCollection.value.id)
    selectedCollectionId.value = null
    await refreshCollections()
    await loadFirstPage()
    message.success('收藏夹已删除')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

function collectionCoverUrl(collection: FavoriteCollection): string | undefined {
  return toAssetUrl(collection.coverPath)
}

watch([selectedAuthors, selectedTags, excludedTags, metadataFilters], () => {
  void loadFirstPage()
})

onMounted(loadPage)
</script>

<template>
  <section class="page-section">
    <NPageHeader>
      <template #title>收藏</template>
      <template #subtitle>用多个收藏夹整理你收藏的漫画。</template>
      <template #extra>
        <RouterLink to="/library" custom v-slot="{ navigate }">
          <NButton @click="navigate">返回书架</NButton>
        </RouterLink>
      </template>
    </NPageHeader>

    <NAlert v-if="error" type="error" class="state-block" :show-icon="false">
      {{ error }}
    </NAlert>

    <div class="favorites-layout">
      <NCard class="collection-sidebar" embedded :bordered="false">
        <NSpace vertical>
          <NButton
            block
            :type="selectedCollectionId === null ? 'primary' : 'default'"
            :secondary="selectedCollectionId !== null"
            @click="selectCollection(null)"
          >
            <span class="collection-button-content">
              <span>全部收藏</span>
              <NTag size="small" round>{{ totalFavoriteCount }}</NTag>
            </span>
          </NButton>
          <NButton
            v-for="collection in collections"
            :key="collection.id"
            block
            :type="selectedCollectionId === collection.id ? 'primary' : 'default'"
            :secondary="selectedCollectionId !== collection.id"
            @click="selectCollection(collection.id)"
          >
            <span class="collection-button-content">
              <span class="collection-cover">
                <img v-if="collectionCoverUrl(collection)" :src="collectionCoverUrl(collection)" :alt="collection.name" />
                <span v-else>{{ collection.name.slice(0, 1) }}</span>
              </span>
              <span class="collection-button-text">
                <span>{{ collection.name }}</span>
                <NText v-if="collection.description" depth="3">{{ collection.description }}</NText>
              </span>
              <NTag size="small" round>{{ collection.bookCount }}</NTag>
            </span>
          </NButton>

          <form class="collection-form" @submit.prevent="createCollection">
            <NInput v-model:value="newCollectionName" placeholder="新收藏夹" />
            <NButton type="primary" attr-type="submit">新建</NButton>
          </form>
        </NSpace>
      </NCard>

      <div class="collection-main">
        <NCard :bordered="false" class="toolbar-card">
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

        <NCard v-if="selectedCollection" :bordered="false" class="toolbar-card">
          <NSpace justify="space-between" align="center" :wrap="true">
            <div class="collection-heading">
              <div class="collection-heading-cover">
                <img
                  v-if="collectionCoverUrl(selectedCollection)"
                  :src="collectionCoverUrl(selectedCollection)"
                  :alt="selectedCollection.name"
                />
                <span v-else>{{ selectedCollection.name.slice(0, 1) }}</span>
              </div>
              <div>
                <h2 class="section-title">{{ selectedCollection.name }}</h2>
                <NText depth="3">{{ selectedCollectionDescription }}</NText>
              </div>
            </div>
            <NButton secondary @click="openCollectionMetadataModal">编辑信息</NButton>
          </NSpace>
        </NCard>

        <NCard :bordered="false" class="toolbar-card">
          <NSpace justify="space-between" align="center" :wrap="true">
            <div>
              <h2 class="section-title">{{ selectedCollection?.name ?? '全部收藏' }}</h2>
              <NText depth="3">第 {{ currentPage }} / {{ pageCount }} 页，{{ books.length }} / {{ totalBooks }} 本漫画</NText>
            </div>
            <NSpace>
              <NInput v-model:value="query" clearable placeholder="搜索标题、作者、标签" class="search-input" />
              <NButton @click="showAdvancedFilters = !showAdvancedFilters">
                {{ showAdvancedFilters ? '收起筛选' : '高级筛选' }}
              </NButton>
              <NSelect v-model:value="sortKey" :options="sortKeyOptions" class="sort-select" />
              <NSelect v-model:value="sortDirection" :options="sortDirectionOptions" class="sort-select" />
              <NSelect v-model:value="pageSize" :options="pageSizeOptions" class="sort-select" />
            </NSpace>
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
                :value="selectedTags"
                clearable
                filterable
                multiple
                placeholder="包含标签"
                :options="tagOptions"
                class="tag-select"
                @update:value="updateSelectedTags"
              />
              <NSelect
                :value="excludedTags"
                clearable
                filterable
                multiple
                placeholder="排除标签"
                :options="tagOptions"
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

        <NCard v-if="!loading && totalBooks > pageSize" :bordered="false" class="toolbar-card">
          <NSpace justify="center" align="center">
            <NPagination v-model:page="currentPage" :page-count="pageCount" />
          </NSpace>
        </NCard>

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
                v-if="selectedCollectionId"
                type="primary"
                secondary
                :disabled="!selectedCount || batchActionLoading"
                :loading="batchActionLoading && batchTargetMode === 'move'"
                @click="openBatchTargetModal('move')"
              >
                移动到收藏夹
              </NButton>
              <NButton
                v-else
                type="primary"
                secondary
                :disabled="!selectedCount || batchActionLoading"
                :loading="batchActionLoading && batchTargetMode === 'add'"
                @click="openBatchTargetModal('add')"
              >
                加入收藏夹
              </NButton>
              <NButton
                type="error"
                secondary
                :disabled="!selectedCount || batchActionLoading"
                :loading="batchActionLoading"
                @click="removeSelectedFavorites"
              >
                移出收藏
              </NButton>
            </NSpace>
          </NSpace>
        </NCard>

        <NCard v-if="selectedCollection && !selectedCollection.isDefault" :bordered="false" class="toolbar-card">
          <form class="inline-form" @submit.prevent="renameSelectedCollection">
            <NInput v-model:value="renameValue" placeholder="收藏夹名称" />
            <NButton attr-type="submit">重命名</NButton>
            <NPopconfirm @positive-click="deleteSelectedCollection">
              <template #trigger>
                <NButton type="error" secondary>删除收藏夹</NButton>
              </template>
              确定要删除收藏夹「{{ selectedCollection.name }}」吗？收藏夹内的漫画原文件不会被删除。
            </NPopconfirm>
          </form>
        </NCard>

        <NSpin v-if="loading" class="state-block" description="正在加载收藏..." />

        <template v-else-if="books.length">
          <NSpin :show="pageLoading" description="正在加载当前页...">
            <BookList
              :books="books"
              :settings="viewSettings"
              :highlight-query="debouncedQuery"
              selectable
              :selected-book-paths="selectedBookPaths"
              @open="openBook"
              @detail="openBookDetail"
              @toggle-favorite="removeFavorite"
              @toggle-selection="toggleBookSelection"
              @select-author="selectCardAuthor"
              @select-tag="selectCardTag"
              @book-context-menu="openBookContextMenu"
            />
          </NSpin>
        </template>

        <NEmpty v-else-if="hasFilters" class="state-block" description="没有匹配结果" />

        <NEmpty v-else class="state-block" :description="selectedCollection ? '这个收藏夹还是空的' : '还没有收藏'">
          <template #extra>
            <NSpace vertical align="center">
              <NText depth="3">在书架中点击漫画封面上的星标，即可选择要加入的收藏夹。</NText>
              <RouterLink to="/library" custom v-slot="{ navigate }">
                <NButton type="primary" @click="navigate">去书架看看</NButton>
              </RouterLink>
            </NSpace>
          </template>
        </NEmpty>
      </div>
    </div>

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
      :show="batchTargetModalVisible"
      preset="card"
      :title="batchTargetTitle"
      class="favorite-modal"
      :style="{ width: 'min(520px, calc(100vw - 32px))' }"
      @update:show="(value) => { if (!value) closeBatchTargetModal() }"
    >
      <NSpace vertical size="large">
        <NText depth="3">
          已选择 {{ selectedCount }} 本漫画，{{ batchTargetMode === 'move' ? '请选择要移动到的收藏夹。' : '请选择要加入的收藏夹。' }}{{ sourceFilesSafeNotice }}
        </NText>

        <NSpin v-if="targetCollectionLoading" description="正在处理收藏夹..." />
        <NList v-else bordered hoverable>
          <NListItem v-for="collection in targetCollections" :key="collection.id">
            <NSpace align="center" justify="space-between">
              <NCheckbox @update:checked="() => applyTargetCollection(collection)">
                {{ collection.name }}
              </NCheckbox>
              <NText depth="3">{{ collection.bookCount }} 本</NText>
            </NSpace>
          </NListItem>
        </NList>

        <NEmpty v-if="!targetCollectionLoading && !targetCollections.length" description="没有可选的目标收藏夹" />

        <form class="collection-form" @submit.prevent="createCollectionFromTargetModal">
          <NInput v-model:value="targetNewCollectionName" placeholder="新收藏夹名称" />
          <NButton type="primary" attr-type="submit" :loading="targetCollectionLoading">
            新建并{{ batchTargetMode === 'move' ? '移动' : '加入' }}
          </NButton>
        </form>
      </NSpace>
    </NModal>

    <NModal
      :show="metadataModalVisible"
      preset="card"
      title="编辑收藏夹信息"
      class="favorite-modal"
      :style="{ width: 'min(520px, calc(100vw - 32px))' }"
      @update:show="(value) => { metadataModalVisible = value }"
    >
      <form @submit.prevent="submitCollectionMetadata">
        <NSpace vertical size="large">
          <div class="collection-metadata-preview">
            <div class="collection-heading-cover">
              <img
                v-if="metadataCoverPath"
                :src="toAssetUrl(metadataCoverPath)"
                :alt="selectedCollection?.name ?? '收藏夹封面'"
              />
              <span v-else>{{ selectedCollection?.name.slice(0, 1) }}</span>
            </div>
            <NSpace vertical size="small">
              <NInput v-model:value="metadataCoverPath" placeholder="封面图片路径" />
              <NSpace>
                <NButton type="primary" secondary @click="chooseCollectionCover">选择图片</NButton>
                <NButton @click="clearCollectionCover">清空封面</NButton>
              </NSpace>
            </NSpace>
          </div>
          <NInput
            v-model:value="metadataDescription"
            type="textarea"
            placeholder="收藏夹描述"
            :autosize="{ minRows: 3, maxRows: 6 }"
          />
          <NSpace justify="end">
            <NButton :disabled="metadataSubmitting" @click="metadataModalVisible = false">取消</NButton>
            <NButton type="primary" attr-type="submit" :loading="metadataSubmitting">保存</NButton>
          </NSpace>
        </NSpace>
      </form>
    </NModal>
  </section>
</template>
