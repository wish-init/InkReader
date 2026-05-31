import type { BookSummary } from '@/api/tauri'

export type { BookSortKey, SortDirection } from '@/api/tauri'

export function sortBooks<T extends BookSummary>(books: T[], sortKey: import('@/api/tauri').BookSortKey, direction: import('@/api/tauri').SortDirection): T[] {
  const multiplier = direction === 'asc' ? 1 : -1
  return [...books].sort((a, b) => {
    if (sortKey === 'lastReadAt') {
      const aTime = parseOptionalDate(a.lastReadAt)
      const bTime = parseOptionalDate(b.lastReadAt)
      if (aTime === null && bTime !== null) return 1
      if (aTime !== null && bTime === null) return -1
      if (aTime !== null && bTime !== null) {
        const primary = (aTime - bTime) * multiplier
        if (primary !== 0) return primary
      }
      return compareBooksByStableFallback(a, b)
    }

    const primary = compareBooks(a, b, sortKey) * multiplier
    if (primary !== 0) return primary
    return compareBooksByStableFallback(a, b)
  })
}

function compareBooks(a: BookSummary, b: BookSummary, sortKey: import('@/api/tauri').BookSortKey): number {
  if (sortKey === 'title') {
    return compareText(a.title, b.title)
  }

  if (sortKey === 'totalPages') {
    return a.totalPages - b.totalPages
  }

  return new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime()
}

function parseOptionalDate(value?: string): number | null {
  if (!value) return null
  const time = new Date(value).getTime()
  return Number.isFinite(time) ? time : null
}

function compareBooksByStableFallback(a: BookSummary, b: BookSummary): number {
  return compareText(a.title, b.title) || compareText(a.path, b.path) || compareText(a.id, b.id)
}

function compareText(a: string, b: string): number {
  return a.localeCompare(b, 'zh-Hans-CN')
}
