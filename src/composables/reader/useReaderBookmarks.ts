import { computed, ref, type Ref } from 'vue'
import { createBookmark, deleteBookmark, listBookmarks, type Bookmark } from '@/api/bookmark'
import type { Book, Chapter } from '@/api/tauri'

type UseReaderBookmarksOptions = {
  book: Ref<Book | null>
  currentChapter: Ref<Chapter | undefined>
  pageIndex: Ref<number>
  jumpToBookmarkPage: (bookmark: Bookmark) => Promise<void>
}

export function useReaderBookmarks(options: UseReaderBookmarksOptions) {
  const bookmarks = ref<Bookmark[]>([])
  const showBookmarkDrawer = ref(false)
  const bookmarkedPageKeys = computed(() => {
    const keys = new Set<string>()
    for (const bookmark of bookmarks.value) {
      keys.add(`${bookmark.chapterId}:${bookmark.pageIndex}`)
    }
    return keys
  })
  const isCurrentPageBookmarked = computed(() => {
    if (!options.currentChapter.value) return false
    return bookmarkedPageKeys.value.has(`${options.currentChapter.value.id}:${options.pageIndex.value}`)
  })

  async function loadBookmarks() {
    if (!options.book.value) return
    try {
      bookmarks.value = await listBookmarks(options.book.value.id)
    } catch { /* ignore */ }
  }

  async function toggleBookmark() {
    if (!options.book.value || !options.currentChapter.value) return

    const existing = bookmarks.value.find(
      (bookmark) => bookmark.chapterId === options.currentChapter.value!.id
        && bookmark.pageIndex === options.pageIndex.value,
    )

    if (existing) {
      try {
        await deleteBookmark(existing.id)
        bookmarks.value = bookmarks.value.filter((bookmark) => bookmark.id !== existing.id)
      } catch { /* ignore */ }
      return
    }

    try {
      const title = `${options.currentChapter.value.title} · 第${options.pageIndex.value + 1}页`
      const created = await createBookmark({
        bookId: options.book.value.id,
        chapterId: options.currentChapter.value.id,
        pageIndex: options.pageIndex.value,
        title,
      })
      bookmarks.value = [created, ...bookmarks.value]
    } catch { /* ignore */ }
  }

  async function jumpToBookmark(bookmark: Bookmark) {
    await options.jumpToBookmarkPage(bookmark)
    showBookmarkDrawer.value = false
  }

  async function removeBookmark(bookmark: Bookmark) {
    try {
      await deleteBookmark(bookmark.id)
      bookmarks.value = bookmarks.value.filter((item) => item.id !== bookmark.id)
    } catch { /* ignore */ }
  }

  return {
    bookmarks,
    showBookmarkDrawer,
    bookmarkedPageKeys,
    isCurrentPageBookmarked,
    loadBookmarks,
    toggleBookmark,
    jumpToBookmark,
    removeBookmark,
  }
}
