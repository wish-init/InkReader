import { invoke } from '@tauri-apps/api/core'
import { convertFileSrc } from '@tauri-apps/api/core'

export type Repository = {
  id: string
  name: string
  path: string
  bookCount: number
  lastScannedAt?: string
  createdAt: string
  updatedAt: string
}

export type Page = {
  index: number
  name: string
  path: string
  uri: string
}

export type Chapter = {
  id: string
  bookId: string
  sourceChapterId?: string
  title: string
  path: string
  order: number
  pageCount: number
  pages: Page[]
}

export type BookSummary = {
  id: string
  repositoryId: string
  sourceId?: string
  title: string
  scannedTitle: string
  titleOverride?: string | null
  path: string
  kind: 'folder' | 'zip' | 'cbz' | 'rar' | 'cbr'
  metadataPath?: string
  coverPath?: string
  thumbnailPath?: string | null
  description?: string
  authors: string[]
  tags: string[]
  chapterCount: number
  totalPages: number
  lastChapterId?: string
  lastPage: number
  lastReadAt?: string
  isReadComplete: boolean
  isFavorite: boolean
  createdAt: string
  updatedAt: string
}

export type BookSortKey = 'title' | 'totalPages' | 'createdAt' | 'lastReadAt'
export type SortDirection = 'asc' | 'desc'
export type ReadingStatus = 'all' | 'unread' | 'reading' | 'read'
export type FavoriteStatus = 'all' | 'favorited' | 'notFavorited'

export type BookListRequest = {
  repositoryId?: string | null
  collectionId?: string | null
  query?: string | null
  tag?: string | null
  tags?: string[] | null
  readingStatus?: ReadingStatus | null
  favoriteStatus?: FavoriteStatus | null
  sortKey?: BookSortKey
  sortDirection?: SortDirection
  limit?: number
  offset?: number
}

export type BookListResponse = {
  books: BookSummary[]
  total: number
}

export type UpdateBookMetadataRequest = {
  bookPath: string
  title: string
  description?: string | null
  authors: string[]
  tags: string[]
}

export type BookThumbnail = {
  bookId: string
  thumbnailPath?: string | null
}

export type Book = BookSummary & {
  chapters: Chapter[]
}

export type FavoriteCollection = {
  id: string
  name: string
  bookCount: number
  isDefault: boolean
  createdAt: string
  updatedAt: string
}

export type ReadingHistoryRecord = {
  id: string
  bookId: string
  bookTitle: string
  bookPath: string
  bookKind: string
  coverPath?: string
  chapterId?: string
  chapterTitle?: string
  page: number
  readAt: string
}

export type RepositoryScanResult = {
  repository: Repository
  books: Book[]
  summary: RepositoryScanSummary
}

export type RepositoryScanSummary = {
  totalEntries: number
  scannedBooks: number
  unchangedBooks: number
  skippedEntries: RepositoryScanIssue[]
  failedEntries: RepositoryScanIssue[]
  duplicateBooks: RepositoryDuplicateBook[]
}

export type RepositoryScanIssue = {
  path: string
  reason: string
}

export type RepositoryDuplicateBook = {
  path: string
  duplicateOf: string
  title: string
}

export type RepositoryScanProgress = {
  scanId: string
  repositoryPath: string
  current: number
  total: number
  phase: string
  message: string
}

export type ReaderSettings = {
  mode: 'single' | 'double' | 'scroll'
  fit: 'width' | 'height' | 'original'
  direction: 'ltr' | 'rtl'
  background: string
  spaceScrollRatio: number
  spaceHoldSpeedRatio: number
  brightness: number
  contrast: number
  pageAnimation: 'none' | 'slide' | 'fade'
  preloadCacheLimit: number
}

export type LibraryViewSettings = {
  layout: 'grid' | 'compact' | 'list'
  coverSize: 'small' | 'medium' | 'large'
  showAuthors: boolean
  showTags: boolean
  tagLimit: number
}
export function toAssetUrl(path?: string | null): string | undefined {
  if (!path) return undefined
  return convertFileSrc(path)
}

export function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args)
}
