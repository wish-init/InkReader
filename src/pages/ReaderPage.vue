<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { NAlert, NSpin } from 'naive-ui'
import type { Bookmark } from '@/api/bookmark'
import BookmarkDrawer from '@/components/reader/BookmarkDrawer.vue'
import ChapterDrawer from '@/components/reader/ChapterDrawer.vue'
import ReaderToolbar from '@/components/reader/ReaderToolbar.vue'
import ReaderViewport from '@/components/reader/ReaderViewport.vue'
import { useAutoScroll } from '@/composables/reader/useAutoScroll'
import { useImagePreload } from '@/composables/reader/useImagePreload'
import { useReaderBook } from '@/composables/reader/useReaderBook'
import { useReaderBookmarks } from '@/composables/reader/useReaderBookmarks'
import { useReaderKeyboard } from '@/composables/reader/useReaderKeyboard'
import { useReaderNavigation } from '@/composables/reader/useReaderNavigation'
import { useReaderProgress } from '@/composables/reader/useReaderProgress'
import { useReaderSettings } from '@/composables/reader/useReaderSettings'
import { useReaderZoom } from '@/composables/reader/useReaderZoom'

const props = defineProps<{ bookId: string }>()

const route = useRoute()
const router = useRouter()
const {
  book,
  chapters,
  pages,
  chapterIndex,
  pageIndex,
  currentChapter,
  currentPage,
  loadBookMetadata,
  loadPagesForCurrentChapter,
  getPageImageUrl,
} = useReaderBook(props.bookId)
const loading = ref(true)
const error = ref('')
const scrollViewport = ref<HTMLElement | null>(null)
const readerPageElement = ref<HTMLElement | null>(null)
const {
  settings,
  settingsReady,
  hasBookReaderSettings,
  loadReaderSettings,
  saveFilterSettings,
  queueReaderSettingsSave,
  flushReaderSettings,
  resetFilter,
  saveBookOverride,
  clearBookOverride,
} = useReaderSettings()
const progressSaveDelayMs = 600
const largeImagePixelLimit = 60_000_000
const immersiveChromeHideDelayMs = 1800
let stopAutoScrollForZoom: () => void = () => undefined
let readerChromeHideTimer = 0

const showChapterDrawer = ref(false)
const isImmersiveMode = ref(false)
const isReaderChromeVisible = ref(true)

const {
  bookmarks,
  showBookmarkDrawer,
  bookmarkedPageKeys,
  isCurrentPageBookmarked,
  loadBookmarks,
  toggleBookmark,
  jumpToBookmark,
  removeBookmark,
} = useReaderBookmarks({
  book,
  currentChapter,
  pageIndex,
  jumpToBookmarkPage,
})
const {
  progressReady,
  resetProgressTracking,
  markProgressReady,
  queueProgressSave,
  flushProgress,
} = useReaderProgress({
  book,
  currentChapter,
  pageIndex,
  saveDelayMs: progressSaveDelayMs,
})

const {
  preloadNearbyPages,
  trimPreloadCache,
  clearPreloadCache,
} = useImagePreload({
  pages,
  pageIndex,
  mode: computed(() => settings.value.mode),
  preloadCacheLimit: computed(() => settings.value.preloadCacheLimit),
  getPageImageUrl,
})
const {
  isZoomed,
  zoomedImageSrc,
  zoomedImageName,
  zoomedPage,
  zoomedPageImageUrl,
  zoomStyle,
  onImageClick,
  exitZoom,
  onZoomMouseDown,
  onZoomMouseMove,
  onZoomMouseUp,
  onZoomClick,
  onZoomWheel,
  onViewportWheel,
} = useReaderZoom({
  currentPage,
  mode: computed(() => settings.value.mode),
  getPageImageUrl,
  onZoomStart: () => stopAutoScrollForZoom(),
})
const {
  isAutoScrollActive,
  isAutoScrollWaiting,
  toggleAutoScroll,
  stopAutoScroll,
  stopAutoScrollForManualInput,
} = useAutoScroll({
  settings,
  isZoomed,
  loading,
  pages,
  chapters,
  chapterIndex,
  pageIndex,
  scrollViewport,
  loadPagesForCurrentChapter,
  scrollToCurrentPageSoon,
})
stopAutoScrollForZoom = stopAutoScroll
const {
  pageJumpValue,
  pageDirection,
  syncPageJumpValue,
  nextPage,
  previousPage,
  jumpToChapter,
  manuallySelectChapter,
  manuallyJumpToPage,
  jumpToChapterBoundary,
} = useReaderNavigation({
  settings,
  chapters,
  pages,
  chapterIndex,
  pageIndex,
  loadPagesForCurrentChapter,
  scrollToCurrentPageSoon,
  stopAutoScrollForManualInput,
  closeChapterDrawer: () => {
    showChapterDrawer.value = false
  },
})
const {
  onManualScrollInput,
  stopSpaceHold,
} = useReaderKeyboard({
  settings,
  isZoomed,
  scrollViewport,
  nextPage,
  previousPage,
  jumpToChapterBoundary,
  exitZoom,
  leaveReader: handleReaderEscape,
  toggleBookmark,
  stopAutoScrollForManualInput,
})

