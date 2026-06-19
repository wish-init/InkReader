import { ref, type Ref } from 'vue'
import { getLibraryViewSettings, saveLibraryViewSettings } from '@/api/settings'
import type { LibraryViewSettings } from '@/api/tauri'
import {
  defaultLibraryViewSettings,
  normalizeLibraryViewSettings,
} from '@/utils/libraryViewSettings'

type UseLibraryViewSettingsOptions = {
  error: Ref<string>
  onSaveSuccess?: (message: string) => void
}

export function useLibraryViewSettings(options: UseLibraryViewSettingsOptions) {
  const viewSettings = ref<LibraryViewSettings>({ ...defaultLibraryViewSettings })

  async function loadLibraryViewSettings() {
    const nextSettings = await getLibraryViewSettings()
    viewSettings.value = normalizeLibraryViewSettings(nextSettings)
    return viewSettings.value
  }

  async function saveViewSettings() {
    options.error.value = ''
    try {
      const normalizedSettings = normalizeLibraryViewSettings(viewSettings.value)
      viewSettings.value = normalizedSettings
      await saveLibraryViewSettings(normalizedSettings)
      options.onSaveSuccess?.('显示设置已保存')
    } catch (innerError) {
      options.error.value = String(innerError)
      throw innerError
    }
  }

  return {
    viewSettings,
    loadLibraryViewSettings,
    saveViewSettings,
  }
}
