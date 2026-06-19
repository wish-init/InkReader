import type { LibraryViewSettings } from '@/api/tauri'

export type LibraryViewPresetId = 'coverWall' | 'compactManagement' | 'listManagement'

export type LibraryViewSettingsPreset = {
  id: LibraryViewPresetId
  label: string
  settings: Partial<LibraryViewSettings>
}

export const libraryLayoutOptions = ['grid', 'compact', 'list'] as const
export const libraryCoverSizeOptions = ['small', 'medium', 'large'] as const

export const libraryLayoutSelectOptions = [
  { label: '网格', value: 'grid' },
  { label: '紧凑网格', value: 'compact' },
  { label: '列表', value: 'list' },
]

export const libraryCoverSizeSelectOptions = [
  { label: '小', value: 'small' },
  { label: '中', value: 'medium' },
  { label: '大', value: 'large' },
]

export const tagLimitOptions = [
  { label: '不显示', value: 0 },
  { label: '2 个', value: 2 },
  { label: '4 个', value: 4 },
  { label: '8 个', value: 8 },
  { label: '全部', value: 999 },
]

export const defaultLibraryViewSettings: LibraryViewSettings = {
  layout: 'grid',
  coverSize: 'medium',
  showAuthors: true,
  showTags: true,
  tagLimit: 4,
  titleLineClamp: 2,
  titleFontSize: 15,
}

export const libraryViewSettingsPresets: LibraryViewSettingsPreset[] = [
  {
    id: 'coverWall',
    label: '封面墙',
    settings: {
      layout: 'grid',
      coverSize: 'large',
      showAuthors: false,
      showTags: false,
      tagLimit: 0,
      titleLineClamp: 2,
      titleFontSize: 15,
    },
  },
  {
    id: 'compactManagement',
    label: '紧凑整理',
    settings: {
      layout: 'compact',
      coverSize: 'small',
      showAuthors: true,
      showTags: true,
      tagLimit: 2,
      titleLineClamp: 1,
      titleFontSize: 13,
    },
  },
  {
    id: 'listManagement',
    label: '列表管理',
    settings: {
      layout: 'list',
      coverSize: 'medium',
      showAuthors: true,
      showTags: true,
      tagLimit: 8,
      titleLineClamp: 2,
      titleFontSize: 15,
    },
  },
]

export const titleLineClampOptions = [
  { label: '1 行', value: 1 },
  { label: '2 行', value: 2 },
  { label: '3 行', value: 3 },
  { label: '4 行', value: 4 },
]

export const titleFontSizeOptions = [
  { label: '小 13px', value: 13 },
  { label: '默认 15px', value: 15 },
  { label: '大 17px', value: 17 },
  { label: '特大 19px', value: 19 },
]

export function normalizeTitleLineClamp(value: number) {
  return titleLineClampOptions.some((option) => option.value === value)
    ? value
    : defaultLibraryViewSettings.titleLineClamp
}

export function normalizeTitleFontSize(value: number) {
  return titleFontSizeOptions.some((option) => option.value === value)
    ? value
    : defaultLibraryViewSettings.titleFontSize
}

export function normalizeTagLimit(value: number) {
  return tagLimitOptions.some((option) => option.value === value)
    ? value
    : defaultLibraryViewSettings.tagLimit
}

export function normalizeLibraryViewSettings(
  value?: Partial<LibraryViewSettings> | null,
): LibraryViewSettings {
  const nextValue = value ?? {}
  return {
    ...defaultLibraryViewSettings,
    ...nextValue,
    layout: libraryLayoutOptions.includes(nextValue.layout as LibraryViewSettings['layout'])
      ? nextValue.layout as LibraryViewSettings['layout']
      : defaultLibraryViewSettings.layout,
    coverSize: libraryCoverSizeOptions.includes(nextValue.coverSize as LibraryViewSettings['coverSize'])
      ? nextValue.coverSize as LibraryViewSettings['coverSize']
      : defaultLibraryViewSettings.coverSize,
    showAuthors: typeof nextValue.showAuthors === 'boolean'
      ? nextValue.showAuthors
      : defaultLibraryViewSettings.showAuthors,
    showTags: typeof nextValue.showTags === 'boolean'
      ? nextValue.showTags
      : defaultLibraryViewSettings.showTags,
    tagLimit: normalizeTagLimit(Number(nextValue.tagLimit)),
    titleLineClamp: normalizeTitleLineClamp(Number(nextValue.titleLineClamp)),
    titleFontSize: normalizeTitleFontSize(Number(nextValue.titleFontSize)),
  }
}

export function applyLibraryViewSettingsPreset(
  currentSettings: LibraryViewSettings,
  preset: LibraryViewSettingsPreset,
): LibraryViewSettings {
  return normalizeLibraryViewSettings({
    ...currentSettings,
    ...preset.settings,
  })
}