const currentImageUrl = computed(() => currentPage.value ? getPageImageUrl(currentPage.value) : undefined)
const imageFitClass = computed(() => `fit-${settings.value.fit}`)
const doublePages = computed(() => {
  const pair = pages.value.slice(pageIndex.value, pageIndex.value + 2)
  return settings.value.direction === 'rtl' ? pair.reverse() : pair
})
const readerStyle = computed(() => ({ background: settings.value.background }))

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

// ── Load Book ──

async function loadBook() {
  loading.value = true
  resetProgressTracking()
  error.value = ''
  try {
    await loadReaderSettings(props.bookId)
    await loadBookMetadata(route.query.chapter)
    await loadPagesForCurrentChapter()
    pageIndex.value = Math.min(pageIndex.value, Math.max(pages.value.length - 1, 0))
    loading.value = false
    await nextTick()
    syncPageJumpValue()
    scrollToCurrentPageSoon()
    markProgressReady()
    loadBookmarks()
  } catch (innerError) {
    error.value = String(innerError)
    loading.value = false
  }
}

async function jumpToBookmarkPage(bookmark: Bookmark) {
  stopAutoScrollForManualInput()
  const targetChapterIndex = chapters.value.findIndex((c) => c.id === bookmark.chapterId)
  if (targetChapterIndex < 0) return
  chapterIndex.value = targetChapterIndex
  pageIndex.value = bookmark.pageIndex
  await loadPagesForCurrentChapter()
  await nextTick()
  syncPageJumpValue()
  scrollToCurrentPageSoon()
}

async function jumpToChapterPage(targetPageIndex: number) {
  stopAutoScrollForManualInput()
  pageIndex.value = Math.min(Math.max(targetPageIndex, 0), Math.max(pages.value.length - 1, 0))
  await nextTick()
  syncPageJumpValue()
  scrollToCurrentPageSoon()
  showChapterDrawer.value = false
}

async function leaveReader() {
  stopAutoScroll()
  await exitImmersiveMode()
  await flushReaderSettings().catch(() => undefined)
  await flushProgress().catch(() => undefined)
  router.push('/library')
}

