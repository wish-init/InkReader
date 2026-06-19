import { computed, ref } from 'vue'
import { toArchiveUrl } from '@/api/archive'
import { getBook } from '@/api/library'
import { listChapterPages } from '@/api/reader'
import { toAssetUrl, type Book, type Chapter, type Page } from '@/api/tauri'

export function useReaderBook(bookId: string) {
  const book = ref<Book | null>(null)
  const chapters = ref<Chapter[]>([])
  const pages = ref<Page[]>([])
  const chapterIndex = ref(0)
  const pageIndex = ref(0)

  const currentChapter = computed(() => chapters.value[chapterIndex.value])
  const currentPage = computed(() => pages.value[pageIndex.value])
  const isArchiveBook = computed(() => book.value ? book.value.kind !== 'folder' : false)

  async function loadBookMetadata(requestedChapterValue: unknown) {
    const loadedBook = await getBook(bookId)
    book.value = loadedBook
    chapters.value = loadedBook.chapters

    const requestedChapterIndex = Number(requestedChapterValue)
    const savedChapterIndex = Number.isInteger(requestedChapterIndex) && requestedChapterIndex >= 0
      ? requestedChapterIndex
      : loadedBook.lastChapterId
      ? chapters.value.findIndex((chapter) => chapter.id === loadedBook.lastChapterId)
      : 0

    chapterIndex.value = Math.min(savedChapterIndex >= 0 ? savedChapterIndex : 0, Math.max(chapters.value.length - 1, 0))
    pageIndex.value = Number.isInteger(requestedChapterIndex) && requestedChapterIndex >= 0
      ? 0
      : Math.max(loadedBook.lastPage || 0, 0)
  }

  async function loadPagesForCurrentChapter() {
    const chapter = currentChapter.value
    if (!chapter) {
      pages.value = []
      return
    }
    pages.value = await listChapterPages(chapter.id)
    pageIndex.value = Math.min(pageIndex.value, Math.max(pages.value.length - 1, 0))
  }

  function getPageImageUrl(page: Page): string | undefined {
    if (isArchiveBook.value && book.value) {
      return toArchiveUrl(book.value.path, page.uri)
    }
    if (page.path !== page.uri) {
      return toArchiveUrl(page.path, page.uri)
    }
    return toAssetUrl(page.path)
  }

  return {
    book,
    chapters,
    pages,
    chapterIndex,
    pageIndex,
    currentChapter,
    currentPage,
    isArchiveBook,
    loadBookMetadata,
    loadPagesForCurrentChapter,
    getPageImageUrl,
  }
}
