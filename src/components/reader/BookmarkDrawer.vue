<script setup lang="ts">
import { NButton, NDrawer, NDrawerContent, NEmpty, NText } from 'naive-ui'
import type { Bookmark } from '@/api/bookmark'

defineProps<{
  bookmarks: Bookmark[]
}>()

const show = defineModel<boolean>('show', { required: true })
const emit = defineEmits<{
  jump: [bookmark: Bookmark]
  remove: [bookmark: Bookmark]
}>()
</script>

<template>
  <NDrawer v-model:show="show" :width="360" placement="right">
    <NDrawerContent title="书签" closable>
      <NEmpty v-if="!bookmarks.length" description="暂无书签，按 Ctrl+B 添加" />
      <div v-else class="bookmark-list">
        <div
          v-for="bookmark in bookmarks"
          :key="bookmark.id"
          class="bookmark-item"
          role="button"
          tabindex="0"
          @click="emit('jump', bookmark)"
          @keydown.enter="emit('jump', bookmark)"
        >
          <div class="bookmark-item-info">
            <NText class="bookmark-item-title">{{ bookmark.title }}</NText>
            <NText depth="3" class="bookmark-item-time">
              {{ new Date(bookmark.createdAt).toLocaleString('zh-CN') }}
            </NText>
          </div>
          <NButton size="tiny" quaternary type="error" @click.stop="emit('remove', bookmark)">
            删除
          </NButton>
        </div>
      </div>
    </NDrawerContent>
  </NDrawer>
</template>
