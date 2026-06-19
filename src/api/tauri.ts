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
  publishedAt?: string | null
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

export type BookSortKey = 'title' | 'totalPages' | 'createdAt' | 'lastReadAt' | 'publishedAt'
export type SortDirection = 'asc' | 'desc'
export type ReadingStatus = 'all' | 'unread' | 'reading' | 'read'
export type FavoriteStatus = 'all' | 'favorited' | 'notFavorited'
export type MetadataFilter = 'missingDescription' | 'missingAuthors' | 'missingTags' | 'missingCover' | 'missingPublishedAt'

export type BookListRequest = {
  repositoryId?: string | null
  collectionId?: string | null
  query?: string | null
  author?: string | null
  authors?: string[] | null
  tag?: string | null
  tags?: string[] | null
  excludeTags?: string[] | null
  metadataFilters?: MetadataFilter[] | null
  readingStatus?: ReadingStatus | null
  favoriteStatus?: FavoriteStatus | null
  sortKey?: BookSortKey
  sortDirection?: SortDirection
  limit?: number
  offset?: number
}

export type BookAggregationItem = {
  name: string
  count: number
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
  coverPath?: string | null
  description?: string | null
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
  summary: RepositoryScanSummary
}

export type RepositoryScanHistoryRecord = {
  id: string
  repositoryId: string
  repositoryName: string
  repositoryPath: string
  scannedAt: string
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
  code?: RepositoryScanIssueCode
  severity?: RepositoryScanIssueSeverity
  suggestion?: string
}

export type RepositoryScanIssueCode =
  | 'unchangedBook'
  | 'noImages'
  | 'readFailed'
  | 'duplicateBook'
  | 'unknown'

export type RepositoryScanIssueSeverity = 'info' | 'warning' | 'error'

export type RepositoryDuplicateBook = {
  path: string
  duplicateOf: string
  title: string
}

export type MetadataHealthBookIssue = {
  book: BookSummary
  reasons: string[]
}

export type MetadataHealthScanIssue = {
  repositoryId: string
  repositoryName: string
  repositoryPath: string
  scannedAt: string
  path: string
  reason: string
  code: RepositoryScanIssueCode
  severity: RepositoryScanIssueSeverity
  suggestion?: string
}

export type MetadataHealthDuplicateIssue = {
  repositoryId: string
  repositoryName: string
  repositoryPath: string
  scannedAt: string
  path: string
  duplicateOf: string
  title: string
}

export type MetadataHealthSummary = {
  missingMetadata: MetadataHealthBookIssue[]
  missingCovers: MetadataHealthBookIssue[]
  noImageIssues: MetadataHealthScanIssue[]
  duplicateIssues: MetadataHealthDuplicateIssue[]
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
  autoScrollSpeed: number
  autoScrollStartDelay: number
  autoScrollStopOnManualScroll: boolean
}

export type PerBookReaderSettings = {
  bookId: string
  settings: ReaderSettings
}

export type EffectiveReaderSettingsState = {
  settings: ReaderSettings
  hasBookReaderSettings: boolean
}

export type LibraryViewSettings = {
  layout: 'grid' | 'compact' | 'list'
  coverSize: 'small' | 'medium' | 'large'
  showAuthors: boolean
  showTags: boolean
  tagLimit: number
  titleLineClamp: number
  titleFontSize: number
}

export type SettingsExport = {
  schemaVersion: number
  exportedAt: string
  reader: ReaderSettings
  libraryView: LibraryViewSettings
  perBookReaderSettings: PerBookReaderSettings[]
}

export type SettingsRestoreScope = 'all' | 'reader' | 'libraryView'

export type CacheMaintenanceSummary = {
  thumbnailCacheDir: string
  thumbnailFiles: number
  thumbnailBytes: number
  booksWithThumbnails: number
  missingThumbnails: number
}

export type CacheMaintenanceFailure = {
  path: string
  title?: string | null
  reason: string
}

export type CacheMaintenanceResult = {
  operation: string
  total: number
  succeeded: number
  failed: CacheMaintenanceFailure[]
  removedFiles: number
  removedBytes: number
  rebuiltThumbnails: number
  sourceFilesAffected: false
}

export type DatabaseBackupResult = {
  backupPath: string
  createdAt: string
  bytes: number
  sourceFilesAffected: false
}

export type DatabaseRestoreResult = {
  restoredFrom: string
  restoredAt: string
  rollbackPath: string
  sourceFilesAffected: false
}

export type OperationLogRecord = {
  id: string
  operationType: string
  target: string
  summary: string
  reversible: boolean
  createdAt: string
}

export function toAssetUrl(path?: string | null): string | undefined {
  if (!path) return undefined
  return convertFileSrc(path)
}

export function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args)
}
