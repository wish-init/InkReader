<script setup lang="ts">
import { computed, ref } from 'vue'
import {
  NButton,
  NInputNumber,
  NPopover,
  NSelect,
  NSlider,
  NSpace,
  NSwitch,
  NTag,
  NText,
  type SelectOption,
} from 'naive-ui'
import type { Book, Chapter, ReaderSettings } from '@/api/tauri'
import {
  readerFitSelectOptions,
  readerModeSelectOptions,
  readerSettingRanges,
} from '@/utils/readerSettings'

const props = defineProps<{
  book: Book | null
  currentChapter?: Chapter
  chapters: Chapter[]
  chapterIndex: number
  pageIndex: number
  pageCount: number
  isCurrentPageBookmarked: boolean
  bookmarkCount: number
  isAutoScrollActive: boolean
  isAutoScrollWaiting: boolean
  isImmersiveMode: boolean
  hasBookReaderSettings: boolean
  loading: boolean
}>()

const settings = defineModel<ReaderSettings>('settings', { required: true })
const pageJumpValue = defineModel<number | null>('pageJumpValue', { required: true })
const emit = defineEmits<{
  leave: []
  toggleImmersiveMode: []
  saveBookReaderSettings: []
  clearBookReaderSettings: []
  toggleBookmark: []
  openBookmarks: []
  openChapters: []
  saveFilterSettings: []
  resetFilter: []
  toggleAutoScroll: []
  queueReaderSettingsSave: []
  selectChapter: [index: number]
  jumpToPage: []
}>()

const showFilterPopover = ref(false)
const showAutoScrollPopover = ref(false)

const autoScrollSpeedLabel = computed(() => `${Math.round(settings.value.autoScrollSpeed)} px/s`)
const autoScrollDelayLabel = computed(() => `${settings.value.autoScrollStartDelay.toFixed(1)} s`)
const chapterOptions = computed<SelectOption[]>(() => props.chapters.map((chapter, index) => ({
  label: chapter.title,
  value: index,
})))

function updateFilterPopover(value: boolean) {
  showFilterPopover.value = value
  if (!value) emit('saveFilterSettings')
}

function updateAutoScrollPopover(value: boolean) {
  showAutoScrollPopover.value = value
  if (!value) emit('queueReaderSettingsSave')
}
</script>

<template>
  <header class="reader-toolbar">
    <NButton size="small" @click="emit('leave')">返回书架</NButton>
    <div v-if="book" class="reader-title">
      <strong>{{ book.title }}</strong>
      <NText v-if="currentChapter" depth="3">{{ currentChapter.title }}</NText>
    </div>
    <NSpace class="reader-actions" align="center" :wrap="true">
      <NButton size="small" :type="isCurrentPageBookmarked ? 'warning' : 'default'" @click="emit('toggleBookmark')">
        {{ isCurrentPageBookmarked ? '🔖' : '📑' }}
      </NButton>
      <NButton size="small" @click="emit('openBookmarks')">
        书签 ({{ bookmarkCount }})
      </NButton>
      <NButton size="small" @click="emit('openChapters')">
        目录
      </NButton>

      <NButton
        size="small"
        :type="isImmersiveMode ? 'primary' : 'default'"
        @click="emit('toggleImmersiveMode')"
      >
        {{ isImmersiveMode ? '退出专注' : '专注' }}
      </NButton>

      <NButton
        size="small"
        :type="hasBookReaderSettings ? 'success' : 'default'"
        @click="emit('saveBookReaderSettings')"
      >
        保存本书
      </NButton>
      <NButton
        size="small"
        :disabled="!hasBookReaderSettings"
        @click="emit('clearBookReaderSettings')"
      >
        清除本书
      </NButton>

      <NPopover trigger="click" :show="showFilterPopover" @update:show="updateFilterPopover">
        <template #trigger>
          <NButton size="small">☀</NButton>
        </template>
        <div class="filter-panel">
          <div class="filter-row">
            <span>亮度</span>
            <NSlider v-model:value="settings.brightness" :min="readerSettingRanges.brightness.min" :max="readerSettingRanges.brightness.max" :step="0.05" />
            <span class="filter-value">{{ settings.brightness.toFixed(2) }}</span>
          </div>
          <div class="filter-row">
            <span>对比度</span>
            <NSlider v-model:value="settings.contrast" :min="readerSettingRanges.contrast.min" :max="readerSettingRanges.contrast.max" :step="0.05" />
            <span class="filter-value">{{ settings.contrast.toFixed(2) }}</span>
          </div>
          <NButton size="small" block @click="emit('resetFilter')">重置</NButton>
        </div>
      </NPopover>

      <NButton
        size="small"
        :type="isAutoScrollActive ? 'success' : 'default'"
        :disabled="settings.mode !== 'scroll' || loading || !pageCount"
        @click="emit('toggleAutoScroll')"
      >
        {{ isAutoScrollWaiting ? '等待中' : isAutoScrollActive ? '暂停自动滚动' : '自动滚动' }}
      </NButton>

      <NPopover trigger="click" :show="showAutoScrollPopover" @update:show="updateAutoScrollPopover">
        <template #trigger>
          <NButton size="small" :disabled="settings.mode !== 'scroll'">自动设置</NButton>
        </template>
        <div class="auto-scroll-panel">
          <div class="auto-scroll-header">
            <NButton
              size="small"
              :type="isAutoScrollActive ? 'success' : 'primary'"
              block
              :disabled="settings.mode !== 'scroll' || loading || !pageCount"
              @click="emit('toggleAutoScroll')"
            >
              {{ isAutoScrollWaiting ? '等待中' : isAutoScrollActive ? '暂停' : '开始' }}
            </NButton>
          </div>
          <div class="filter-row">
            <span>速度</span>
            <NSlider v-model:value="settings.autoScrollSpeed" :min="readerSettingRanges.autoScrollSpeed.min" :max="readerSettingRanges.autoScrollSpeed.max" :step="10" />
            <span class="filter-value">{{ autoScrollSpeedLabel }}</span>
          </div>
          <div class="filter-row">
            <span>延迟</span>
            <NSlider v-model:value="settings.autoScrollStartDelay" :min="readerSettingRanges.autoScrollStartDelay.min" :max="readerSettingRanges.autoScrollStartDelay.max" :step="0.5" />
            <span class="filter-value">{{ autoScrollDelayLabel }}</span>
          </div>
          <div class="auto-scroll-switch-row">
            <span>手动操作停止</span>
            <NSwitch v-model:value="settings.autoScrollStopOnManualScroll" />
          </div>
        </div>
      </NPopover>

      <NSelect v-model:value="settings.mode" :options="readerModeSelectOptions" size="small" class="reader-mode-select" />
      <NSelect v-model:value="settings.fit" :options="readerFitSelectOptions" size="small" class="reader-fit-select" />
      <NSelect
        :value="chapterIndex"
        :options="chapterOptions"
        size="small"
        class="reader-chapter-select"
        @update:value="emit('selectChapter', $event)"
      />
      <NSpace align="center" size="small">
        <NInputNumber
          v-model:value="pageJumpValue"
          size="small"
          class="reader-page-input"
          :min="1"
          :max="Math.max(pageCount, 1)"
          :show-button="false"
          @keyup.enter="emit('jumpToPage')"
        />
        <NButton size="small" :disabled="!pageCount" @click="emit('jumpToPage')">跳页</NButton>
      </NSpace>
      <NTag size="small" round>{{ pageCount ? pageIndex + 1 : 0 }} / {{ pageCount }}</NTag>
    </NSpace>
  </header>
</template>
