import type { Ref } from 'vue'
import { ensureBookThumbnails } from '@/api/library'
import type { BookSummary } from '@/api/tauri'

export function useBookThumbnailHydration(books: Ref<BookSummary[]>) {
  let thumbnailRequestToken = 0

  async function hydrateBookThumbnails(sourceBooks: BookSummary[]) {
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

  return {
    hydrateBookThumbnails,
  }
}
