import type { Book, BookSummary } from '@/api/tauri'
import { getReadingStatus } from '@/utils/readingStatus'

export function getReadingProgressPercent(book: BookSummary | Book): number {
  const status = getReadingStatus(book)
  if (status === 'unread') return 0
  if (status === 'read') return 100

  if ('chapters' in book && book.chapters.length > 0 && book.lastChapterId) {
    const chapterIndex = book.chapters.findIndex((chapter) => chapter.id === book.lastChapterId)
    if (chapterIndex >= 0) {
      const completedChapters = chapterIndex
      const chapterProgress = getChapterProgress(book.lastPage, book.chapters[chapterIndex].pageCount)
      return clampProgress(Math.round(((completedChapters + chapterProgress) / book.chapters.length) * 100))
    }
  }

  if (book.chapterCount > 1) return 1
  if (book.totalPages <= 0) return 0
  return clampProgress(Math.round(((Math.max(book.lastPage, 0) + 1) / book.totalPages) * 100))
}

function getChapterProgress(lastPage: number, pageCount: number): number {
  if (pageCount <= 0) return 0
  return Math.min(1, Math.max(0, (lastPage + 1) / pageCount))
}

function clampProgress(value: number): number {
  return Math.min(99, Math.max(1, value))
}
