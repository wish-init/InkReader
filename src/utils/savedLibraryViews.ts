import type {
  FavoriteStatus,
  LibraryViewSettings,
  MetadataFilter,
  ReadingStatus,
} from '@/api/tauri'
import type { BookSortKey, SortDirection } from '@/utils/bookSort'
import { normalizeLibraryViewSettings } from '@/utils/libraryViewSettings'

const savedLibraryViewsKey = 'inkreader:saved-library-views'

export type SavedLibraryViewScope = 'library' | 'favorites'

export type SavedLibraryViewState = {
  query: string
  authors?: string[]
  sortKey: BookSortKey
  sortDirection: SortDirection
  pageSize: number
  viewSettings: LibraryViewSettings
  repositoryId?: string | null
  selectedTags?: string[]
  excludeTags?: string[]
  metadataFilters?: MetadataFilter[]
  readingStatus?: ReadingStatus
  favoriteStatus?: FavoriteStatus
  collectionId?: string | null
}

export type SavedLibraryView = {
  id: string
  name: string
  scope: SavedLibraryViewScope
  state: SavedLibraryViewState
  createdAt: string
  updatedAt: string
}

export function loadSavedLibraryViews(scope: SavedLibraryViewScope): SavedLibraryView[] {
  return loadAllSavedLibraryViews().filter((view) => view.scope === scope)
}

export function createSavedLibraryView(
  scope: SavedLibraryViewScope,
  name: string,
  state: SavedLibraryViewState,
): SavedLibraryView[] {
  const views = loadAllSavedLibraryViews()
  const timestamp = new Date().toISOString()
  const nextView: SavedLibraryView = {
    id: crypto.randomUUID(),
    name: normalizeViewName(name),
    scope,
    state: normalizeSavedLibraryViewState(state),
    createdAt: timestamp,
    updatedAt: timestamp,
  }
  saveAllSavedLibraryViews([...views, nextView])
  return loadSavedLibraryViews(scope)
}

export function renameSavedLibraryView(
  scope: SavedLibraryViewScope,
  id: string,
  name: string,
): SavedLibraryView[] {
  const views = loadAllSavedLibraryViews()
  const nextViews = views.map((view) => view.id === id
    ? { ...view, name: normalizeViewName(name), updatedAt: new Date().toISOString() }
    : view)
  saveAllSavedLibraryViews(nextViews)
  return loadSavedLibraryViews(scope)
}

export function deleteSavedLibraryView(scope: SavedLibraryViewScope, id: string): SavedLibraryView[] {
  saveAllSavedLibraryViews(loadAllSavedLibraryViews().filter((view) => view.id !== id))
  return loadSavedLibraryViews(scope)
}

function loadAllSavedLibraryViews(): SavedLibraryView[] {
  try {
    const rawValue = window.localStorage.getItem(savedLibraryViewsKey)
    if (!rawValue) return []
    const parsed = JSON.parse(rawValue) as Partial<SavedLibraryView>[]
    if (!Array.isArray(parsed)) return []
    return parsed.flatMap(normalizeSavedLibraryView)
  } catch {
    return []
  }
}

function saveAllSavedLibraryViews(views: SavedLibraryView[]) {
  window.localStorage.setItem(savedLibraryViewsKey, JSON.stringify(views))
}

function normalizeSavedLibraryView(value: Partial<SavedLibraryView>): SavedLibraryView[] {
  if (!value || typeof value.id !== 'string' || typeof value.name !== 'string') return []
  if (value.scope !== 'library' && value.scope !== 'favorites') return []
  if (!value.state) return []

  return [{
    id: value.id,
    name: normalizeViewName(value.name),
    scope: value.scope,
    state: normalizeSavedLibraryViewState(value.state as Partial<SavedLibraryViewState>),
    createdAt: typeof value.createdAt === 'string' ? value.createdAt : '',
    updatedAt: typeof value.updatedAt === 'string' ? value.updatedAt : '',
  }]
}

function normalizeSavedLibraryViewState(value: Partial<SavedLibraryViewState>): SavedLibraryViewState {
  return {
    query: typeof value.query === 'string' ? value.query : '',
    authors: normalizeStringArray(value.authors),
    sortKey: isBookSortKey(value.sortKey) ? value.sortKey : 'createdAt',
    sortDirection: isSortDirection(value.sortDirection) ? value.sortDirection : 'desc',
    pageSize: isPageSize(value.pageSize) ? value.pageSize : 80,
    viewSettings: normalizeLibraryViewSettings(value.viewSettings),
    repositoryId: typeof value.repositoryId === 'string' && value.repositoryId ? value.repositoryId : null,
    selectedTags: Array.isArray(value.selectedTags)
      ? value.selectedTags.filter((tag): tag is string => typeof tag === 'string')
      : [],
    excludeTags: normalizeStringArray(value.excludeTags),
    metadataFilters: normalizeMetadataFilters(value.metadataFilters),
    readingStatus: isReadingStatus(value.readingStatus) ? value.readingStatus : 'all',
    favoriteStatus: isFavoriteStatus(value.favoriteStatus) ? value.favoriteStatus : 'all',
    collectionId: typeof value.collectionId === 'string' && value.collectionId ? value.collectionId : null,
  }
}

function normalizeStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return []
  return Array.from(new Set(value.filter((item): item is string => typeof item === 'string').map((item) => item.trim()).filter(Boolean)))
}

function normalizeMetadataFilters(value: unknown): MetadataFilter[] {
  if (!Array.isArray(value)) return []
  return value.filter((item): item is MetadataFilter => (
    item === 'missingDescription'
      || item === 'missingAuthors'
      || item === 'missingTags'
      || item === 'missingCover'
      || item === 'missingPublishedAt'
  ))
}

function normalizeViewName(value: string) {
  return value.trim() || 'Saved view'
}

function isBookSortKey(value: unknown): value is BookSortKey {
  return value === 'title'
    || value === 'totalPages'
    || value === 'createdAt'
    || value === 'lastReadAt'
    || value === 'publishedAt'
}

function isSortDirection(value: unknown): value is SortDirection {
  return value === 'asc' || value === 'desc'
}

function isReadingStatus(value: unknown): value is ReadingStatus {
  return value === 'all' || value === 'unread' || value === 'reading' || value === 'read'
}

function isFavoriteStatus(value: unknown): value is FavoriteStatus {
  return value === 'all' || value === 'favorited' || value === 'notFavorited'
}

function isPageSize(value: unknown): value is number {
  return value === 40 || value === 80 || value === 120 || value === 200
}
