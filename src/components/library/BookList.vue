<script setup lang="ts">
import BookCard from '@/components/library/BookCard.vue'
import type { BookSummary, LibraryViewSettings } from '@/api/tauri'

const props = withDefaults(defineProps<{
  books: BookSummary[]
  settings: LibraryViewSettings
  highlightQuery?: string
  selectable?: boolean
  selectedBookPaths?: Set<string>
}>(), {
  selectable: false,
  selectedBookPaths: () => new Set<string>(),
})

const emit = defineEmits<{
  open: [book: BookSummary]
  toggleFavorite: [book: BookSummary]
  toggleSelection: [book: BookSummary]
  selectTag: [tag: string]
  detail: [book: BookSummary]
  bookContextMenu: [payload: { book: BookSummary, x: number, y: number }]
}>()
</script>

<template>
  <div class="book-grid" :class="[`layout-${props.settings.layout}`, `cover-${props.settings.coverSize}`]">
    <BookCard
      v-for="book in props.books"
      :key="book.id"
      :book="book"
      :settings="props.settings"
      :highlight-query="props.highlightQuery"
      :favorite-button-label="book.isFavorite ? `管理收藏夹 ${book.title}` : `选择收藏夹 ${book.title}`"
      :selectable="props.selectable"
      :selected="props.selectedBookPaths.has(book.path)"
      @open="emit('open', $event)"
      @toggle-favorite="emit('toggleFavorite', $event)"
      @toggle-selection="emit('toggleSelection', $event)"
      @select-tag="emit('selectTag', $event)"
      @detail="emit('detail', $event)"
      @book-context-menu="emit('bookContextMenu', $event)"
    />
  </div>
</template>
