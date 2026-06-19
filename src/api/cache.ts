import { call, type CacheMaintenanceResult, type CacheMaintenanceSummary } from './tauri'

export function getCacheMaintenanceSummary(): Promise<CacheMaintenanceSummary> {
  return call('get_cache_maintenance_summary')
}

export function cleanupThumbnailCache(): Promise<CacheMaintenanceResult> {
  return call('cleanup_thumbnail_cache')
}

export function rebuildMissingThumbnails(): Promise<CacheMaintenanceResult> {
  return call('rebuild_missing_thumbnails')
}
