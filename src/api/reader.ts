import { call, type Page, type ReadingHistoryRecord } from './tauri'

export function listChapterPages(chapterId: string): Promise<Page[]> {
  return call('list_chapter_pages', { chapterId })
}

export function updateBookProgress(bookId: string, chapterId: string, page: number): Promise<void> {
  return call('update_book_progress', { bookId, chapterId, page })
}

export function listReadingHistory(): Promise<ReadingHistoryRecord[]> {
  return call('list_reading_history')
}
