import { computed, ref, type Ref } from 'vue'
import type { BookSummary } from '@/api/tauri'

export function useBookSelection(books: Ref<BookSummary[]>) {
  const selectedBookPaths = ref<Set<string>>(new Set())

  const selectedBooks = computed(() => books.value.filter((book) => selectedBookPaths.value.has(book.path)))
  const selectedCount = computed(() => selectedBooks.value.length)
  const allCurrentPageSelected = computed(() => (
    books.value.length > 0 && books.value.every((book) => selectedBookPaths.value.has(book.path))
  ))

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

  function selectedItems() {
    return selectedBooks.value.map((book) => ({
      path: book.path,
      title: book.title,
    }))
  }

  function pruneSelectionToCurrentBooks() {
    selectedBookPaths.value = new Set(
      [...selectedBookPaths.value].filter((path) => books.value.some((book) => book.path === path)),
    )
  }

  function snapshotSelection() {
    return new Set(selectedBookPaths.value)
  }

  function restoreSelection(paths: Set<string>) {
    selectedBookPaths.value = new Set(paths)
  }

  return {
    selectedBookPaths,
    selectedBooks,
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
  }
}
