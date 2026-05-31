import { call } from './tauri'

/** Construct an archive:// URL for loading an image from inside a CBZ/ZIP archive. */
export function toArchiveUrl(archivePath: string, entryName: string): string {
  const path = encodeURIComponent(archivePath)
  const entry = encodeURIComponent(entryName.replace(/\\/g, '/'))
  return `http://archive.localhost/${path}?entry=${entry}`
}

/** Get the cover entry name from a CBZ/ZIP archive (e.g., "cover.jpg" or "001.jpg"). */
export function getArchiveCoverEntry(archivePath: string): Promise<string> {
  return call('get_archive_cover_entry', { archivePath })
}
