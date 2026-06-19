import type { MetadataFilter } from '@/api/tauri'

const recentLibraryFiltersKey = 'inkreader:recent-library-filters'
const maxRecentFilters = 8

export type RecentLibraryFilterScope = 'library' | 'favorites'

export type RecentLibraryFilterState = {
  query: string
  authors: string[]
  tags: string[]
  excludeTags: string[]
  metadataFilters: MetadataFilter[]
}

export type RecentLibraryFilter = {
  id: string
  scope: RecentLibraryFilterScope
  label: string
  state: RecentLibraryFilterState
  updatedAt: string
}

export function loadRecentLibraryFilters(scope: RecentLibraryFilterScope): RecentLibraryFilter[] {
  return loadAllRecentLibraryFilters().filter((filter) => filter.scope === scope)
}

export function saveRecentLibraryFilter(
  scope: RecentLibraryFilterScope,
  state: RecentLibraryFilterState,
): RecentLibraryFilter[] {
  const normalizedState = normalizeRecentLibraryFilterState(state)
  if (!hasRecentFilterValue(normalizedState)) return loadRecentLibraryFilters(scope)

  const existing = loadAllRecentLibraryFilters().filter((filter) => filter.scope !== scope)
  const scoped = loadRecentLibraryFilters(scope).filter(
    (filter) => filterSignature(filter.state) !== filterSignature(normalizedState),
  )
  const now = new Date().toISOString()
  const next: RecentLibraryFilter = {
    id: crypto.randomUUID(),
    scope,
    label: recentFilterLabel(normalizedState),
    state: normalizedState,
    updatedAt: now,
  }
  saveAllRecentLibraryFilters([...existing, next, ...scoped].slice(0, existing.length + maxRecentFilters))
  return loadRecentLibraryFilters(scope)
}

export function normalizeRecentLibraryFilterState(value: Partial<RecentLibraryFilterState>): RecentLibraryFilterState {
  return {
    query: typeof value.query === 'string' ? value.query : '',
    authors: normalizeStringArray(value.authors),
    tags: normalizeStringArray(value.tags),
    excludeTags: normalizeStringArray(value.excludeTags),
    metadataFilters: normalizeMetadataFilters(value.metadataFilters),
  }
}

function loadAllRecentLibraryFilters(): RecentLibraryFilter[] {
  try {
    const rawValue = window.localStorage.getItem(recentLibraryFiltersKey)
    if (!rawValue) return []
    const parsed = JSON.parse(rawValue) as Partial<RecentLibraryFilter>[]
    if (!Array.isArray(parsed)) return []
    return parsed.flatMap(normalizeRecentLibraryFilter)
  } catch {
    return []
  }
}

function saveAllRecentLibraryFilters(filters: RecentLibraryFilter[]) {
  window.localStorage.setItem(recentLibraryFiltersKey, JSON.stringify(filters))
}

function normalizeRecentLibraryFilter(value: Partial<RecentLibraryFilter>): RecentLibraryFilter[] {
  if (!value || typeof value.id !== 'string') return []
  if (value.scope !== 'library' && value.scope !== 'favorites') return []
  return [{
    id: value.id,
    scope: value.scope,
    label: typeof value.label === 'string' && value.label.trim() ? value.label : '筛选',
    state: normalizeRecentLibraryFilterState(value.state ?? {}),
    updatedAt: typeof value.updatedAt === 'string' ? value.updatedAt : '',
  }]
}

function hasRecentFilterValue(state: RecentLibraryFilterState) {
  return Boolean(
    state.query.trim()
      || state.authors.length
      || state.tags.length
      || state.excludeTags.length
      || state.metadataFilters.length,
  )
}

function recentFilterLabel(state: RecentLibraryFilterState) {
  const parts = [
    state.query.trim(),
    ...state.authors.map((author) => `作者:${author}`),
    ...state.tags.map((tag) => `标签:${tag}`),
    ...state.excludeTags.map((tag) => `排除:${tag}`),
    ...state.metadataFilters.map((filter) => metadataFilterLabel(filter)),
  ].filter(Boolean)
  return parts.slice(0, 3).join(' / ') || '筛选'
}

function filterSignature(state: RecentLibraryFilterState) {
  return JSON.stringify({
    query: state.query.trim(),
    authors: state.authors,
    tags: state.tags,
    excludeTags: state.excludeTags,
    metadataFilters: state.metadataFilters,
  })
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

function metadataFilterLabel(value: MetadataFilter) {
  const labels: Record<MetadataFilter, string> = {
    missingDescription: '缺简介',
    missingAuthors: '缺作者',
    missingTags: '缺标签',
    missingCover: '缺封面',
    missingPublishedAt: '缺发布时间',
  }
  return labels[value]
}
