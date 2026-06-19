<script setup lang="ts">
import { computed, type CSSProperties } from 'vue'
import { NButton, NCard, NCheckbox, NEllipsis, NTag, NText } from 'naive-ui'
import { toAssetUrl, type BookSummary, type LibraryViewSettings } from '@/api/tauri'
import { getReadingStatus, getReadingStatusLabel } from '@/utils/readingStatus'
import { getReadingProgressPercent } from '@/utils/readingProgress'
import { normalizeTitleFontSize, normalizeTitleLineClamp } from '@/utils/libraryViewSettings'
import { formatBookPublishedAt } from '@/utils/bookDates'

const props = withDefaults(defineProps<{
  book: BookSummary
  settings: LibraryViewSettings
  favoriteButtonLabel: string
  highlightQuery?: string
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
  selectAuthor: [author: string]
  detail: [book: BookSummary]
  bookContextMenu: [payload: { book: BookSummary, x: number, y: number }]
}>()

const coverUrl = computed(() => {
  return toAssetUrl(props.book.thumbnailPath)
})

const visibleTags = computed(() => {
  if (!props.settings.showTags || props.settings.tagLimit <= 0) return []
  return props.book.tags.slice(0, props.settings.tagLimit)
})

const progressPercent = computed(() => {
  return getReadingProgressPercent(props.book)
})
const publishedAtLabel = computed(() => formatBookPublishedAt(props.book.publishedAt))

const readingStatus = computed(() => getReadingStatus(props.book))
const readingStatusLabel = computed(() => getReadingStatusLabel(readingStatus.value))
const hasProgress = computed(() => readingStatus.value !== 'unread')
const normalizedHighlightQuery = computed(() => normalizeText(props.highlightQuery ?? ''))
const highlightedTitle = computed(() => highlightText(props.book.title, normalizedHighlightQuery.value))
const titleLineHeight = 1.4
const titleLineClamp = computed(() => normalizeTitleLineClamp(props.settings.titleLineClamp))
const titleFontSize = computed(() => normalizeTitleFontSize(props.settings.titleFontSize))
const bookTitleStyle = computed<CSSProperties>(() => ({
  fontSize: `${titleFontSize.value}px`,
  lineHeight: String(titleLineHeight),
  minHeight: `${Math.ceil(titleLineClamp.value * titleFontSize.value * titleLineHeight)}px`,
}))

type TextSegment = {
  text: string
  highlight: boolean
}

function normalizeText(value: string) {
  return value.trim().toLocaleLowerCase()
}

function highlightText(value: string, normalizedQuery: string): TextSegment[] {
  if (!normalizedQuery) return [{ text: value, highlight: false }]

  const normalizedValue = value.toLocaleLowerCase()
  const index = normalizedValue.indexOf(normalizedQuery)
  if (index < 0) return [{ text: value, highlight: false }]

  const endIndex = index + normalizedQuery.length
  return [
    { text: value.slice(0, index), highlight: false },
    { text: value.slice(index, endIndex), highlight: true },
    { text: value.slice(endIndex), highlight: false },
  ].filter((segment) => segment.text)
}
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
        <NEllipsis
          :line-clamp="titleLineClamp"
          class="book-title"
          :style="bookTitleStyle"
          :tooltip="{ contentStyle: { maxWidth: '360px', whiteSpace: 'normal' } }"
        >
          <template #tooltip>{{ props.book.title }}</template>
          <template v-for="(segment, index) in highlightedTitle" :key="index">
            <mark v-if="segment.highlight" class="search-highlight">{{ segment.text }}</mark>
            <template v-else>{{ segment.text }}</template>
          </template>
        </NEllipsis>
        <NText v-if="props.settings.showAuthors && props.book.authors.length" depth="3" class="book-meta book-authors">
          <template v-for="(author, authorIndex) in props.book.authors" :key="`${author}-${authorIndex}`">
            <button
              type="button"
              class="book-author-link"
              @click.stop="emit('selectAuthor', author)"
              @keydown.enter.stop
              @keydown.space.stop
            >
              <template v-for="(segment, segmentIndex) in highlightText(author, normalizedHighlightQuery)" :key="segmentIndex">
                <mark v-if="segment.highlight" class="search-highlight">{{ segment.text }}</mark>
                <template v-else>{{ segment.text }}</template>
              </template>
            </button>
            <span v-if="authorIndex < props.book.authors.length - 1" class="book-author-separator"> / </span>
          </template>
        </NText>
        <NText depth="3" class="book-meta">{{ props.book.chapterCount }} 章 · {{ props.book.totalPages }} 页</NText>
        <NText v-if="publishedAtLabel" depth="3" class="book-meta">漫画发布时间：{{ publishedAtLabel }}</NText>
        <div class="book-progress">
          <span class="book-progress-bar">
            <span :style="{ width: `${progressPercent}%` }" />
          </span>
          <NText depth="3" class="book-meta">
            {{ hasProgress ? `${readingStatusLabel} ${progressPercent}%` : readingStatusLabel }}
          </NText>
        </div>
        <NTag
          size="small"
          round
          :type="readingStatus === 'read' ? 'success' : readingStatus === 'reading' ? 'info' : 'default'"
        >
          {{ readingStatusLabel }}
        </NTag>
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
