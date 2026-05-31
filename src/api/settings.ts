import { call, type LibraryViewSettings, type ReaderSettings } from './tauri'

export function ping(): Promise<string> {
  return call('ping')
}

export function getReaderSettings(): Promise<ReaderSettings> {
  return call('get_reader_settings')
}

export function saveReaderSettings(settings: ReaderSettings): Promise<void> {
  return call('save_reader_settings', { settings })
}

export function getLibraryViewSettings(): Promise<LibraryViewSettings> {
  return call('get_library_view_settings')
}

export function saveLibraryViewSettings(settings: LibraryViewSettings): Promise<void> {
  return call('save_library_view_settings', { settings })
}
