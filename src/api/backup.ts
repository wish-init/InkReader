import { call, type DatabaseBackupResult, type DatabaseRestoreResult } from './tauri'

export function createDatabaseBackup(backupPath: string): Promise<DatabaseBackupResult> {
  return call('create_database_backup', { backupPath })
}

export function restoreDatabaseBackup(backupPath: string): Promise<DatabaseRestoreResult> {
  return call('restore_database_backup', { backupPath })
}
