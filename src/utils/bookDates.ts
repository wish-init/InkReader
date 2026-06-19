export function formatBookPublishedAt(value?: string | null): string {
  const timestamp = parseBookPublishedAtTime(value)
  if (timestamp === null) return ''

  const date = new Date(timestamp)
  if (!Number.isFinite(date.getTime())) return ''

  return date.toLocaleDateString('zh-CN')
}

export function parseBookPublishedAtTime(value?: string | null): number | null {
  if (!value) return null

  const timestamp = Number(value.trim())
  if (!Number.isFinite(timestamp) || timestamp <= 0) return null

  const milliseconds = timestamp >= 10_000_000_000 ? timestamp : timestamp * 1000
  const date = new Date(milliseconds)
  if (!Number.isFinite(date.getTime())) return null

  return milliseconds
}
