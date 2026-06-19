import { ref } from 'vue'
import {
  clearBookReaderSettings,
  getEffectiveReaderSettingsState,
  getReaderSettings,
  saveBookReaderSettings,
  saveReaderSettings,
} from '@/api/settings'
import type { ReaderSettings } from '@/api/tauri'
import { defaultReaderSettings, normalizeReaderSettings } from '@/utils/readerSettings'

export function useReaderSettings() {
  const settingsReady = ref(false)
  const settings = ref<ReaderSettings>({ ...defaultReaderSettings })
  const hasBookReaderSettings = ref(false)
  const currentBookId = ref<string | null>(null)
  let readerSettingsSaveTimer = 0

  async function loadReaderSettings(bookId?: string) {
    settingsReady.value = false
    currentBookId.value = bookId ?? null
    if (bookId) {
      const effectiveState = await getEffectiveReaderSettingsState(bookId)
      hasBookReaderSettings.value = effectiveState.hasBookReaderSettings
      settings.value = normalizeReaderSettings(effectiveState.settings)
    } else {
      hasBookReaderSettings.value = false
      settings.value = normalizeReaderSettings(await getReaderSettings())
    }
    settingsReady.value = true
    return settings.value
  }

  async function saveFilterSettings() {
    try {
      await saveCurrentReaderSettings()
    } catch { /* ignore */ }
  }

  function queueReaderSettingsSave() {
    if (!settingsReady.value) return
    if (readerSettingsSaveTimer) window.clearTimeout(readerSettingsSaveTimer)
    readerSettingsSaveTimer = window.setTimeout(() => {
      readerSettingsSaveTimer = 0
      settings.value = normalizeReaderSettings(settings.value)
      saveCurrentReaderSettings().catch(() => undefined)
    }, 250)
  }

  async function flushReaderSettings() {
    if (readerSettingsSaveTimer) {
      window.clearTimeout(readerSettingsSaveTimer)
      readerSettingsSaveTimer = 0
    }
    if (!settingsReady.value) return
    settings.value = normalizeReaderSettings(settings.value)
    await saveCurrentReaderSettings()
  }

  function resetFilter() {
    settings.value.brightness = 1
    settings.value.contrast = 1
    void saveFilterSettings()
  }

  async function saveCurrentReaderSettings() {
    const normalizedSettings = normalizeReaderSettings(settings.value)
    settings.value = normalizedSettings
    if (hasBookReaderSettings.value && currentBookId.value) {
      await saveBookReaderSettings(currentBookId.value, normalizedSettings)
      return
    }

    await saveReaderSettings(normalizedSettings)
  }

  async function saveBookOverride() {
    if (!currentBookId.value) return
    settings.value = normalizeReaderSettings(settings.value)
    await saveBookReaderSettings(currentBookId.value, settings.value)
    hasBookReaderSettings.value = true
  }

  async function clearBookOverride() {
    if (!currentBookId.value) return
    await clearBookReaderSettings(currentBookId.value)
    hasBookReaderSettings.value = false
    settings.value = normalizeReaderSettings(await getReaderSettings())
  }

  return {
    settings,
    settingsReady,
    hasBookReaderSettings,
    loadReaderSettings,
    saveFilterSettings,
    queueReaderSettingsSave,
    flushReaderSettings,
    resetFilter,
    saveBookOverride,
    clearBookOverride,
  }
}
