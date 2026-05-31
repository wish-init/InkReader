import { call, type Repository, type RepositoryScanResult } from './tauri'

export function listRepositories(): Promise<Repository[]> {
  return call('list_repositories')
}

export function scanRepository(path: string): Promise<RepositoryScanResult> {
  return call('scan_repository', { path })
}

export function autoScanRepositories(): Promise<RepositoryScanResult[]> {
  return call('auto_scan_repositories')
}

export function removeRepository(repositoryId: string): Promise<void> {
  return call('remove_repository', { repositoryId })
}
