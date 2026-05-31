<script setup lang="ts">
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
  listFavoriteBooks,
  listFavoriteCollections,
  moveBooksBetweenFavoriteCollections,
  removeBooksFromAllFavoriteCollections,
  removeBooksFromFavoriteCollection,
  renameBookTitle,
  renameFavoriteCollection,
  resetBookTitle,
} from '@/api/library'
import { getLibraryViewSettings } from '@/api/settings'
import BookList from '@/components/library/BookList.vue'
import type { BookSummary, FavoriteCollection, LibraryViewSettings } from '@/api/tauri'
import type { BookSortKey, SortDirection } from '@/utils/bookSort'

const defaultViewSettings: LibraryViewSettings = {
  layout: 'grid',
  coverSize: 'medium',
  showAuthors: true,
  showTags: true,
  tagLimit: 4,
}

const defaultPageSize = 80
const pageSizeOptions: SelectOption[] = [
  { label: '每页 40 本', value: 40 },
  { label: '每页 80 本', value: 80 },
  { label: '每页 120 本', value: 120 },
  { label: '每页 200 本', value: 200 },
]

const router = useRouter()
const dialog = useDialog()
const message = useMessage()
const books = ref<BookSummary[]>([])
const totalBooks = ref(0)
const collections = ref<FavoriteCollection[]>([])
const selectedCollectionId = ref<string | null>(null)
const loading = ref(true)
const pageLoading = ref(false)
const error = ref('')
const viewSettings = ref<LibraryViewSettings>({ ...defaultViewSettings })
const newCollectionName = ref('')
const renameValue = ref('')
const sortKey = ref<BookSortKey>('createdAt')
const sortDirection = ref<SortDirection>('desc')
const currentPage = ref(1)
const pageSize = ref(defaultPageSize)
const selectedBookPaths = ref<Set<string>>(new Set())
const batchTargetModalVisible = ref(false)
const batchTargetMode = ref<'move' | 'add'>('move')
const batchActionLoading = ref(false)
const targetCollectionLoading = ref(false)
const targetNewCollectionName = ref('')
const contextMenuBook = ref<BookSummary | null>(null)
const contextMenuVisible = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const renameDialogBook = ref<BookSummary | null>(null)
const renameTitleValue = ref('')
const renameSubmitting = ref(false)
let initialized = false
let requestToken = 0

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

const selectedCollection = computed(() => (
  selectedCollectionId.value
    ? collections.value.find((collection) => collection.id === selectedCollectionId.value) ?? null
    : null
))
const totalFavoriteCount = computed(() => collections.value.reduce((sum, collection) => sum + collection.bookCount, 0))
const selectedBooks = computed(() => books.value.filter((book) => selectedBookPaths.value.has(book.path)))
const selectedCount = computed(() => selectedBooks.value.length)
const allCurrentPageSelected = computed(() => (
  books.value.length > 0 && books.value.every((book) => selectedBookPaths.value.has(book.path))
))
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
const pageCount = computed(() => Math.max(1, Math.ceil(totalBooks.value / pageSize.value)))

async function loadPage() {
  loading.value = true
  error.value = ''
  try {
    const [nextCollections, nextSettings] = await Promise.all([
      listFavoriteCollections(),
      getLibraryViewSettings(),
    ])
    collections.value = nextCollections
    viewSettings.value = { ...defaultViewSettings, ...nextSettings }
    await loadBooks()
    initialized = true
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
}

async function loadBooks() {
  const token = ++requestToken
  pageLoading.value = true
  error.value = ''
  try {
    const response = await listFavoriteBooks({
      collectionId: selectedCollectionId.value,
      sortKey: sortKey.value,
      sortDirection: sortDirection.value,
      limit: pageSize.value,
      offset: (currentPage.value - 1) * pageSize.value,
    })
    if (token !== requestToken) return
    books.value = response.books
    totalBooks.value = response.total
    selectedBookPaths.value = new Set([...selectedBookPaths.value].filter((path) => books.value.some((book) => book.path === path)))
    if (currentPage.value > pageCount.value) {
      currentPage.value = pageCount.value
      await loadBooks()
    }
  } catch (innerError) {
    if (token === requestToken) error.value = String(innerError)
  } finally {
    if (token === requestToken) pageLoading.value = false
  }
}

async function refreshCollections() {
  collections.value = await listFavoriteCollections()
}

function clearSelection() {
  selectedBookPaths.value = new Set()
}

function toggleBookSelection(book: BookSummary) {
  const nextPaths = new Set(selectedBookPaths.value)
  if (nextPaths.has(book.path)) {
    nextPaths.delete(book.path)
  } else {
    nextPaths.add(book.path)
  }
  selectedBookPaths.value = nextPaths
}

function toggleSelectAllCurrentPage() {
  if (allCurrentPageSelected.value) {
    clearSelection()
    return
  }

  selectedBookPaths.value = new Set(books.value.map((book) => book.path))
}

function selectedPaths() {
  return selectedBooks.value.map((book) => book.path)
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
  currentPage.value = 1
  clearSelection()
  loading.value = true
  error.value = ''
  try {
    await loadBooks()
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    loading.value = false
  }
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
  if (!paths.length) return

  error.value = ''
  const content = selectedCollectionId.value
    ? `确定要从收藏夹「${selectedCollection.value?.name ?? ''}」移出当前页选中的 ${paths.length} 本漫画吗？原文件不会被删除。`
    : `确定要从所有收藏夹移出当前页选中的 ${paths.length} 本漫画吗？原文件不会被删除。`
  const confirmed = await confirmDialog('批量移出收藏', content)
  if (!confirmed) return

  const previousBooks = books.value
  const previousTotal = totalBooks.value
  const previousSelection = new Set(selectedBookPaths.value)
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
    message.success(`已移出 ${paths.length} 本漫画`)
  } catch (innerError) {
    books.value = previousBooks
    totalBooks.value = previousTotal
    selectedBookPaths.value = previousSelection
    error.value = String(innerError)
  } finally {
    batchActionLoading.value = false
  }
}

