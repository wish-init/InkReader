import { call, type Book, type Page, type ReadingHistoryRecord } from './tauri'

export function listChapterPages(chapterId: string): Promise<Page[]> {
  return call('list_chapter_pages', { chapterId })
}

export function updateBookProgress(bookId: string, chapterId: string, page: number): Promise<void> {
  return call('update_book_progress', { bookId, chapterId, page })
}

export function markBookRead(bookId: string): Promise<Book> {
  return call('mark_book_read', { bookId })
}

export function markBookUnread(bookId: string): Promise<Book> {
  return call('mark_book_unread', { bookId })
}

export function listReadingHistory(): Promise<ReadingHistoryRecord[]> {
  return call('list_reading_history')
}
