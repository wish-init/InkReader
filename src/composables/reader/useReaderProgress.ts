import { ref, type Ref } from 'vue'
import { updateBookProgress } from '@/api/reader'
import type { Book, Chapter } from '@/api/tauri'

type UseReaderProgressOptions = {
  book: Ref<Book | null>
  currentChapter: Ref<Chapter | undefined>
  pageIndex: Ref<number>
  saveDelayMs: number
}

type PendingProgress = {
  bookId: string
  chapterId: string
  pageIndex: number
  key: string
}

export function useReaderProgress(options: UseReaderProgressOptions) {
  const progressReady = ref(false)
  let progressSaveTimer = 0
  let progressSaveInFlight: Promise<void> | null = null
  let lastSavedProgressKey = ''
  let pendingProgress: PendingProgress | null = null

  function progressKey() {
    return `${options.currentChapter.value?.id ?? ''}:${options.pageIndex.value}`
  }

  function resetProgressTracking() {
    progressReady.value = false
    pendingProgress = null
  }

  function markProgressReady() {
    lastSavedProgressKey = progressKey()
    progressReady.value = true
  }

  function queueProgressSave() {
    if (!progressReady.value || !options.book.value || !options.currentChapter.value) return
    const nextProgressKey = progressKey()
    if (nextProgressKey === lastSavedProgressKey) return

    pendingProgress = {
      bookId: options.book.value.id,
      chapterId: options.currentChapter.value.id,
      pageIndex: options.pageIndex.value,
      key: nextProgressKey,
    }

    if (progressSaveTimer) window.clearTimeout(progressSaveTimer)
    progressSaveTimer = window.setTimeout(() => {
      void flushProgress()
    }, options.saveDelayMs)
  }

  async function flushProgress(): Promise<void> {
    if (progressSaveTimer) {
      window.clearTimeout(progressSaveTimer)
      progressSaveTimer = 0
    }

    if (progressSaveInFlight) {
      await progressSaveInFlight
    }

    const progress = pendingProgress
    if (!progress) return

    if (progress.key === lastSavedProgressKey) {
      pendingProgress = null
      return
    }

    pendingProgress = null
    progressSaveInFlight = updateBookProgress(progress.bookId, progress.chapterId, progress.pageIndex)
      .then(() => {
        lastSavedProgressKey = progress.key
      })
      .finally(() => {
        progressSaveInFlight = null
      })

    await progressSaveInFlight

    if (pendingProgress) {
      await flushProgress()
    }
  }

  return {
    progressReady,
    progressKey,
    resetProgressTracking,
    markProgressReady,
    queueProgressSave,
    flushProgress,
  }
}
