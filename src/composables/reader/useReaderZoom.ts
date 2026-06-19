import { computed, ref, type ComputedRef } from 'vue'
import type { Page, ReaderSettings } from '@/api/tauri'

type Point = {
  x: number
  y: number
}

type UseReaderZoomOptions = {
  currentPage: ComputedRef<Page | undefined>
  mode: ComputedRef<ReaderSettings['mode']>
  getPageImageUrl: (page: Page) => string | undefined
  onZoomStart?: () => void
  dragThreshold?: number
}

export function useReaderZoom(options: UseReaderZoomOptions) {
  const dragThreshold = options.dragThreshold ?? 4
  const isZoomed = ref(false)
  const zoomScale = ref(2.0)
  const zoomOrigin = ref<Point>({ x: 50, y: 50 })
  const isDragging = ref(false)
  const dragStart = ref<Point>({ x: 0, y: 0 })
  const dragOffset = ref<Point>({ x: 0, y: 0 })
  const zoomedImageSrc = ref<string | undefined>()
  const zoomedImageName = ref('')
  const zoomedPage = ref<Page | null>(null)
  let zoomDragMoved = false
  let zoomPointerStart: Point = { x: 0, y: 0 }

  const zoomedPageImageUrl = computed(() => (
    zoomedPage.value
      ? options.getPageImageUrl(zoomedPage.value)
      : options.currentPage.value
      ? options.getPageImageUrl(options.currentPage.value)
      : undefined
  ))

  const zoomStyle = computed(() => {
    if (!isZoomed.value) return undefined
    return {
      transform: `scale(${zoomScale.value}) translate(${dragOffset.value.x / zoomScale.value}px, ${dragOffset.value.y / zoomScale.value}px)`,
      transformOrigin: `${zoomOrigin.value.x}% ${zoomOrigin.value.y}%`,
      cursor: isDragging.value ? 'grabbing' : 'grab',
      transition: isDragging.value ? 'none' : 'transform 0.2s ease',
    }
  })

  function onImageClick(event: MouseEvent, page: Page) {
    if (isZoomed.value) {
      exitZoom()
      return
    }
    options.onZoomStart?.()

    const image = event.currentTarget as HTMLImageElement
    if (!image) return
    const rect = image.getBoundingClientRect()
    const x = ((event.clientX - rect.left) / rect.width) * 100
    const y = ((event.clientY - rect.top) / rect.height) * 100

    zoomOrigin.value = { x, y }
    zoomScale.value = 2.0
    dragOffset.value = { x: 0, y: 0 }
    zoomedPage.value = page

    if (options.mode.value === 'scroll') {
      zoomedImageSrc.value = options.getPageImageUrl(page)
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
    if (Math.hypot(distanceX, distanceY) > dragThreshold) {
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

  return {
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
  }
}
