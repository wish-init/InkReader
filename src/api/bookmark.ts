import { call } from './tauri'

export type Bookmark = {
  id: string
  bookId: string
  chapterId: string
  pageIndex: number
  title: string
  note?: string | null
  createdAt: string
  updatedAt: string
}

export type CreateBookmarkRequest = {
  bookId: string
  chapterId: string
  pageIndex: number
  title?: string | null
  note?: string | null
}

export function listBookmarks(bookId: string): Promise<Bookmark[]> {
  return call('list_bookmarks', { bookId })
}

export function createBookmark(request: CreateBookmarkRequest): Promise<Bookmark> {
  return call('create_bookmark', { request })
}

export function deleteBookmark(bookmarkId: string): Promise<void> {
  return call('delete_bookmark', { bookmarkId })
}

export function isPageBookmarked(bookId: string, chapterId: string, pageIndex: number): Promise<boolean> {
  return call('is_page_bookmarked', { bookId, chapterId, pageIndex })
}
