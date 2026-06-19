import { ref, type Ref } from 'vue'
import { renameBookTitle, resetBookTitle } from '@/api/library'
import type { BookSummary } from '@/api/tauri'

type UseBookRenameDialogOptions = {
  error: Ref<string>
  replaceBook: (book: BookSummary) => void
  onSuccess?: (message: string) => void
}

export function useBookRenameDialog(options: UseBookRenameDialogOptions) {
  const renameDialogBook = ref<BookSummary | null>(null)
  const renameTitleValue = ref('')
  const renameSubmitting = ref(false)

  function openRenameDialog(book: BookSummary) {
    renameDialogBook.value = book
    renameTitleValue.value = book.title
  }

  function closeRenameDialog() {
    if (renameSubmitting.value) return
    renameDialogBook.value = null
    renameTitleValue.value = ''
  }

  async function submitRenameTitle() {
    const book = renameDialogBook.value
    const title = renameTitleValue.value.trim()
    if (!book || !title || renameSubmitting.value) return

    renameSubmitting.value = true
    options.error.value = ''
    try {
      const updated = await renameBookTitle(book.path, title)
      options.replaceBook(updated)
      renameDialogBook.value = null
      renameTitleValue.value = ''
      options.onSuccess?.('漫画标题已重命名')
    } catch (innerError) {
      options.error.value = String(innerError)
    } finally {
      renameSubmitting.value = false
    }
  }

  async function resetBookTitleFromMenu(book: BookSummary) {
    options.error.value = ''
    try {
      const updated = await resetBookTitle(book.path)
      options.replaceBook(updated)
      options.onSuccess?.('已恢复默认标题')
    } catch (innerError) {
      options.error.value = String(innerError)
    }
  }

  return {
    renameDialogBook,
    renameTitleValue,
    renameSubmitting,
    openRenameDialog,
    closeRenameDialog,
    submitRenameTitle,
    resetBookTitleFromMenu,
  }
}
