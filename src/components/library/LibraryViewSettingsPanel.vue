<script setup lang="ts">
import { NForm, NFormItem, NGrid, NGridItem, NSelect, NSwitch } from 'naive-ui'
import type { LibraryViewSettings } from '@/api/tauri'

const settings = defineModel<LibraryViewSettings>({ required: true })

const layoutOptions = [
  { label: '网格', value: 'grid' },
  { label: '紧凑网格', value: 'compact' },
  { label: '列表', value: 'list' },
]

const coverSizeOptions = [
  { label: '小', value: 'small' },
  { label: '中', value: 'medium' },
  { label: '大', value: 'large' },
]

const tagLimitOptions = [
  { label: '不显示', value: 0 },
  { label: '2 个', value: 2 },
  { label: '4 个', value: 4 },
  { label: '8 个', value: 8 },
  { label: '全部', value: 999 },
]
</script>

<template>
  <NForm label-placement="top" class="view-settings-panel">
    <NGrid :cols="5" :x-gap="14" :y-gap="14" responsive="screen">
      <NGridItem>
        <NFormItem label="列表样式" path="layout">
          <NSelect v-model:value="settings.layout" :options="layoutOptions" />
        </NFormItem>
      </NGridItem>

      <NGridItem>
        <NFormItem label="封面大小" path="coverSize">
          <NSelect v-model:value="settings.coverSize" :options="coverSizeOptions" />
        </NFormItem>
      </NGridItem>

      <NGridItem>
        <NFormItem label="标签数量" path="tagLimit">
          <NSelect v-model:value="settings.tagLimit" :options="tagLimitOptions" />
        </NFormItem>
      </NGridItem>

      <NGridItem>
        <NFormItem label="显示作者" path="showAuthors">
          <NSwitch v-model:value="settings.showAuthors" />
        </NFormItem>
      </NGridItem>

      <NGridItem>
        <NFormItem label="显示标签" path="showTags">
          <NSwitch v-model:value="settings.showTags" />
        </NFormItem>
      </NGridItem>
    </NGrid>
  </NForm>
</template>
