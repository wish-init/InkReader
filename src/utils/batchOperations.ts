export type BatchOperationItem = {
  path: string
  title: string
}

export type BatchOperationFailure = BatchOperationItem & {
  reason: string
}

export type BatchOperationResult = {
  operation: string
  total: number
  succeeded: number
  failed: BatchOperationFailure[]
  sourceFilesAffected: false
}

export const sourceFilesSafeNotice = '不会删除或修改本地原始漫画文件。'

export function batchConfirmationMessage(action: string, count: number, target?: string) {
  const targetText = target ? `到「${target}」` : ''
  return `确定要${action}${targetText}当前页选中的 ${count} 本漫画吗？${sourceFilesSafeNotice}`
}

export function createBatchSuccessResult(operation: string, items: BatchOperationItem[]): BatchOperationResult {
  return {
    operation,
    total: items.length,
    succeeded: items.length,
    failed: [],
    sourceFilesAffected: false,
  }
}

export function createBatchFailureResult(
  operation: string,
  items: BatchOperationItem[],
  reason: string,
): BatchOperationResult {
  return {
    operation,
    total: items.length,
    succeeded: 0,
    failed: items.map((item) => ({ ...item, reason })),
    sourceFilesAffected: false,
  }
}

export function createBatchPartialResult(
  operation: string,
  items: BatchOperationItem[],
  failed: BatchOperationFailure[],
): BatchOperationResult {
  return {
    operation,
    total: items.length,
    succeeded: Math.max(0, items.length - failed.length),
    failed,
    sourceFilesAffected: false,
  }
}

export function batchResultSummary(result: BatchOperationResult) {
  if (result.failed.length) {
    return `${result.operation}完成 ${result.succeeded} / ${result.total} 本，失败 ${result.failed.length} 本`
  }
  return `${result.operation} ${result.succeeded} 本漫画`
}
