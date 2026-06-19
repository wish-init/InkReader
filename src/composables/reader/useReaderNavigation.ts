import { nextTick, ref, type Ref } from 'vue'
import type { Chapter, Page, ReaderSettings } from '@/api/tauri'

type PageDirection = 'forward' | 'backward'

type UseReaderNavigationOptions = {
  settings: Ref<ReaderSettings>
  chapters: Ref<Chapter[]>
  pages: Ref<Page[]>
  chapterIndex: Ref<number>
  pageIndex: Ref<number>
  loadPagesForCurrentChapter: () => Promise<void>
  scrollToCurrentPageSoon: () => void
  stopAutoScrollForManualInput: () => void
  closeChapterDrawer: () => void
}

export function useReaderNavigation(options: UseReaderNavigationOptions) {
  const pageJumpValue = ref<number | null>(null)
  const pageDirection = ref<PageDirection>('forward')

  function syncPageJumpValue() {
    pageJumpValue.value = options.pages.value.length ? options.pageIndex.value + 1 : null
  }

  async function nextPage() {
    pageDirection.value = 'forward'
    const step = options.settings.value.mode === 'double' ? 2 : 1
    if (options.pageIndex.value < options.pages.value.length - step) {
      options.pageIndex.value += step
      options.scrollToCurrentPageSoon()
      return
    }

    if (options.chapterIndex.value < options.chapters.value.length - 1) {
      options.chapterIndex.value += 1
      options.pageIndex.value = 0
      await options.loadPagesForCurrentChapter()
      await nextTick()
      syncPageJumpValue()
      options.scrollToCurrentPageSoon()
    }
  }

  async function previousPage() {
    pageDirection.value = 'backward'
    const step = options.settings.value.mode === 'double' ? 2 : 1
    if (options.pageIndex.value > 0) {
      options.pageIndex.value = Math.max(0, options.pageIndex.value - step)
      options.scrollToCurrentPageSoon()
      return
    }

    if (options.chapterIndex.value > 0) {
      options.chapterIndex.value -= 1
      await options.loadPagesForCurrentChapter()
      options.pageIndex.value = options.settings.value.mode === 'double'
        ? Math.max(options.pages.value.length - 2, 0)
        : Math.max(options.pages.value.length - 1, 0)
      await nextTick()
      syncPageJumpValue()
      options.scrollToCurrentPageSoon()
    }
  }

  async function selectChapter(index: number) {
    options.chapterIndex.value = index
    options.pageIndex.value = 0
    await options.loadPagesForCurrentChapter()
    await nextTick()
    syncPageJumpValue()
    options.scrollToCurrentPageSoon()
  }

  async function jumpToChapter(index: number) {
    options.stopAutoScrollForManualInput()
    await selectChapter(index)
    options.closeChapterDrawer()
  }

  function jumpToPage() {
    if (!options.pages.value.length || !pageJumpValue.value) return
    options.pageIndex.value = Math.min(Math.max(Math.round(pageJumpValue.value), 1), options.pages.value.length) - 1
    options.scrollToCurrentPageSoon()
  }

  function manuallySelectChapter(index: number) {
    options.stopAutoScrollForManualInput()
    void selectChapter(index)
  }

  function manuallyJumpToPage() {
    options.stopAutoScrollForManualInput()
    jumpToPage()
  }

  function jumpToChapterBoundary(target: 'first' | 'last') {
    if (!options.pages.value.length) return
    options.pageIndex.value = target === 'first' ? 0 : options.pages.value.length - 1
    options.scrollToCurrentPageSoon()
  }

  return {
    pageJumpValue,
    pageDirection,
    syncPageJumpValue,
    nextPage,
    previousPage,
    selectChapter,
    jumpToChapter,
    jumpToPage,
    manuallySelectChapter,
    manuallyJumpToPage,
    jumpToChapterBoundary,
  }
}
