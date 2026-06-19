import { onBeforeUnmount, onMounted, type Ref } from 'vue'
import type { ReaderSettings } from '@/api/tauri'
import { clampNumber } from '@/utils/readerSettings'

type PageDirection = 1 | -1

type UseReaderKeyboardOptions = {
  settings: Ref<ReaderSettings>
  isZoomed: Ref<boolean>
  scrollViewport: Ref<HTMLElement | null>
  nextPage: () => Promise<void>
  previousPage: () => Promise<void>
  jumpToChapterBoundary: (target: 'first' | 'last') => void
  exitZoom: () => void
  leaveReader: () => Promise<void>
  toggleBookmark: () => Promise<void>
  stopAutoScrollForManualInput: () => void
}

export function useReaderKeyboard(options: UseReaderKeyboardOptions) {
  const spaceHoldDelayMs = 180
  let spaceHoldTimer = 0
  let spaceHoldFrame = 0
  let spaceHoldLastTime = 0
  let spaceHoldDirection: PageDirection = 1

  function scrollByAmount(direction: PageDirection, amount: number, behavior: ScrollBehavior = 'auto') {
    const viewport = options.scrollViewport.value
    if (!viewport) return false

    const maxScrollTop = viewport.scrollHeight - viewport.clientHeight
    const atStart = viewport.scrollTop <= 0
    const atEnd = viewport.scrollTop >= maxScrollTop - 2

    if (direction > 0 && atEnd) {
      stopSpaceHold()
      void options.nextPage()
      return false
    }

    if (direction < 0 && atStart) {
      stopSpaceHold()
      void options.previousPage()
      return false
    }

    viewport.scrollBy({ top: amount * direction, behavior })
    return true
  }

  function scrollBySpaceStep(direction: PageDirection) {
    const viewport = options.scrollViewport.value
    if (!viewport) return

    const ratio = clampNumber(options.settings.value.spaceScrollRatio, 0.1, 2, 0.88)
    const scrollAmount = Math.max(Math.round(viewport.clientHeight * ratio), 120)
    scrollByAmount(direction, scrollAmount, 'smooth')
  }

  function startSpaceHold(direction: PageDirection) {
    stopSpaceHold()
    spaceHoldDirection = direction
    spaceHoldTimer = window.setTimeout(() => {
      const tick = (time: number) => {
        const viewport = options.scrollViewport.value
        if (!viewport || options.settings.value.mode !== 'scroll') {
          stopSpaceHold()
          return
        }

        if (!spaceHoldLastTime) spaceHoldLastTime = time
        const deltaSeconds = (time - spaceHoldLastTime) / 1000
        spaceHoldLastTime = time
        const speedRatio = clampNumber(options.settings.value.spaceHoldSpeedRatio, 0.5, 10, 2.5)
        const amount = viewport.clientHeight * speedRatio * deltaSeconds

        if (!scrollByAmount(spaceHoldDirection, amount, 'auto')) return
        spaceHoldFrame = window.requestAnimationFrame(tick)
      }

      spaceHoldLastTime = 0
      spaceHoldFrame = window.requestAnimationFrame(tick)
    }, spaceHoldDelayMs)
  }

  function stopSpaceHold() {
    if (spaceHoldTimer) {
      window.clearTimeout(spaceHoldTimer)
      spaceHoldTimer = 0
    }
    if (spaceHoldFrame) {
      window.cancelAnimationFrame(spaceHoldFrame)
      spaceHoldFrame = 0
    }
    spaceHoldLastTime = 0
  }

  function onManualScrollInput() {
    options.stopAutoScrollForManualInput()
  }

  function onKeydown(event: KeyboardEvent) {
    if (isEditableKeyboardTarget(event.target)) return

    if (event.key === 'Escape' && options.isZoomed.value) {
      event.preventDefault()
      options.exitZoom()
      return
    }

    if (event.key === ' ') {
      event.preventDefault()
      if (event.repeat) return
      options.stopAutoScrollForManualInput()
      if (options.settings.value.mode === 'scroll') {
        const direction = event.shiftKey ? -1 : 1
        scrollBySpaceStep(direction)
        startSpaceHold(direction)
      } else {
        void (options.settings.value.direction === 'rtl' ? options.previousPage() : options.nextPage())
      }
    }
    if (event.key === 'ArrowRight') {
      event.preventDefault()
      options.stopAutoScrollForManualInput()
      void (options.settings.value.direction === 'rtl' ? options.previousPage() : options.nextPage())
    }
    if (event.key === 'ArrowLeft') {
      event.preventDefault()
      options.stopAutoScrollForManualInput()
      void (options.settings.value.direction === 'rtl' ? options.nextPage() : options.previousPage())
    }
    if (event.key === 'Home') {
      event.preventDefault()
      options.stopAutoScrollForManualInput()
      options.jumpToChapterBoundary('first')
    }
    if (event.key === 'End') {
      event.preventDefault()
      options.stopAutoScrollForManualInput()
      options.jumpToChapterBoundary('last')
    }
    if (event.key === 'Escape') {
      void options.leaveReader()
    }
    if (event.key === 'b' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault()
      void options.toggleBookmark()
    }
  }

  function onKeyup(event: KeyboardEvent) {
    if (event.key === ' ') stopSpaceHold()
  }

  function isEditableKeyboardTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) return false
    const tagName = target.tagName.toLowerCase()
    return tagName === 'input'
      || tagName === 'textarea'
      || tagName === 'select'
      || target.isContentEditable
  }

  onMounted(() => {
    window.addEventListener('keydown', onKeydown)
    window.addEventListener('keyup', onKeyup)
    window.addEventListener('blur', stopSpaceHold)
  })

  onBeforeUnmount(() => {
    stopSpaceHold()
    window.removeEventListener('keydown', onKeydown)
    window.removeEventListener('keyup', onKeyup)
    window.removeEventListener('blur', stopSpaceHold)
  })

  return {
    onManualScrollInput,
    stopSpaceHold,
  }
}
