<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { NAlert, NButton, NDrawer, NDrawerContent, NEmpty, NInputNumber, NPopover, NSelect, NSlider, NSpace, NSpin, NTag, NText, type SelectOption } from 'naive-ui'
import { toArchiveUrl } from '@/api/archive'
import { createBookmark, deleteBookmark, listBookmarks, type Bookmark } from '@/api/bookmark'
import { getBook } from '@/api/library'
import { listChapterPages, updateBookProgress } from '@/api/reader'
import { getReaderSettings, saveReaderSettings } from '@/api/settings'
import { toAssetUrl, type Book, type Chapter, type Page, type ReaderSettings } from '@/api/tauri'

const props = defineProps<{ bookId: string }>()

const route = useRoute()
const router = useRouter()
const book = ref<Book | null>(null)
const chapters = ref<Chapter[]>([])
const pages = ref<Page[]>([])
const chapterIndex = ref(0)
const pageIndex = ref(0)
const loading = ref(true)
const error = ref('')
const scrollViewport = ref<HTMLElement | null>(null)
const settingsReady = ref(false)
const settings = ref<ReaderSettings>({
  mode: 'single',
  fit: 'height',
  direction: 'ltr',
  background: '#111410',
  spaceScrollRatio: 0.88,
  spaceHoldSpeedRatio: 2.5,
  brightness: 1,
  contrast: 1,
  pageAnimation: 'none',
  preloadCacheLimit: 80,
})
const spaceHoldDelayMs = 180
const progressSaveDelayMs = 600
const zoomDragThreshold = 4
const largeImagePixelLimit = 60_000_000
const progressReady = ref(false)
let spaceHoldTimer = 0
let spaceHoldFrame = 0
let spaceHoldLastTime = 0
let spaceHoldDirection: 1 | -1 = 1
let readerSettingsSaveTimer = 0
let progressSaveTimer = 0
let progressSaveInFlight: Promise<void> | null = null
let lastSavedProgressKey = ''
let zoomDragMoved = false
let zoomPointerStart = { x: 0, y: 0 }
const preloadedImageUrls = new Map<string, HTMLImageElement>()
let pendingProgress: {
  bookId: string
  chapterId: string
  pageIndex: number
  key: string
} | null = null

// ── Bookmarks ──
const bookmarks = ref<Bookmark[]>([])
const showBookmarkDrawer = ref(false)
const showChapterDrawer = ref(false)

// ── Brightness/Contrast Quick Adjust ──
const showFilterPopover = ref(false)
const pageJumpValue = ref<number | null>(null)

// ── Page Animation ──
const pageDirection = ref<'forward' | 'backward'>('forward')

// ── Click-to-Zoom ──
const isZoomed = ref(false)
const zoomScale = ref(2.0)
const zoomOrigin = ref({ x: 50, y: 50 })
const isDragging = ref(false)
const dragStart = ref({ x: 0, y: 0 })
const dragOffset = ref({ x: 0, y: 0 })
const zoomedImageSrc = ref<string | undefined>()
const zoomedImageName = ref('')
const zoomedPage = ref<Page | null>(null)

const modeOptions: SelectOption[] = [
  { label: '单页', value: 'single' },
  { label: '双页', value: 'double' },
  { label: '滚动', value: 'scroll' },
]

const fitOptions: SelectOption[] = [
  { label: '适应高度', value: 'height' },
  { label: '适应宽度', value: 'width' },
  { label: '原始尺寸', value: 'original' },
]

const animationOptions: SelectOption[] = [
  { label: '无', value: 'none' },
  { label: '滑动', value: 'slide' },
  { label: '淡入淡出', value: 'fade' },
]

const currentChapter = computed(() => chapters.value[chapterIndex.value])
const currentPage = computed(() => pages.value[pageIndex.value])
const isArchiveBook = computed(() => book.value ? book.value.kind !== 'folder' : false)

