import { computed, onBeforeUnmount, ref, watch } from 'vue'
import type { BookListResponse, BookSummary } from '@/api/tauri'
import type { BookSortKey, SortDirection } from '@/utils/bookSort'

type BookListControllerRequest = {
  query: string
  sortKey: BookSortKey
  sortDirection: SortDirection
  limit: number
  offset: number
}

type UseBookListControllerOptions = {
  defaultPageSize: number
  searchDebounceMs: number
  initialQuery?: string
  initialSortKey: BookSortKey
  initialSortDirection: SortDirection
  initialPageSize?: number
  initialCurrentPage?: number
  error: { value: string }
  load: (request: BookListControllerRequest) => Promise<BookListResponse>
  onStateChanged?: () => void
  onBeforeListReset?: () => void
  onItemsLoaded?: (response: BookListResponse) => void
  onPageChanged?: () => void | Promise<void>
}

export function useBookListController(options: UseBookListControllerOptions) {
  const books = ref<BookSummary[]>([])
  const totalBooks = ref(0)
  const pageLoading = ref(false)
  const query = ref(options.initialQuery ?? '')
  const debouncedQuery = ref(options.initialQuery ?? '')
  const sortKey = ref<BookSortKey>(options.initialSortKey)
  const sortDirection = ref<SortDirection>(options.initialSortDirection)
  const currentPage = ref(options.initialCurrentPage ?? 1)
  const pageSize = ref(options.initialPageSize ?? options.defaultPageSize)
  const pageCount = computed(() => Math.max(1, Math.ceil(totalBooks.value / pageSize.value)))
  const hasFilters = computed(() => Boolean(debouncedQuery.value.trim()))

  let initialized = false
  let requestToken = 0
  let searchTimer: number | undefined
  let suppressNextPageWatch = false

  async function loadBooks() {
    const token = ++requestToken
    pageLoading.value = true
    options.error.value = ''
    try {
      const response = await options.load({
        query: debouncedQuery.value,
        sortKey: sortKey.value,
        sortDirection: sortDirection.value,
        limit: pageSize.value,
        offset: (currentPage.value - 1) * pageSize.value,
      })
      if (token !== requestToken) return
      books.value = response.books
      totalBooks.value = response.total
      options.onItemsLoaded?.(response)
      if (currentPage.value > pageCount.value) {
        suppressNextPageWatch = currentPage.value !== pageCount.value
        currentPage.value = pageCount.value
        options.onStateChanged?.()
        await loadBooks()
      }
    } catch (innerError) {
      if (token === requestToken) options.error.value = String(innerError)
    } finally {
      if (token === requestToken) pageLoading.value = false
    }
  }

  function markInitialized() {
    initialized = true
  }

  async function loadFirstPage() {
    options.onBeforeListReset?.()
    if (currentPage.value !== 1) {
      suppressNextPageWatch = true
      currentPage.value = 1
    }
    await loadBooks()
  }

  function setQueryNow(value: string) {
    if (searchTimer) {
      window.clearTimeout(searchTimer)
      searchTimer = undefined
    }
    query.value = value
    debouncedQuery.value = value
    options.onStateChanged?.()
  }

  function startWatchers() {
    // Watchers are created lazily so pages can finish wiring callbacks first.
    const unwatchQuery = watchQuery()
    const unwatchDebouncedQuery = watchDebouncedQuery()
    const unwatchListControls = watchListControls()
    const unwatchCurrentPage = watchCurrentPage()

    onBeforeUnmount(() => {
      if (searchTimer) window.clearTimeout(searchTimer)
      unwatchQuery()
      unwatchDebouncedQuery()
      unwatchListControls()
      unwatchCurrentPage()
    })
  }

  function watchQuery() {
    return watch(query, () => {
      options.onStateChanged?.()
      if (searchTimer) window.clearTimeout(searchTimer)
      searchTimer = window.setTimeout(() => {
        debouncedQuery.value = query.value
      }, options.searchDebounceMs)
    })
  }

  function watchDebouncedQuery() {
    return watch(debouncedQuery, () => {
      if (!initialized) return
      options.onStateChanged?.()
      void loadFirstPage()
    })
  }

  function watchListControls() {
    return watch([sortKey, sortDirection, pageSize], () => {
      options.onStateChanged?.()
      if (!initialized) return
      void loadFirstPage()
    })
  }

  function watchCurrentPage() {
    return watch(currentPage, async () => {
      options.onStateChanged?.()
      if (suppressNextPageWatch) {
        suppressNextPageWatch = false
        return
      }
      if (!initialized) return
      options.onBeforeListReset?.()
      await loadBooks()
      await options.onPageChanged?.()
    })
  }

  startWatchers()

  return {
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
  }
}
