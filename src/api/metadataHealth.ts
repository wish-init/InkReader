import { call, type MetadataHealthSummary } from './tauri'

export function listMetadataHealth(): Promise<MetadataHealthSummary> {
  return call('list_metadata_health')
}
