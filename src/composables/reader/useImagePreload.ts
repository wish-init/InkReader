import type { ComputedRef, Ref } from 'vue'
import type { Page, ReaderSettings } from '@/api/tauri'
import { clampNumber } from '@/utils/readerSettings'

type UseImagePreloadOptions = {
  pages: Ref<Page[]>
  pageIndex: Ref<number>
  mode: ComputedRef<ReaderSettings['mode']>
  preloadCacheLimit: ComputedRef<number>
  getPageImageUrl: (page: Page) => string | undefined
}

export function useImagePreload(options: UseImagePreloadOptions) {
  const preloadedImageUrls = new Map<string, HTMLImageElement>()

  function normalizedPreloadCacheLimit() {
    return Math.round(clampNumber(options.preloadCacheLimit.value, 0, 500, 80))
  }

  function preloadImageUrl(url?: string) {
    const cacheLimit = normalizedPreloadCacheLimit()
    if (!url || cacheLimit <= 0) return

    const existingImage = preloadedImageUrls.get(url)
    if (existingImage) {
      preloadedImageUrls.delete(url)
      preloadedImageUrls.set(url, existingImage)
      return
    }

    window.setTimeout(() => {
      const image = new Image()
      image.decoding = 'async'
      image.src = url
      preloadedImageUrls.set(url, image)
      trimPreloadCache()
    }, 0)
  }

  function trimPreloadCache() {
    const cacheLimit = normalizedPreloadCacheLimit()
    if (cacheLimit <= 0) {
      clearPreloadCache()
      return
    }

    while (preloadedImageUrls.size > cacheLimit) {
      const oldestUrl = preloadedImageUrls.keys().next().value
      if (!oldestUrl) break
      preloadedImageUrls.delete(oldestUrl)
    }
  }

  function clearPreloadCache() {
    preloadedImageUrls.clear()
  }

  function preloadNearbyPages() {
    if (!options.pages.value.length) return
    const radius = options.mode.value === 'scroll' ? 4 : 2
    const start = Math.max(0, options.pageIndex.value - radius)
    const end = Math.min(options.pages.value.length - 1, options.pageIndex.value + radius)
    const visibleIndexes = new Set(
      options.mode.value === 'double'
        ? [options.pageIndex.value, options.pageIndex.value + 1]
        : [options.pageIndex.value],
    )

    for (let index = start; index <= end; index += 1) {
      if (visibleIndexes.has(index)) continue
      preloadImageUrl(options.getPageImageUrl(options.pages.value[index]))
    }
  }

  return {
    preloadNearbyPages,
    trimPreloadCache,
    clearPreloadCache,
  }
}