async function applyTargetCollection(collection: FavoriteCollection) {
  const paths = selectedPaths()
  if (!paths.length) return

  const previousBooks = books.value
  const previousSelection = new Set(selectedBookPaths.value)
  error.value = ''
  batchActionLoading.value = true
  targetCollectionLoading.value = true
  try {
    if (batchTargetMode.value === 'move' && selectedCollectionId.value) {
      await moveBooksBetweenFavoriteCollections(paths, selectedCollectionId.value, collection.id)
      message.success(`已移动 ${paths.length} 本漫画到「${collection.name}」`)
    } else {
      await addBooksToFavoriteCollection(paths, collection.id)
      message.success(`已加入 ${paths.length} 本漫画到「${collection.name}」`)
    }
    await refreshCollections()
    await loadBooks()
    clearSelection()
    closeBatchTargetModal()
  } catch (innerError) {
    books.value = previousBooks
    selectedBookPaths.value = previousSelection
    error.value = String(innerError)
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

async function deleteSelectedCollection() {
  if (!selectedCollection.value || selectedCollection.value.isDefault) return

  error.value = ''
  try {
    await deleteFavoriteCollection(selectedCollection.value.id)
    selectedCollectionId.value = null
    currentPage.value = 1
    await refreshCollections()
    await loadBooks()
    message.success('收藏夹已删除')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

watch([sortKey, sortDirection, pageSize], () => {
  if (!initialized) return
  currentPage.value = 1
  clearSelection()
  void loadBooks()
})

watch(currentPage, async () => {
  if (!initialized) return
  clearSelection()
  await loadBooks()
  await scrollListToTop()
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
              <span>{{ collection.name }}</span>
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
          <NSpace justify="space-between" align="center" :wrap="true">
            <div>
              <h2 class="section-title">{{ selectedCollection?.name ?? '全部收藏' }}</h2>
              <NText depth="3">第 {{ currentPage }} / {{ pageCount }} 页，{{ books.length }} / {{ totalBooks }} 本漫画</NText>
            </div>
            <NSpace>
              <NSelect v-model:value="sortKey" :options="sortKeyOptions" class="sort-select" />
              <NSelect v-model:value="sortDirection" :options="sortDirectionOptions" class="sort-select" />
              <NSelect v-model:value="pageSize" :options="pageSizeOptions" class="sort-select" />
            </NSpace>
          </NSpace>
        </NCard>

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
              selectable
              :selected-book-paths="selectedBookPaths"
            @open="openBook"
            @detail="openBookDetail"
            @toggle-favorite="removeFavorite"
              @toggle-selection="toggleBookSelection"
              @book-context-menu="openBookContextMenu"
            />
          </NSpin>
        </template>

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
      :show="batchTargetModalVisible"
      preset="card"
      :title="batchTargetTitle"
      class="favorite-modal"
      :style="{ width: 'min(520px, calc(100vw - 32px))' }"
      @update:show="(value) => { if (!value) closeBatchTargetModal() }"
    >
      <NSpace vertical size="large">
        <NText depth="3">
          已选择 {{ selectedCount }} 本漫画，{{ batchTargetMode === 'move' ? '请选择要移动到的收藏夹。' : '请选择要加入的收藏夹。' }}
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
  </section>
</template>
