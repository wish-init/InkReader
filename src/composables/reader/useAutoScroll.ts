import { nextTick, ref, type Ref } from 'vue'
import type { Chapter, Page, ReaderSettings } from '@/api/tauri'
import { clampNumber, defaultReaderSettings } from '@/utils/readerSettings'

type UseAutoScrollOptions = {
  settings: Ref<ReaderSettings>
  isZoomed: Ref<boolean>
  loading: Ref<boolean>
  pages: Ref<Page[]>
  chapters: Ref<Chapter[]>
  chapterIndex: Ref<number>
  pageIndex: Ref<number>
  scrollViewport: Ref<HTMLElement | null>
  loadPagesForCurrentChapter: () => Promise<void>
  scrollToCurrentPageSoon: () => void
}

export function useAutoScroll(options: UseAutoScrollOptions) {
  const isAutoScrollActive = ref(false)
  const isAutoScrollWaiting = ref(false)
  let autoScrollStartTimer = 0
  let autoScrollFrame = 0
  let autoScrollLastTime = 0

  function toggleAutoScroll() {
    if (isAutoScrollActive.value) {
      stopAutoScroll()
      return
    }
    startAutoScroll()
  }

  function startAutoScroll() {
    if (
      options.settings.value.mode !== 'scroll'
      || options.isZoomed.value
      || options.loading.value
      || !options.pages.value.length
    ) {
      return
    }

    stopAutoScroll()
    isAutoScrollActive.value = true
    const delayMs = Math.round(clampNumber(options.settings.value.autoScrollStartDelay, 0, 5, 0) * 1000)
    if (delayMs > 0) {
      isAutoScrollWaiting.value = true
      autoScrollStartTimer = window.setTimeout(() => {
        autoScrollStartTimer = 0
        isAutoScrollWaiting.value = false
        scheduleAutoScrollFrame()
      }, delayMs)
      return
    }
    scheduleAutoScrollFrame()
  }

  function stopAutoScroll() {
    isAutoScrollActive.value = false
    isAutoScrollWaiting.value = false
    if (autoScrollStartTimer) {
      window.clearTimeout(autoScrollStartTimer)
      autoScrollStartTimer = 0
    }
    if (autoScrollFrame) {
      window.cancelAnimationFrame(autoScrollFrame)
      autoScrollFrame = 0
    }
    autoScrollLastTime = 0
  }

  function stopAutoScrollForManualInput() {
    if (options.settings.value.autoScrollStopOnManualScroll) stopAutoScroll()
  }

  function scheduleAutoScrollFrame() {
    if (!isAutoScrollActive.value || isAutoScrollWaiting.value || autoScrollFrame) return
    autoScrollFrame = window.requestAnimationFrame((time) => {
      autoScrollFrame = 0
      void runAutoScrollFrame(time)
    })
  }

  async function runAutoScrollFrame(time: number) {
    const viewport = options.scrollViewport.value
    if (!isAutoScrollActive.value || options.settings.value.mode !== 'scroll' || options.isZoomed.value || !viewport) {
      stopAutoScroll()
      return
    }

    const maxScrollTop = Math.max(0, viewport.scrollHeight - viewport.clientHeight)
    if (maxScrollTop <= 2 && hasPendingScrollImages(viewport)) {
      scheduleAutoScrollFrame()
      return
    }

    if (viewport.scrollTop >= maxScrollTop - 2) {
      await continueAutoScrollAtChapterEnd()
      return
    }

    if (!autoScrollLastTime) autoScrollLastTime = time
    const deltaSeconds = Math.min((time - autoScrollLastTime) / 1000, 0.1)
    autoScrollLastTime = time
    const speed = clampNumber(
      options.settings.value.autoScrollSpeed,
      20,
      400,
      defaultReaderSettings.autoScrollSpeed,
    )
    viewport.scrollBy({ top: speed * deltaSeconds, behavior: 'auto' })
    scheduleAutoScrollFrame()
  }

  function hasPendingScrollImages(viewport: HTMLElement) {
    const images = [...viewport.querySelectorAll<HTMLImageElement>('[data-page-index]')]
    return images.some((image) => !image.complete)
  }

  async function continueAutoScrollAtChapterEnd() {
    if (options.chapterIndex.value >= options.chapters.value.length - 1) {
      stopAutoScroll()
      return
    }

    options.chapterIndex.value += 1
    options.pageIndex.value = 0
    await options.loadPagesForCurrentChapter()
    await nextTick()
    options.scrollToCurrentPageSoon()
    autoScrollLastTime = 0
    scheduleAutoScrollFrame()
  }

  return {
    isAutoScrollActive,
    isAutoScrollWaiting,
    toggleAutoScroll,
    startAutoScroll,
    stopAutoScroll,
    stopAutoScrollForManualInput,
  }
}