async function handleReaderEscape() {
  if (isImmersiveMode.value) {
    await exitImmersiveMode()
    return
  }

  await leaveReader()
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

function setScrollViewport(element: HTMLElement | null) {
  scrollViewport.value = element
}

function clearReaderChromeHideTimer() {
  if (!readerChromeHideTimer) return
  window.clearTimeout(readerChromeHideTimer)
  readerChromeHideTimer = 0
}

function scheduleReaderChromeHide() {
  clearReaderChromeHideTimer()
  if (!isImmersiveMode.value) return

  readerChromeHideTimer = window.setTimeout(() => {
    isReaderChromeVisible.value = false
    readerChromeHideTimer = 0
  }, immersiveChromeHideDelayMs)
}

function revealReaderChrome() {
  if (!isImmersiveMode.value) return
  isReaderChromeVisible.value = true
  scheduleReaderChromeHide()
}

function onReaderMouseMove(event: MouseEvent) {
  revealReaderChrome()
  onZoomMouseMove(event)
}

async function enterImmersiveMode() {
  isImmersiveMode.value = true
  isReaderChromeVisible.value = true
  scheduleReaderChromeHide()

  if (readerPageElement.value?.requestFullscreen) {
    await readerPageElement.value.requestFullscreen().catch(() => undefined)
  }
}

async function exitImmersiveMode() {
  clearReaderChromeHideTimer()
  isImmersiveMode.value = false
  isReaderChromeVisible.value = true

  if (document.fullscreenElement === readerPageElement.value && document.exitFullscreen) {
    await document.exitFullscreen().catch(() => undefined)
  }
}

function toggleImmersiveMode() {
  void (isImmersiveMode.value ? exitImmersiveMode() : enterImmersiveMode())
}

function onFullscreenChange() {
  if (!isImmersiveMode.value) return
  if (document.fullscreenElement === readerPageElement.value) return

  clearReaderChromeHideTimer()
  isImmersiveMode.value = false
  isReaderChromeVisible.value = true
}

function onReaderImageLoad(event: Event) {
  const image = event.currentTarget as HTMLImageElement
  const pixels = image.naturalWidth * image.naturalHeight
  image.classList.toggle('reader-large-image', pixels > largeImagePixelLimit)
}

watch([chapterIndex, pageIndex], () => {
  syncPageJumpValue()
  queueProgressSave()
  preloadNearbyPages()
})

watch(() => settings.value.mode, () => {
  if (settings.value.mode !== 'scroll') stopAutoScroll()
  preloadNearbyPages()
  queueReaderSettingsSave()
})
watch(() => settings.value.fit, queueReaderSettingsSave)
watch(() => settings.value.autoScrollSpeed, queueReaderSettingsSave)
watch(() => settings.value.autoScrollStartDelay, queueReaderSettingsSave)
watch(() => settings.value.autoScrollStopOnManualScroll, queueReaderSettingsSave)
watch(() => settings.value.preloadCacheLimit, () => {
  trimPreloadCache()
  preloadNearbyPages()
})

onMounted(() => {
  document.addEventListener('fullscreenchange', onFullscreenChange)
  loadBook()
})

onBeforeUnmount(() => {
  void flushProgress().catch(() => undefined)
  void flushReaderSettings().catch(() => undefined)
  clearPreloadCache()
  clearReaderChromeHideTimer()
  document.removeEventListener('fullscreenchange', onFullscreenChange)
  if (document.fullscreenElement === readerPageElement.value && document.exitFullscreen) {
    void document.exitFullscreen().catch(() => undefined)
  }
  stopSpaceHold()
  stopAutoScroll()
  exitZoom()
})
</script>

<template>
  <section
    ref="readerPageElement"
    class="reader-page"
    :class="{
      'reader-page-immersive': isImmersiveMode,
      'reader-chrome-visible': !isImmersiveMode || isReaderChromeVisible,
    }"
    :style="readerStyle"
    @mousemove="onReaderMouseMove"
    @pointerdown="revealReaderChrome"
    @touchstart.passive="revealReaderChrome"
    @mouseup="onZoomMouseUp"
  >
    <ReaderToolbar
      v-model:settings="settings"
      v-model:page-jump-value="pageJumpValue"
      :book="book"
      :current-chapter="currentChapter"
      :chapters="chapters"
      :chapter-index="chapterIndex"
      :page-index="pageIndex"
      :page-count="pages.length"
      :is-current-page-bookmarked="isCurrentPageBookmarked"
      :bookmark-count="bookmarks.length"
      :is-auto-scroll-active="isAutoScrollActive"
      :is-auto-scroll-waiting="isAutoScrollWaiting"
      :is-immersive-mode="isImmersiveMode"
      :has-book-reader-settings="hasBookReaderSettings"
      :loading="loading"
      @leave="leaveReader"
      @toggle-immersive-mode="toggleImmersiveMode"
      @save-book-reader-settings="saveBookOverride"
      @clear-book-reader-settings="clearBookOverride"
      @toggle-bookmark="toggleBookmark"
      @open-bookmarks="showBookmarkDrawer = true"
      @open-chapters="showChapterDrawer = true"
      @save-filter-settings="saveFilterSettings"
      @reset-filter="resetFilter"
      @toggle-auto-scroll="toggleAutoScroll"
      @queue-reader-settings-save="queueReaderSettingsSave"
      @select-chapter="manuallySelectChapter"
      @jump-to-page="manuallyJumpToPage"
    />

    <div v-if="loading" class="reader-state">
      <NSpin description="正在加载..." />
    </div>
    <div v-else-if="error" class="reader-state">
      <NAlert type="error" :show-icon="false">{{ error }}</NAlert>
    </div>

    <ReaderViewport
      v-else
      :mode="settings.mode"
      :pages="pages"
      :current-page="currentPage"
      :current-chapter="currentChapter"
      :current-image-url="currentImageUrl"
      :double-pages="doublePages"
      :image-fit-class="imageFitClass"
      :image-filter-style="imageFilterStyle"
      :current-page-key="currentPageKey"
      :transition-name="transitionName"
      :is-zoomed="isZoomed"
      :is-current-page-bookmarked="isCurrentPageBookmarked"
      :bookmarked-page-keys="bookmarkedPageKeys"
      :zoom-style="zoomStyle"
      :zoomed-page-image-url="zoomedPageImageUrl"
      :zoomed-page="zoomedPage"
      :zoomed-image-src="zoomedImageSrc"
      :zoomed-image-name="zoomedImageName"
      :get-page-image-url="getPageImageUrl"
      @scroll-viewport-change="setScrollViewport"
      @scroll="onScroll"
      @manual-scroll-input="onManualScrollInput"
      @image-load="onReaderImageLoad"
      @image-click="onImageClick"
      @previous-page="previousPage"
      @next-page="nextPage"
      @zoom-mouse-down="onZoomMouseDown"
      @zoom-wheel="onZoomWheel"
      @viewport-wheel="onViewportWheel"
      @zoom-click="onZoomClick"
    />

    <button
      v-if="isImmersiveMode"
      class="reader-immersive-exit"
      type="button"
      @click="exitImmersiveMode"
      @mouseenter="revealReaderChrome"
      @focus="revealReaderChrome"
    >
      退出专注
    </button>

    <ChapterDrawer
      v-model:show="showChapterDrawer"
      :chapters="chapters"
      :current-chapter-index="chapterIndex"
      :pages="pages"
      :current-page-index="pageIndex"
      :get-page-image-url="getPageImageUrl"
      @jump="jumpToChapter"
      @jump-page="jumpToChapterPage"
    />

    <BookmarkDrawer
      v-model:show="showBookmarkDrawer"
      :bookmarks="bookmarks"
      @jump="jumpToBookmark"
      @remove="removeBookmark"
    />
  </section>
</template>
