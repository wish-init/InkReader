import { call, type OperationLogRecord } from './tauri'

export function listOperationLogs(limit?: number): Promise<OperationLogRecord[]> {
  return call('list_operation_logs', { limit: limit ?? null })
}
