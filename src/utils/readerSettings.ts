import type { ReaderSettings } from '@/api/tauri'

export type ReaderPresetId = 'night' | 'eyeComfort' | 'rightToLeftManga' | 'scrollReading'

export type ReaderSettingsPreset = {
  id: ReaderPresetId
  label: string
  settings: Partial<ReaderSettings>
}

export const readerModeOptions = ['single', 'double', 'scroll'] as const
export const readerFitOptions = ['width', 'height', 'original'] as const
export const readerDirectionOptions = ['ltr', 'rtl'] as const
export const readerPageAnimationOptions = ['none', 'slide', 'fade'] as const

export const readerModeSelectOptions = [
  { label: '单页', value: 'single' },
  { label: '双页', value: 'double' },
  { label: '长条滚动', value: 'scroll' },
]

export const readerFitSelectOptions = [
  { label: '适应高度', value: 'height' },
  { label: '适应宽度', value: 'width' },
  { label: '原始尺寸', value: 'original' },
]

export const readerDirectionSelectOptions = [
  { label: '从左到右', value: 'ltr' },
  { label: '从右到左', value: 'rtl' },
]

export const readerPageAnimationSelectOptions = [
  { label: '无', value: 'none' },
  { label: '滑动', value: 'slide' },
  { label: '淡入淡出', value: 'fade' },
]

export const readerSettingRanges = {
  spaceScrollRatio: { min: 0.1, max: 2 },
  spaceHoldSpeedRatio: { min: 0.5, max: 10 },
  brightness: { min: 0.2, max: 2 },
  contrast: { min: 0.2, max: 2 },
  preloadCacheLimit: { min: 0, max: 500 },
  autoScrollSpeed: { min: 20, max: 400 },
  autoScrollStartDelay: { min: 0, max: 5 },
} as const

export const defaultReaderSettings: ReaderSettings = {
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
  autoScrollSpeed: 80,
  autoScrollStartDelay: 0,
  autoScrollStopOnManualScroll: true,
}

export const readerSettingsPresets: ReaderSettingsPreset[] = [
  {
    id: 'night',
    label: '夜间',
    settings: {
      background: '#050607',
      brightness: 0.85,
      contrast: 1.08,
      pageAnimation: 'none',
    },
  },
  {
    id: 'eyeComfort',
    label: '护眼',
    settings: {
      background: '#1f271f',
      brightness: 0.95,
      contrast: 0.92,
      pageAnimation: 'fade',
    },
  },
  {
    id: 'rightToLeftManga',
    label: '日漫右翻',
    settings: {
      mode: 'double',
      fit: 'height',
      direction: 'rtl',
      pageAnimation: 'slide',
    },
  },
  {
    id: 'scrollReading',
    label: '条漫滚动',
    settings: {
      mode: 'scroll',
      fit: 'width',
      direction: 'ltr',
      autoScrollSpeed: 80,
      autoScrollStartDelay: 0,
      autoScrollStopOnManualScroll: true,
    },
  },
]

export function clampNumber(value: unknown, min: number, max: number, fallback: number) {
  const numeric = Number(value)
  if (!Number.isFinite(numeric)) return fallback
  return Math.min(max, Math.max(min, numeric))
}

export function normalizeReaderSettings(value?: Partial<ReaderSettings> | null): ReaderSettings {
  const nextValue = value ?? {}
  return {
    ...defaultReaderSettings,
    ...nextValue,
    mode: readerModeOptions.includes(nextValue.mode as ReaderSettings['mode'])
      ? nextValue.mode as ReaderSettings['mode']
      : defaultReaderSettings.mode,
    fit: readerFitOptions.includes(nextValue.fit as ReaderSettings['fit'])
      ? nextValue.fit as ReaderSettings['fit']
      : defaultReaderSettings.fit,
    direction: readerDirectionOptions.includes(nextValue.direction as ReaderSettings['direction'])
      ? nextValue.direction as ReaderSettings['direction']
      : defaultReaderSettings.direction,
    spaceScrollRatio: clampNumber(
      nextValue.spaceScrollRatio,
      readerSettingRanges.spaceScrollRatio.min,
      readerSettingRanges.spaceScrollRatio.max,
      defaultReaderSettings.spaceScrollRatio,
    ),
    spaceHoldSpeedRatio: clampNumber(
      nextValue.spaceHoldSpeedRatio,
      readerSettingRanges.spaceHoldSpeedRatio.min,
      readerSettingRanges.spaceHoldSpeedRatio.max,
      defaultReaderSettings.spaceHoldSpeedRatio,
    ),
    brightness: clampNumber(
      nextValue.brightness,
      readerSettingRanges.brightness.min,
      readerSettingRanges.brightness.max,
      defaultReaderSettings.brightness,
    ),
    contrast: clampNumber(
      nextValue.contrast,
      readerSettingRanges.contrast.min,
      readerSettingRanges.contrast.max,
      defaultReaderSettings.contrast,
    ),
    pageAnimation: readerPageAnimationOptions.includes(nextValue.pageAnimation as ReaderSettings['pageAnimation'])
      ? nextValue.pageAnimation as ReaderSettings['pageAnimation']
      : defaultReaderSettings.pageAnimation,
    preloadCacheLimit: Math.round(clampNumber(
      nextValue.preloadCacheLimit,
      readerSettingRanges.preloadCacheLimit.min,
      readerSettingRanges.preloadCacheLimit.max,
      defaultReaderSettings.preloadCacheLimit,
    )),
    autoScrollSpeed: Math.round(clampNumber(
      nextValue.autoScrollSpeed,
      readerSettingRanges.autoScrollSpeed.min,
      readerSettingRanges.autoScrollSpeed.max,
      defaultReaderSettings.autoScrollSpeed,
    )),
    autoScrollStartDelay: clampNumber(
      nextValue.autoScrollStartDelay,
      readerSettingRanges.autoScrollStartDelay.min,
      readerSettingRanges.autoScrollStartDelay.max,
      defaultReaderSettings.autoScrollStartDelay,
    ),
    autoScrollStopOnManualScroll: typeof nextValue.autoScrollStopOnManualScroll === 'boolean'
      ? nextValue.autoScrollStopOnManualScroll
      : defaultReaderSettings.autoScrollStopOnManualScroll,
  }
}

export function applyReaderSettingsPreset(
  currentSettings: ReaderSettings,
  preset: ReaderSettingsPreset,
): ReaderSettings {
  return normalizeReaderSettings({
    ...currentSettings,
    ...preset.settings,
  })
}
