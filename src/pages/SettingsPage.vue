<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { NAlert, NButton, NCard, NColorPicker, NForm, NFormItem, NGrid, NGridItem, NInputNumber, NPageHeader, NSelect, NText, useMessage, type SelectOption } from 'naive-ui'
import { getReaderSettings, ping, saveReaderSettings } from '@/api/settings'
import type { ReaderSettings } from '@/api/tauri'

const defaultSettings: ReaderSettings = {
  mode: 'single',
  fit: 'height',
  direction: 'ltr',
  background: '#111410',
  spaceScrollRatio: 0.88,
  spaceHoldSpeedRatio: 2.5,
  brightness: 1,
  contrast: 1,
  pageAnimation: 'none',
  preloadCacheLimit: 80,
}

const modeOptions: SelectOption[] = [
  { label: '单页', value: 'single' },
  { label: '双页', value: 'double' },
  { label: '长条滚动', value: 'scroll' },
]

const fitOptions: SelectOption[] = [
  { label: '适应高度', value: 'height' },
  { label: '适应宽度', value: 'width' },
  { label: '原始尺寸', value: 'original' },
]

const directionOptions: SelectOption[] = [
  { label: '从左到右', value: 'ltr' },
  { label: '从右到左', value: 'rtl' },
]

const animationOptions: SelectOption[] = [
  { label: '无', value: 'none' },
  { label: '滑动', value: 'slide' },
  { label: '淡入淡出', value: 'fade' },
]

const message = useMessage()
const status = ref('')
const error = ref('')
const settings = ref<ReaderSettings>({ ...defaultSettings })

function clampNumber(value: unknown, min: number, max: number, fallback: number) {
  const numeric = Number(value)
  if (!Number.isFinite(numeric)) return fallback
  return Math.min(max, Math.max(min, numeric))
}

function normalizeSettings(value: ReaderSettings): ReaderSettings {
  return {
    ...defaultSettings,
    ...value,
    spaceScrollRatio: clampNumber(value.spaceScrollRatio, 0.1, 2, defaultSettings.spaceScrollRatio),
    spaceHoldSpeedRatio: clampNumber(value.spaceHoldSpeedRatio, 0.5, 10, defaultSettings.spaceHoldSpeedRatio),
    brightness: clampNumber(value.brightness, 0.2, 2, defaultSettings.brightness),
    contrast: clampNumber(value.contrast, 0.2, 2, defaultSettings.contrast),
    pageAnimation: ['none', 'slide', 'fade'].includes(value.pageAnimation) ? value.pageAnimation : 'none',
    preloadCacheLimit: Math.round(clampNumber(value.preloadCacheLimit, 0, 500, defaultSettings.preloadCacheLimit)),
  }
}

async function loadSettings() {
  error.value = ''
  try {
    status.value = await ping()
    settings.value = normalizeSettings(await getReaderSettings())
  } catch (innerError) {
    error.value = String(innerError)
    status.value = String(innerError)
  }
}

async function save() {
  error.value = ''
  try {
    settings.value = normalizeSettings(settings.value)
    await saveReaderSettings(settings.value)
    message.success('设置已保存')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

onMounted(loadSettings)
</script>

<template>
  <section class="page-section">
    <NPageHeader>
      <template #title>设置</template>
      <template #subtitle>配置默认阅读体验。</template>
      <template #extra>
        <NButton type="primary" @click="save">保存设置</NButton>
      </template>
    </NPageHeader>

    <NAlert v-if="error" type="error" class="state-block" :show-icon="false">
      {{ error }}
    </NAlert>

    <NCard title="阅读偏好" :bordered="false">
      <NForm label-placement="top">
        <NGrid :cols="3" :x-gap="18" :y-gap="8" responsive="screen">
          <NGridItem>
            <NFormItem label="阅读模式" path="mode">
              <NSelect v-model:value="settings.mode" :options="modeOptions" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="图片适配" path="fit">
              <NSelect v-model:value="settings.fit" :options="fitOptions" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="阅读方向" path="direction">
              <NSelect v-model:value="settings.direction" :options="directionOptions" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="翻页动画" path="pageAnimation">
              <NSelect v-model:value="settings.pageAnimation" :options="animationOptions" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="阅读背景" path="background">
              <NColorPicker v-model:value="settings.background" :show-alpha="false" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="亮度" path="brightness">
              <NInputNumber v-model:value="settings.brightness" :min="0.2" :max="2" :step="0.05" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="对比度" path="contrast">
              <NInputNumber v-model:value="settings.contrast" :min="0.2" :max="2" :step="0.05" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="Space 单次滚动比例" path="spaceScrollRatio">
              <NInputNumber v-model:value="settings.spaceScrollRatio" :min="0.1" :max="2" :step="0.01" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="Space 长按速度" path="spaceHoldSpeedRatio">
              <NInputNumber v-model:value="settings.spaceHoldSpeedRatio" :min="0.5" :max="10" :step="0.1" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="预加载缓存上限" path="preloadCacheLimit">
              <NInputNumber v-model:value="settings.preloadCacheLimit" :min="0" :max="500" :step="10" />
            </NFormItem>
          </NGridItem>
        </NGrid>
      </NForm>
    </NCard>

    <NText depth="3" class="settings-status">Rust command: {{ status || '检查中...' }}</NText>
  </section>
</template>