function getPageImageUrl(page: Page): string | undefined {
  if (isArchiveBook.value && book.value) {
    return toArchiveUrl(book.value.path, page.uri)
  }
  if (page.path !== page.uri) {
    return toArchiveUrl(page.path, page.uri)
  }
  return toAssetUrl(page.path)
}

function preloadImageUrl(url?: string) {
  const cacheLimit = preloadCacheLimit()
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

function preloadCacheLimit() {
  return Math.round(clampNumber(settings.value.preloadCacheLimit, 0, 500, 80))
}

function trimPreloadCache() {
  const cacheLimit = preloadCacheLimit()
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
  if (!pages.value.length) return
  const radius = settings.value.mode === 'scroll' ? 4 : 2
  const start = Math.max(0, pageIndex.value - radius)
  const end = Math.min(pages.value.length - 1, pageIndex.value + radius)
  const visibleIndexes = new Set(
    settings.value.mode === 'double'
      ? [pageIndex.value, pageIndex.value + 1]
      : [pageIndex.value],
  )
  for (let index = start; index <= end; index += 1) {
    if (visibleIndexes.has(index)) continue
    preloadImageUrl(getPageImageUrl(pages.value[index]))
  }
}

const currentImageUrl = computed(() => currentPage.value ? getPageImageUrl(currentPage.value) : undefined)
const zoomedPageImageUrl = computed(() => zoomedPage.value ? getPageImageUrl(zoomedPage.value) : currentImageUrl.value)
const imageFitClass = computed(() => `fit-${settings.value.fit}`)
const doublePages = computed(() => {
  const pair = pages.value.slice(pageIndex.value, pageIndex.value + 2)
  return settings.value.direction === 'rtl' ? pair.reverse() : pair
})
const readerStyle = computed(() => ({ background: settings.value.background }))
const chapterOptions = computed<SelectOption[]>(() => chapters.value.map((chapter, index) => ({
  label: chapter.title,
  value: index,
})))

// ── Bookmark computed ──
const bookmarkedPageKeys = computed(() => {
  const keys = new Set<string>()
  for (const b of bookmarks.value) {
    keys.add(`${b.chapterId}:${b.pageIndex}`)
  }
  return keys
})

const isCurrentPageBookmarked = computed(() => {
  if (!currentChapter.value) return false
  return bookmarkedPageKeys.value.has(`${currentChapter.value!.id}:${pageIndex.value}`)
})

// ── Image filter computed ──
const imageFilterStyle = computed(() => {
  const { brightness, contrast } = settings.value
  if (brightness === 1 && contrast === 1) return undefined
  return { filter: `brightness(${brightness}) contrast(${contrast})` }
})

// ── Page animation computed ──
const currentPageKey = computed(() => `${chapterIndex.value}-${pageIndex.value}`)

const transitionName = computed(() => {
  if (settings.value.pageAnimation === 'none') return ''
  if (settings.value.mode === 'scroll') return ''
  if (isZoomed.value) return ''

  if (settings.value.pageAnimation === 'slide') {
    const isLtr = settings.value.direction === 'ltr'
    const isForward = pageDirection.value === 'forward'
    const effectiveLtr = isLtr === isForward
    return effectiveLtr ? 'reader-slide-ltr' : 'reader-slide-rtl'
  }

  return 'reader-fade'
})

// ── Zoom computed ──
const zoomStyle = computed(() => {
  if (!isZoomed.value) return undefined
  return {
    transform: `scale(${zoomScale.value}) translate(${dragOffset.value.x / zoomScale.value}px, ${dragOffset.value.y / zoomScale.value}px)`,
    transformOrigin: `${zoomOrigin.value.x}% ${zoomOrigin.value.y}%`,
    cursor: isDragging.value ? 'grabbing' : 'grab',
    transition: isDragging.value ? 'none' : 'transform 0.2s ease',
  }
})

// ── Load Book ──

async function loadBook() {
  loading.value = true
  progressReady.value = false
  settingsReady.value = false
  error.value = ''
  try {
    settings.value = await getReaderSettings()
    settingsReady.value = true
    const loadedBook = await getBook(props.bookId)
    book.value = loadedBook
    chapters.value = loadedBook.chapters

    const requestedChapterIndex = Number(route.query.chapter)
    const savedChapterIndex = Number.isInteger(requestedChapterIndex) && requestedChapterIndex >= 0
      ? requestedChapterIndex
      : loadedBook.lastChapterId
      ? chapters.value.findIndex((chapter) => chapter.id === loadedBook.lastChapterId)
      : 0

    chapterIndex.value = Math.min(savedChapterIndex >= 0 ? savedChapterIndex : 0, Math.max(chapters.value.length - 1, 0))
    pageIndex.value = Number.isInteger(requestedChapterIndex) && requestedChapterIndex >= 0
      ? 0
      : Math.max(loadedBook.lastPage || 0, 0)
    await loadPagesForCurrentChapter()
    pageIndex.value = Math.min(pageIndex.value, Math.max(pages.value.length - 1, 0))
    loading.value = false
    await nextTick()
    scrollToCurrentPageSoon()
    lastSavedProgressKey = progressKey()
    progressReady.value = true
    loadBookmarks()
  } catch (innerError) {
    error.value = String(innerError)
    loading.value = false
  }
}

async function loadPagesForCurrentChapter() {
  const chapter = currentChapter.value
  if (!chapter) {
    pages.value = []
    return
  }
  pages.value = await listChapterPages(chapter.id)
  pageIndex.value = Math.min(pageIndex.value, Math.max(pages.value.length - 1, 0))
  pageJumpValue.value = pages.value.length ? pageIndex.value + 1 : null
  await nextTick()
}

// ── Bookmarks ──

async function loadBookmarks() {
  if (!book.value) return
  try {
    bookmarks.value = await listBookmarks(book.value.id)
  } catch { /* ignore */ }
}

async function toggleBookmark() {
  if (!book.value || !currentChapter.value) return

  const existing = bookmarks.value.find(
    (b) => b.chapterId === currentChapter.value!.id && b.pageIndex === pageIndex.value,
  )

  if (existing) {
    try {
      await deleteBookmark(existing.id)
      bookmarks.value = bookmarks.value.filter((b) => b.id !== existing.id)
    } catch { /* ignore */ }
  } else {
    try {
      const title = `${currentChapter.value.title} · 第${pageIndex.value + 1}页`
      const created = await createBookmark({
        bookId: book.value.id,
        chapterId: currentChapter.value.id,
        pageIndex: pageIndex.value,
        title,
      })
      bookmarks.value = [created, ...bookmarks.value]
    } catch { /* ignore */ }
  }
}

async function jumpToBookmark(bookmark: Bookmark) {
  const targetChapterIndex = chapters.value.findIndex((c) => c.id === bookmark.chapterId)
  if (targetChapterIndex < 0) return
  chapterIndex.value = targetChapterIndex
  pageIndex.value = bookmark.pageIndex
  await loadPagesForCurrentChapter()
  scrollToCurrentPageSoon()
  showBookmarkDrawer.value = false
}

async function removeBookmark(bookmark: Bookmark) {
  try {
    await deleteBookmark(bookmark.id)
    bookmarks.value = bookmarks.value.filter((b) => b.id !== bookmark.id)
  } catch { /* ignore */ }
}

// ── Progress ──

function progressKey() {
  return `${currentChapter.value?.id ?? ''}:${pageIndex.value}`
}

function queueProgressSave() {
  if (!progressReady.value || !book.value || !currentChapter.value) return
  const nextProgressKey = progressKey()
  if (nextProgressKey === lastSavedProgressKey) return

  pendingProgress = {
    bookId: book.value.id,
    chapterId: currentChapter.value.id,
    pageIndex: pageIndex.value,
    key: nextProgressKey,
  }

  if (progressSaveTimer) window.clearTimeout(progressSaveTimer)
  progressSaveTimer = window.setTimeout(() => {
    void flushProgress()
  }, progressSaveDelayMs)
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

// ── Navigation ──

async function nextPage() {
  pageDirection.value = 'forward'
  const step = settings.value.mode === 'double' ? 2 : 1
  if (pageIndex.value < pages.value.length - step) {
    pageIndex.value += step
    scrollToCurrentPageSoon()
    return
  }

  if (chapterIndex.value < chapters.value.length - 1) {
    chapterIndex.value += 1
    pageIndex.value = 0
    await loadPagesForCurrentChapter()
    scrollToCurrentPageSoon()
  }
}

async function previousPage() {
  pageDirection.value = 'backward'
  const step = settings.value.mode === 'double' ? 2 : 1
  if (pageIndex.value > 0) {
    pageIndex.value = Math.max(0, pageIndex.value - step)
    scrollToCurrentPageSoon()
    return
  }

  if (chapterIndex.value > 0) {
    chapterIndex.value -= 1
    await loadPagesForCurrentChapter()
    pageIndex.value = settings.value.mode === 'double'
      ? Math.max(pages.value.length - 2, 0)
      : Math.max(pages.value.length - 1, 0)
    scrollToCurrentPageSoon()
  }
}

async function selectChapter(index: number) {
  chapterIndex.value = index
  pageIndex.value = 0
  await loadPagesForCurrentChapter()
  scrollToCurrentPageSoon()
}

async function jumpToChapter(index: number) {
  await selectChapter(index)
  showChapterDrawer.value = false
}

function jumpToPage() {
  if (!pages.value.length || !pageJumpValue.value) return
  pageIndex.value = Math.min(Math.max(Math.round(pageJumpValue.value), 1), pages.value.length) - 1
  scrollToCurrentPageSoon()
}

function jumpToChapterBoundary(target: 'first' | 'last') {
  if (!pages.value.length) return
  pageIndex.value = target === 'first' ? 0 : pages.value.length - 1
  scrollToCurrentPageSoon()
}

async function leaveReader() {
  await flushReaderSettings().catch(() => undefined)
  await flushProgress().catch(() => undefined)
  router.push('/library')
}

// ── Scroll ──

function onScroll() {
  if (settings.value.mode !== 'scroll' || !scrollViewport.value) return
  const images = [...scrollViewport.value.querySelectorAll<HTMLImageElement>('[data-page-index]')]
  const viewportTop = scrollViewport.value.getBoundingClientRect().top
  let closest = pageIndex.value
  let closestDistance = Number.POSITIVE_INFINITY

  for (const image of images) {
    const distance = Math.abs(image.getBoundingClientRect().top - viewportTop)
    if (distance < closestDistance) {
      closestDistance = distance
      closest = Number(image.dataset.pageIndex || 0)
    }
  }

  pageIndex.value = closest
}

function scrollToCurrentPage() {
  if (settings.value.mode !== 'scroll' || !scrollViewport.value) return
  const target = scrollViewport.value.querySelector<HTMLElement>(`[data-page-index="${pageIndex.value}"]`)
  target?.scrollIntoView({ block: 'start' })
}

function scrollToCurrentPageSoon() {
  scrollToCurrentPage()
  window.setTimeout(scrollToCurrentPage, 0)
  window.setTimeout(scrollToCurrentPage, 80)
}

function clampNumber(value: unknown, min: number, max: number, fallback: number) {
  const numeric = Number(value)
  if (!Number.isFinite(numeric)) return fallback
  return Math.min(max, Math.max(min, numeric))
}

function scrollByAmount(direction: 1 | -1, amount: number, behavior: ScrollBehavior = 'auto') {
  const viewport = scrollViewport.value
  if (!viewport) return false

  const maxScrollTop = viewport.scrollHeight - viewport.clientHeight
  const atStart = viewport.scrollTop <= 0
  const atEnd = viewport.scrollTop >= maxScrollTop - 2

  if (direction > 0 && atEnd) {
    stopSpaceHold()
    nextPage()
    return false
  }

  if (direction < 0 && atStart) {
    stopSpaceHold()
    previousPage()
    return false
  }

  viewport.scrollBy({ top: amount * direction, behavior })
  return true
}

function scrollBySpaceStep(direction: 1 | -1) {
  const viewport = scrollViewport.value
  if (!viewport) return

  const ratio = clampNumber(settings.value.spaceScrollRatio, 0.1, 2, 0.88)
  const scrollAmount = Math.max(Math.round(viewport.clientHeight * ratio), 120)
  scrollByAmount(direction, scrollAmount, 'smooth')
}

function startSpaceHold(direction: 1 | -1) {
  stopSpaceHold()
  spaceHoldDirection = direction
  spaceHoldTimer = window.setTimeout(() => {
    const tick = (time: number) => {
      const viewport = scrollViewport.value
      if (!viewport || settings.value.mode !== 'scroll') {
        stopSpaceHold()
        return
      }

      if (!spaceHoldLastTime) spaceHoldLastTime = time
      const deltaSeconds = (time - spaceHoldLastTime) / 1000
      spaceHoldLastTime = time
      const speedRatio = clampNumber(settings.value.spaceHoldSpeedRatio, 0.5, 10, 2.5)
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

// ── Brightness/Contrast Quick Save ──

async function saveFilterSettings() {
  try {
    await saveReaderSettings(settings.value)
  } catch { /* ignore */ }
}

function queueReaderSettingsSave() {
  if (!settingsReady.value) return
  if (readerSettingsSaveTimer) window.clearTimeout(readerSettingsSaveTimer)
  readerSettingsSaveTimer = window.setTimeout(() => {
    readerSettingsSaveTimer = 0
    saveReaderSettings(settings.value).catch(() => undefined)
  }, 250)
}

async function flushReaderSettings() {
  if (readerSettingsSaveTimer) {
    window.clearTimeout(readerSettingsSaveTimer)
    readerSettingsSaveTimer = 0
  }
  if (!settingsReady.value) return
  await saveReaderSettings(settings.value)
}

function resetFilter() {
  settings.value.brightness = 1
  settings.value.contrast = 1
  saveFilterSettings()
}

// ── Click-to-Zoom ──

function onImageClick(event: MouseEvent, page: Page) {
  if (isZoomed.value) {
    exitZoom()
    return
  }

  const img = event.currentTarget as HTMLImageElement
  if (!img) return
  const rect = img.getBoundingClientRect()
  const x = ((event.clientX - rect.left) / rect.width) * 100
  const y = ((event.clientY - rect.top) / rect.height) * 100

  zoomOrigin.value = { x, y }
  zoomScale.value = 2.0
  dragOffset.value = { x: 0, y: 0 }
  zoomedPage.value = page

  // For scroll mode, use overlay
  if (settings.value.mode === 'scroll') {
    zoomedImageSrc.value = getPageImageUrl(page)
    zoomedImageName.value = page.name
  }

  isZoomed.value = true
}

function exitZoom() {
  isZoomed.value = false
  isDragging.value = false
  dragOffset.value = { x: 0, y: 0 }
  zoomedImageSrc.value = undefined
  zoomedPage.value = null
  zoomDragMoved = false
}

function onZoomMouseDown(event: MouseEvent) {
  if (!isZoomed.value) return
  event.preventDefault()
  zoomDragMoved = false
  zoomPointerStart = { x: event.clientX, y: event.clientY }
  isDragging.value = true
  dragStart.value = {
    x: event.clientX - dragOffset.value.x,
    y: event.clientY - dragOffset.value.y,
  }
}

function onZoomMouseMove(event: MouseEvent) {
  if (!isDragging.value) return
  const distanceX = event.clientX - zoomPointerStart.x
  const distanceY = event.clientY - zoomPointerStart.y
  if (Math.hypot(distanceX, distanceY) > zoomDragThreshold) {
    zoomDragMoved = true
  }
  dragOffset.value = {
    x: event.clientX - dragStart.value.x,
    y: event.clientY - dragStart.value.y,
  }
}

function onZoomMouseUp() {
  isDragging.value = false
}

function onZoomClick(event: MouseEvent) {
  if (zoomDragMoved) {
    event.preventDefault()
    event.stopPropagation()
    zoomDragMoved = false
    return
  }

  exitZoom()
}

function onZoomWheel(event: WheelEvent) {
  if (!isZoomed.value) return
  event.preventDefault()
  const delta = event.deltaY > 0 ? -0.3 : 0.3
  zoomScale.value = Math.max(1.0, Math.min(5.0, zoomScale.value + delta))
  if (zoomScale.value <= 1.0) {
    exitZoom()
  }
}

function onViewportWheel(event: WheelEvent) {
  if (!isZoomed.value) return
  event.preventDefault()
  onZoomWheel(event)
}

function onReaderImageLoad(event: Event) {
  const image = event.currentTarget as HTMLImageElement
  const pixels = image.naturalWidth * image.naturalHeight
  image.classList.toggle('reader-large-image', pixels > largeImagePixelLimit)
}

// ── Keyboard ──

function onKeydown(event: KeyboardEvent) {
  if (isEditableKeyboardTarget(event.target)) return

  // Zoom escape
  if (event.key === 'Escape' && isZoomed.value) {
    event.preventDefault()
    exitZoom()
    return
  }

  if (event.key === ' ') {
    event.preventDefault()
    if (event.repeat) return
    if (settings.value.mode === 'scroll') {
      const direction = event.shiftKey ? -1 : 1
      scrollBySpaceStep(direction)
      startSpaceHold(direction)
    } else {
      settings.value.direction === 'rtl' ? previousPage() : nextPage()
    }
  }
  if (event.key === 'ArrowRight') {
    event.preventDefault()
    settings.value.direction === 'rtl' ? previousPage() : nextPage()
  }
  if (event.key === 'ArrowLeft') {
    event.preventDefault()
    settings.value.direction === 'rtl' ? nextPage() : previousPage()
  }
  if (event.key === 'Home') {
    event.preventDefault()
    jumpToChapterBoundary('first')
  }
  if (event.key === 'End') {
    event.preventDefault()
    jumpToChapterBoundary('last')
  }
  if (event.key === 'Escape') {
    void leaveReader()
  }
  // Bookmark shortcut
  if (event.key === 'b' && (event.ctrlKey || event.metaKey)) {
    event.preventDefault()
    toggleBookmark()
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

watch([chapterIndex, pageIndex], () => {
  pageJumpValue.value = pages.value.length ? pageIndex.value + 1 : null
  queueProgressSave()
  preloadNearbyPages()
})

watch(() => settings.value.mode, () => {
  preloadNearbyPages()
  queueReaderSettingsSave()
})
watch(() => settings.value.fit, queueReaderSettingsSave)
watch(() => settings.value.preloadCacheLimit, () => {
  trimPreloadCache()
  preloadNearbyPages()
})

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
  window.addEventListener('keyup', onKeyup)
  window.addEventListener('blur', stopSpaceHold)
  loadBook()
})

onBeforeUnmount(() => {
  void flushProgress().catch(() => undefined)
  void flushReaderSettings().catch(() => undefined)
  clearPreloadCache()
  stopSpaceHold()
  exitZoom()
  window.removeEventListener('keydown', onKeydown)
  window.removeEventListener('keyup', onKeyup)
  window.removeEventListener('blur', stopSpaceHold)
})
</script>

<template>
  <section class="reader-page" :style="readerStyle" @mousemove="onZoomMouseMove" @mouseup="onZoomMouseUp">
    <header class="reader-toolbar">
      <NButton size="small" @click="leaveReader">返回书架</NButton>
      <div v-if="book" class="reader-title">
        <strong>{{ book.title }}</strong>
        <NText v-if="currentChapter" depth="3">{{ currentChapter.title }}</NText>
      </div>
      <NSpace class="reader-actions" align="center" :wrap="true">
        <NButton size="small" :type="isCurrentPageBookmarked ? 'warning' : 'default'" @click="toggleBookmark">
          {{ isCurrentPageBookmarked ? '🔖' : '📑' }}
        </NButton>
        <NButton size="small" @click="showBookmarkDrawer = true">
          书签 ({{ bookmarks.length }})
        </NButton>
        <NButton size="small" @click="showChapterDrawer = true">
          目录
        </NButton>

        <NPopover trigger="click" :show="showFilterPopover" @update:show="(v: boolean) => { showFilterPopover = v; if (!v) saveFilterSettings() }">
          <template #trigger>
            <NButton size="small">☀</NButton>
          </template>
          <div class="filter-panel">
            <div class="filter-row">
              <span>亮度</span>
              <NSlider v-model:value="settings.brightness" :min="0.2" :max="2" :step="0.05" />
              <span class="filter-value">{{ settings.brightness.toFixed(2) }}</span>
            </div>
            <div class="filter-row">
              <span>对比度</span>
              <NSlider v-model:value="settings.contrast" :min="0.2" :max="2" :step="0.05" />
              <span class="filter-value">{{ settings.contrast.toFixed(2) }}</span>
            </div>
            <NButton size="small" block @click="resetFilter">重置</NButton>
          </div>
        </NPopover>

        <NSelect v-model:value="settings.mode" :options="modeOptions" size="small" class="reader-mode-select" />
        <NSelect v-model:value="settings.fit" :options="fitOptions" size="small" class="reader-fit-select" />
        <NSelect
          :value="chapterIndex"
          :options="chapterOptions"
          size="small"
          class="reader-chapter-select"
          @update:value="selectChapter"
        />
        <NSpace align="center" size="small">
          <NInputNumber
            v-model:value="pageJumpValue"
            size="small"
            class="reader-page-input"
            :min="1"
            :max="Math.max(pages.length, 1)"
            :show-button="false"
            @keyup.enter="jumpToPage"
          />
          <NButton size="small" :disabled="!pages.length" @click="jumpToPage">跳页</NButton>
        </NSpace>
        <NTag size="small" round>{{ pages.length ? pageIndex + 1 : 0 }} / {{ pages.length }}</NTag>
      </NSpace>
    </header>

    <div v-if="loading" class="reader-state">
      <NSpin description="正在加载..." />
    </div>
    <div v-else-if="error" class="reader-state">
      <NAlert type="error" :show-icon="false">{{ error }}</NAlert>
    </div>

    <!-- Scroll mode -->
    <div v-else-if="settings.mode === 'scroll'" ref="scrollViewport" class="reader-scroll" @scroll.passive="onScroll">
      <img
        v-for="page in pages"
        :key="page.uri"
        :class="[imageFitClass, { 'bookmark-indicator': bookmarkedPageKeys.has(`${currentChapter?.id}:${page.index}`) }]"
        :style="imageFilterStyle"
        :src="getPageImageUrl(page)"
        :alt="page.name"
        :data-page-index="page.index"
        loading="lazy"
        decoding="async"
        @load="onReaderImageLoad"
        @click="onImageClick($event, page)"
      />
    </div>

    <!-- Double page mode -->
    <div v-else-if="settings.mode === 'double' && pages.length" class="reader-viewport double-view">
      <template v-if="!isZoomed">
        <button class="reader-hit-area left" type="button" aria-label="上一页" @click="previousPage" />
        <Transition :name="transitionName" mode="out-in">
          <div :key="currentPageKey" class="reader-double-images">
            <img
              v-for="page in doublePages"
              :key="page.uri"
              :class="imageFitClass"
              :style="imageFilterStyle"
              :src="getPageImageUrl(page)"
              :alt="page.name"
              loading="eager"
              decoding="async"
              @load="onReaderImageLoad"
              @click="onImageClick($event, page)"
            />
          </div>
        </Transition>
        <button class="reader-hit-area right" type="button" aria-label="下一页" @click="nextPage" />
      </template>
      <template v-else>
        <img
          :class="imageFitClass"
          :style="[imageFilterStyle, zoomStyle]"
          :src="zoomedPageImageUrl"
          :alt="zoomedPage?.name || currentPage?.name"
          decoding="async"
          @click="onZoomClick"
          @mousedown="onZoomMouseDown"
          @wheel.prevent="onZoomWheel"
        />
      </template>
    </div>

    <!-- Single page mode -->
    <div v-else-if="currentImageUrl" class="reader-viewport" :class="{ 'reader-zoomed': isZoomed }"
         @mousedown="onZoomMouseDown" @wheel="onViewportWheel">
      <template v-if="!isZoomed">
        <button class="reader-hit-area left" type="button" aria-label="上一页" @click="previousPage" />
        <Transition :name="transitionName" mode="out-in">
          <img
            :key="currentPageKey"
            :class="[imageFitClass, { 'bookmark-indicator': isCurrentPageBookmarked }]"
            :style="imageFilterStyle"
            :src="currentImageUrl"
            :alt="currentPage?.name || currentChapter?.title || '漫画页'"
            loading="eager"
            decoding="async"
            @load="onReaderImageLoad"
            @click="currentPage && onImageClick($event, currentPage)"
          />
        </Transition>
        <button class="reader-hit-area right" type="button" aria-label="下一页" @click="nextPage" />
      </template>
      <template v-else>
        <img
          :class="imageFitClass"
          :style="[imageFilterStyle, zoomStyle]"
          :src="zoomedPageImageUrl"
          :alt="zoomedPage?.name || currentPage?.name"
          decoding="async"
          @click="onZoomClick"
        />
      </template>
    </div>

    <div v-else class="reader-state">
      <NEmpty description="当前章节没有可显示的图片。" />
    </div>

    <!-- Zoom overlay for scroll mode -->
    <div v-if="isZoomed && settings.mode === 'scroll'" class="reader-zoom-overlay"
         @click="onZoomClick" @wheel.prevent="onZoomWheel">
      <img
        :style="[imageFilterStyle, zoomStyle]"
        :src="zoomedImageSrc"
        :alt="zoomedImageName"
        decoding="async"
        @click.stop="onZoomClick"
        @mousedown.stop="onZoomMouseDown"
      />
    </div>

    <!-- Chapter drawer -->
    <NDrawer v-model:show="showChapterDrawer" :width="360" placement="right">
      <NDrawerContent title="目录" closable>
        <NEmpty v-if="!chapters.length" description="暂无章节" />
        <div v-else class="bookmark-list">
          <div
            v-for="(chapter, index) in chapters"
            :key="chapter.id"
            class="bookmark-item"
            :class="{ active: index === chapterIndex }"
            role="button"
            tabindex="0"
            @click="jumpToChapter(index)"
            @keydown.enter="jumpToChapter(index)"
          >
            <div class="bookmark-item-info">
              <NText class="bookmark-item-title">{{ chapter.title }}</NText>
              <NText depth="3" class="bookmark-item-time">{{ chapter.pageCount }} 页</NText>
            </div>
            <NTag v-if="index === chapterIndex" size="small" type="success" round>当前</NTag>
          </div>
        </div>
      </NDrawerContent>
    </NDrawer>

    <!-- Bookmark drawer -->
    <NDrawer v-model:show="showBookmarkDrawer" :width="360" placement="right">
      <NDrawerContent title="书签" closable>
        <NEmpty v-if="!bookmarks.length" description="暂无书签，按 Ctrl+B 添加" />
        <div v-else class="bookmark-list">
          <div
            v-for="bookmark in bookmarks"
            :key="bookmark.id"
            class="bookmark-item"
            role="button"
            tabindex="0"
            @click="jumpToBookmark(bookmark)"
            @keydown.enter="jumpToBookmark(bookmark)"
          >
            <div class="bookmark-item-info">
              <NText class="bookmark-item-title">{{ bookmark.title }}</NText>
              <NText depth="3" class="bookmark-item-time">{{ new Date(bookmark.createdAt).toLocaleString('zh-CN') }}</NText>
            </div>
            <NButton size="tiny" quaternary type="error" @click.stop="removeBookmark(bookmark)">删除</NButton>
          </div>
        </div>
      </NDrawerContent>
    </NDrawer>
  </section>
</template>
