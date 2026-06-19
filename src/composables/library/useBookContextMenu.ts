import { ref } from 'vue'
import type { BookSummary } from '@/api/tauri'

type BookContextMenuPayload = {
  book: BookSummary
  x: number
  y: number
}

export function useBookContextMenu() {
  const contextMenuBook = ref<BookSummary | null>(null)
  const contextMenuVisible = ref(false)
  const contextMenuX = ref(0)
  const contextMenuY = ref(0)

  function openBookContextMenu(payload: BookContextMenuPayload) {
    contextMenuBook.value = payload.book
    contextMenuX.value = payload.x
    contextMenuY.value = payload.y
    contextMenuVisible.value = false
    requestAnimationFrame(() => {
      contextMenuVisible.value = true
    })
  }

  function closeBookContextMenu() {
    contextMenuVisible.value = false
  }

  return {
    contextMenuBook,
    contextMenuVisible,
    contextMenuX,
    contextMenuY,
    openBookContextMenu,
    closeBookContextMenu,
  }
}
