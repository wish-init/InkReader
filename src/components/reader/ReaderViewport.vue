<script setup lang="ts">
import { onBeforeUnmount, ref, watch, type StyleValue } from 'vue'
import { NEmpty } from 'naive-ui'
import type { Chapter, Page, ReaderSettings } from '@/api/tauri'

const props = defineProps<{
  mode: ReaderSettings['mode']
  pages: Page[]
  currentPage?: Page
  currentChapter?: Chapter
  currentImageUrl?: string
  doublePages: Page[]
  imageFitClass: string
  imageFilterStyle?: StyleValue
  currentPageKey: string
  transitionName: string
  isZoomed: boolean
  isCurrentPageBookmarked: boolean
  bookmarkedPageKeys: Set<string>
  zoomStyle?: StyleValue
  zoomedPageImageUrl?: string
  zoomedPage?: Page | null
  zoomedImageSrc?: string
  zoomedImageName: string
  getPageImageUrl: (page: Page) => string | undefined
}>()

const emit = defineEmits<{
  scrollViewportChange: [element: HTMLElement | null]
  scroll: []
  manualScrollInput: []
  imageLoad: [event: Event]
  imageClick: [event: MouseEvent, page: Page]
  previousPage: []
  nextPage: []
  zoomMouseDown: [event: MouseEvent]
  zoomWheel: [event: WheelEvent]
  viewportWheel: [event: WheelEvent]
  zoomClick: [event: MouseEvent]
}>()

const scrollViewportElement = ref<HTMLElement | null>(null)

watch(scrollViewportElement, (element) => {
  emit('scrollViewportChange', element)
}, { immediate: true })

onBeforeUnmount(() => {
  emit('scrollViewportChange', null)
})
</script>

<template>
  <div
    v-if="mode === 'scroll'"
    ref="scrollViewportElement"
    class="reader-scroll"
    @scroll.passive="emit('scroll')"
    @wheel.passive="emit('manualScrollInput')"
    @touchstart.passive="emit('manualScrollInput')"
  >
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
      @load="emit('imageLoad', $event)"
      @click="emit('imageClick', $event, page)"
    />
  </div>

  <div v-else-if="mode === 'double' && pages.length" class="reader-viewport double-view">
    <template v-if="!isZoomed">
      <button class="reader-hit-area left" type="button" aria-label="上一页" @click="emit('previousPage')" />
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
            @load="emit('imageLoad', $event)"
            @click="emit('imageClick', $event, page)"
          />
        </div>
      </Transition>
      <button class="reader-hit-area right" type="button" aria-label="下一页" @click="emit('nextPage')" />
    </template>
    <template v-else>
      <img
        :class="imageFitClass"
        :style="[imageFilterStyle, zoomStyle]"
        :src="zoomedPageImageUrl"
        :alt="zoomedPage?.name || currentPage?.name"
        decoding="async"
        @click="emit('zoomClick', $event)"
        @mousedown="emit('zoomMouseDown', $event)"
        @wheel.prevent="emit('zoomWheel', $event)"
      />
    </template>
  </div>

  <div
    v-else-if="currentImageUrl"
    class="reader-viewport"
    :class="{ 'reader-zoomed': isZoomed }"
    @mousedown="emit('zoomMouseDown', $event)"
    @wheel="emit('viewportWheel', $event)"
  >
    <template v-if="!isZoomed">
      <button class="reader-hit-area left" type="button" aria-label="上一页" @click="emit('previousPage')" />
      <Transition :name="transitionName" mode="out-in">
        <img
          :key="currentPageKey"
          :class="[imageFitClass, { 'bookmark-indicator': isCurrentPageBookmarked }]"
          :style="imageFilterStyle"
          :src="currentImageUrl"
          :alt="currentPage?.name || currentChapter?.title || '漫画页'"
          loading="eager"
          decoding="async"
          @load="emit('imageLoad', $event)"
          @click="currentPage && emit('imageClick', $event, currentPage)"
        />
      </Transition>
      <button class="reader-hit-area right" type="button" aria-label="下一页" @click="emit('nextPage')" />
    </template>
    <template v-else>
      <img
        :class="imageFitClass"
        :style="[imageFilterStyle, zoomStyle]"
        :src="zoomedPageImageUrl"
        :alt="zoomedPage?.name || currentPage?.name"
        decoding="async"
        @click="emit('zoomClick', $event)"
      />
    </template>
  </div>

  <div v-else class="reader-state">
    <NEmpty description="当前章节没有可显示的图片。" />
  </div>

  <div
    v-if="isZoomed && mode === 'scroll'"
    class="reader-zoom-overlay"
    @click="emit('zoomClick', $event)"
    @wheel.prevent="emit('zoomWheel', $event)"
  >
    <img
      :style="[imageFilterStyle, zoomStyle]"
      :src="zoomedImageSrc"
      :alt="zoomedImageName"
      decoding="async"
      @click.stop="emit('zoomClick', $event)"
      @mousedown.stop="emit('zoomMouseDown', $event)"
    />
  </div>
</template>
