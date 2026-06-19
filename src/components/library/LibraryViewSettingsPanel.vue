<script setup lang="ts">
import { NForm, NFormItem, NGrid, NGridItem, NSelect, NSwitch } from 'naive-ui'
import type { LibraryViewSettings } from '@/api/tauri'
import {
  libraryCoverSizeSelectOptions,
  libraryLayoutSelectOptions,
  tagLimitOptions,
  titleFontSizeOptions,
  titleLineClampOptions,
} from '@/utils/libraryViewSettings'

const settings = defineModel<LibraryViewSettings>({ required: true })
</script>

<template>
  <NForm label-placement="top" class="view-settings-panel">
    <NGrid :cols="7" :x-gap="14" :y-gap="14" responsive="screen">
      <NGridItem>
        <NFormItem label="列表样式" path="layout">
          <NSelect v-model:value="settings.layout" :options="libraryLayoutSelectOptions" />
        </NFormItem>
      </NGridItem>

      <NGridItem>
        <NFormItem label="封面大小" path="coverSize">
          <NSelect v-model:value="settings.coverSize" :options="libraryCoverSizeSelectOptions" />
        </NFormItem>
      </NGridItem>

      <NGridItem>
        <NFormItem label="标签数量" path="tagLimit">
          <NSelect v-model:value="settings.tagLimit" :options="tagLimitOptions" />
        </NFormItem>
      </NGridItem>

      <NGridItem>
        <NFormItem label="标题行数" path="titleLineClamp">
          <NSelect v-model:value="settings.titleLineClamp" :options="titleLineClampOptions" />
        </NFormItem>
      </NGridItem>

      <NGridItem>
        <NFormItem label="标题字号" path="titleFontSize">
          <NSelect v-model:value="settings.titleFontSize" :options="titleFontSizeOptions" />
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
