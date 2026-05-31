<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NCard, NCheckbox, NEllipsis, NTag, NText } from 'naive-ui'
import { toArchiveUrl } from '@/api/archive'
import { toAssetUrl, type BookSummary, type LibraryViewSettings } from '@/api/tauri'

const props = withDefaults(defineProps<{
  book: BookSummary
  settings: LibraryViewSettings
  favoriteButtonLabel: string
  selectable?: boolean
  selected?: boolean
}>(), {
  selectable: false,
  selected: false,
})

const emit = defineEmits<{
  open: [book: BookSummary]
  toggleFavorite: [book: BookSummary]
  toggleSelection: [book: BookSummary]
  selectTag: [tag: string]
  detail: [book: BookSummary]
  bookContextMenu: [payload: { book: BookSummary, x: number, y: number }]
}>()

const coverUrl = computed(() => {
  if (props.book.kind !== 'folder' && props.book.coverPath) {
    return toArchiveUrl(props.book.path, props.book.coverPath)
  }
  return toAssetUrl(props.book.coverPath)
})

const visibleTags = computed(() => {
  if (!props.settings.showTags || props.settings.tagLimit <= 0) return []
  return props.book.tags.slice(0, props.settings.tagLimit)
})

const progressPercent = computed(() => {
  if (!props.book.totalPages) return 0
  return Math.min(100, Math.round(((props.book.lastPage + 1) / props.book.totalPages) * 100))
})

const hasProgress = computed(() => Boolean(props.book.lastReadAt || props.book.lastPage > 0))
</script>

<template>
  <NCard class="book-card" embedded :bordered="false" content-style="padding: 10px;">
    <article
      class="book-card-body"
      role="button"
      tabindex="0"
      @click="emit('open', props.book)"
      @keydown.enter="emit('open', props.book)"
      @keydown.space.prevent="emit('open', props.book)"
      @contextmenu.prevent.stop="emit('bookContextMenu', { book: props.book, x: $event.clientX, y: $event.clientY })"
    >
      <div class="cover-wrap">
        <NCheckbox
          v-if="props.selectable"
          class="selection-checkbox"
          :checked="props.selected"
          :aria-label="props.selected ? `取消选择 ${props.book.title}` : `选择 ${props.book.title}`"
          @click.stop
          @update:checked="emit('toggleSelection', props.book)"
        />
        <NButton
          circle
          size="small"
          class="favorite-button"
          :type="props.book.isFavorite ? 'warning' : 'default'"
          :aria-label="props.favoriteButtonLabel"
          @click.stop="emit('toggleFavorite', props.book)"
        >
          {{ props.book.isFavorite ? '★' : '☆' }}
        </NButton>
        <img
          v-if="coverUrl"
          :src="coverUrl"
          :alt="props.book.title"
          loading="lazy"
          decoding="async"
          fetchpriority="low"
        />
        <div v-else class="cover-placeholder">无封面</div>
      </div>

      <div class="book-info">
        <NEllipsis :line-clamp="2" class="book-title" :tooltip="false">
          {{ props.book.title }}
        </NEllipsis>
        <NText v-if="props.settings.showAuthors && props.book.authors.length" depth="3" class="book-meta">
          {{ props.book.authors.join(' / ') }}
        </NText>
        <NText depth="3" class="book-meta">{{ props.book.chapterCount }} 章 · {{ props.book.totalPages }} 页</NText>
        <div class="book-progress">
          <span class="book-progress-bar">
            <span :style="{ width: `${progressPercent}%` }" />
          </span>
          <NText depth="3" class="book-meta">
            {{ hasProgress ? `读到 ${progressPercent}%` : '未开始' }}
          </NText>
        </div>
        <NButton
          v-if="hasProgress"
          size="tiny"
          secondary
          class="continue-button"
          @click.stop="emit('open', props.book)"
        >
          继续阅读
        </NButton>
        <NButton size="tiny" quaternary class="continue-button" @click.stop="emit('detail', props.book)">
          详情
        </NButton>
        <div v-if="visibleTags.length" class="tag-row">
          <NTag
            v-for="tag in visibleTags"
            :key="tag"
            size="small"
            round
            class="book-tag-clickable"
            role="button"
            tabindex="0"
            @click.stop="emit('selectTag', tag)"
            @keydown.enter.stop="emit('selectTag', tag)"
            @keydown.space.prevent.stop="emit('selectTag', tag)"
          >
            {{ tag }}
          </NTag>
        </div>
      </div>
    </article>
  </NCard>
</template>
