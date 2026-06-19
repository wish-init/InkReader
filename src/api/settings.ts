import {
  call,
  type EffectiveReaderSettingsState,
  type LibraryViewSettings,
  type ReaderSettings,
  type SettingsExport,
  type SettingsRestoreScope,
} from './tauri'

export function ping(): Promise<string> {
  return call('ping')
}

export function getReaderSettings(): Promise<ReaderSettings> {
  return call('get_reader_settings')
}

export function saveReaderSettings(settings: ReaderSettings): Promise<void> {
  return call('save_reader_settings', { settings })
}

export function getBookReaderSettings(bookId: string): Promise<ReaderSettings | null> {
  return call('get_book_reader_settings', { bookId })
}

export function getEffectiveReaderSettings(bookId: string): Promise<ReaderSettings> {
  return call('get_effective_reader_settings', { bookId })
}

export function getEffectiveReaderSettingsState(bookId: string): Promise<EffectiveReaderSettingsState> {
  return call('get_effective_reader_settings_state', { bookId })
}

export function saveBookReaderSettings(bookId: string, settings: ReaderSettings): Promise<void> {
  return call('save_book_reader_settings', { bookId, settings })
}

export function clearBookReaderSettings(bookId: string): Promise<void> {
  return call('clear_book_reader_settings', { bookId })
}

export function getLibraryViewSettings(): Promise<LibraryViewSettings> {
  return call('get_library_view_settings')
}

export function saveLibraryViewSettings(settings: LibraryViewSettings): Promise<void> {
  return call('save_library_view_settings', { settings })
}

export function exportSettings(): Promise<SettingsExport> {
  return call('export_settings')
}

export function importSettingsExport(settingsJson: string): Promise<SettingsExport> {
  return call('import_settings_export', { settingsJson })
}

export function restoreDefaultSettings(scope: SettingsRestoreScope): Promise<SettingsExport> {
  return call('restore_default_settings', { scope })
}
