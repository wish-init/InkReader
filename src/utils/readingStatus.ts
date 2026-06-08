import type { Book, BookSummary } from '@/api/tauri'

export type DerivedReadingStatus = 'unread' | 'reading' | 'read'

export function getReadingStatus(book: Pick<BookSummary, 'lastReadAt' | 'lastPage' | 'lastChapterId' | 'isReadComplete'> | Book): DerivedReadingStatus {
  const hasProgress = Boolean(book.lastReadAt || book.lastPage > 0)
  if (!hasProgress) return 'unread'
  if (book.isReadComplete) return 'read'
  if ('chapters' in book) {
    const lastChapter = book.chapters.at(-1)
    if (
      lastChapter
        && book.lastChapterId === lastChapter.id
        && book.lastPage + 1 >= lastChapter.pageCount
    ) {
      return 'read'
    }
    return 'reading'
  }
  return 'reading'
}

export function getReadingStatusLabel(status: DerivedReadingStatus): string {
  if (status === 'read') return '已读完'
  if (status === 'reading') return '阅读中'
  return '未阅读'
}
