<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { NButton, NDrawer, NDrawerContent, NEmpty, NTag, NText } from 'naive-ui'
import type { Chapter, Page } from '@/api/tauri'

const props = defineProps<{
  chapters: Chapter[]
  currentChapterIndex: number
  pages: Page[]
  currentPageIndex: number
  getPageImageUrl: (page: Page) => string | undefined
}>()

const show = defineModel<boolean>('show', { required: true })
const emit = defineEmits<{
  jump: [index: number]
  jumpPage: [index: number]
}>()

const thumbnailBatchSize = 48
const visibleThumbnailCount = ref(thumbnailBatchSize)
const visiblePages = computed(() => props.pages.slice(0, visibleThumbnailCount.value))
const hasMorePages = computed(() => visibleThumbnailCount.value < props.pages.length)

watch([show, () => props.currentChapterIndex], () => {
  visibleThumbnailCount.value = thumbnailBatchSize
})

function showMoreThumbnails() {
  visibleThumbnailCount.value = Math.min(visibleThumbnailCount.value + thumbnailBatchSize, props.pages.length)
}
</script>

<template>
  <NDrawer v-model:show="show" :width="520" placement="right">
    <NDrawerContent title="目录" closable>
      <NEmpty v-if="!chapters.length" description="暂无章节" />
      <div v-else class="chapter-navigation">
        <div class="bookmark-list">
          <div
            v-for="(chapter, index) in chapters"
            :key="chapter.id"
            class="bookmark-item"
            :class="{ active: index === currentChapterIndex }"
            role="button"
            tabindex="0"
            @click="emit('jump', index)"
            @keydown.enter="emit('jump', index)"
          >
            <div class="bookmark-item-info">
              <NText class="bookmark-item-title">{{ chapter.title }}</NText>
              <NText depth="3" class="bookmark-item-time">{{ chapter.pageCount }} 页</NText>
            </div>
            <NTag v-if="index === currentChapterIndex" size="small" type="success" round>当前</NTag>
          </div>
        </div>

        <section class="chapter-thumbnail-section">
          <div class="chapter-thumbnail-header">
            <NText strong>页面</NText>
            <NText depth="3">{{ pages.length ? currentPageIndex + 1 : 0 }} / {{ pages.length }}</NText>
          </div>

          <NEmpty v-if="!pages.length" description="当前章节暂无页面" />
          <div v-else class="chapter-thumbnail-grid">
            <button
              v-for="page in visiblePages"
              :key="page.uri"
              class="chapter-thumbnail"
              :class="{ active: page.index === currentPageIndex }"
              type="button"
              @click="emit('jumpPage', page.index)"
            >
              <img
                :src="getPageImageUrl(page)"
                :alt="page.name"
                loading="lazy"
                decoding="async"
              />
              <span>{{ page.index + 1 }}</span>
            </button>
          </div>

          <NButton
            v-if="hasMorePages"
            size="small"
            block
            class="chapter-thumbnail-more"
            @click="showMoreThumbnails"
          >
            显示更多页面
          </NButton>
        </section>
      </div>
    </NDrawerContent>
  </NDrawer>
</template>
