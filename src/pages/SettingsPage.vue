<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { open, save as saveDialog } from '@tauri-apps/plugin-dialog'
import { NAlert, NButton, NCard, NColorPicker, NForm, NFormItem, NGrid, NGridItem, NInput, NInputNumber, NPageHeader, NPopconfirm, NSelect, NSpace, NSwitch, NText, useMessage } from 'naive-ui'
import { createDatabaseBackup, restoreDatabaseBackup } from '@/api/backup'
import { cleanupThumbnailCache, getCacheMaintenanceSummary, rebuildMissingThumbnails } from '@/api/cache'
import { exportSettings, getLibraryViewSettings, getReaderSettings, importSettingsExport, ping, restoreDefaultSettings, saveLibraryViewSettings, saveReaderSettings } from '@/api/settings'
import type { CacheMaintenanceResult, CacheMaintenanceSummary, DatabaseBackupResult, DatabaseRestoreResult, LibraryViewSettings, ReaderSettings, SettingsExport, SettingsRestoreScope } from '@/api/tauri'
import LibraryViewSettingsPanel from '@/components/library/LibraryViewSettingsPanel.vue'
import {
  applyLibraryViewSettingsPreset,
  defaultLibraryViewSettings,
  libraryViewSettingsPresets,
  normalizeLibraryViewSettings,
  type LibraryViewSettingsPreset,
} from '@/utils/libraryViewSettings'
import {
  applyReaderSettingsPreset,
  defaultReaderSettings,
  normalizeReaderSettings,
  readerDirectionSelectOptions,
  readerFitSelectOptions,
  readerModeSelectOptions,
  readerPageAnimationSelectOptions,
  readerSettingRanges,
  readerSettingsPresets,
  type ReaderSettingsPreset,
} from '@/utils/readerSettings'

const message = useMessage()
const status = ref('')
const error = ref('')
const settings = ref<ReaderSettings>({ ...defaultReaderSettings })
const libraryViewSettings = ref<LibraryViewSettings>({ ...defaultLibraryViewSettings })
const exportedSettingsJson = ref('')
const importSettingsJson = ref('')
const cacheSummary = ref<CacheMaintenanceSummary | null>(null)
const cacheResult = ref<CacheMaintenanceResult | null>(null)
const cacheLoading = ref(false)
const backupLoading = ref(false)
const backupResult = ref<DatabaseBackupResult | null>(null)
const restoreResult = ref<DatabaseRestoreResult | null>(null)
const selectedRestorePath = ref('')

async function loadSettings() {
  error.value = ''
  try {
    status.value = await ping()
    settings.value = normalizeReaderSettings(await getReaderSettings())
    libraryViewSettings.value = normalizeLibraryViewSettings(await getLibraryViewSettings())
    await refreshExport()
    await refreshCacheSummary()
  } catch (innerError) {
    error.value = String(innerError)
    status.value = String(innerError)
  }
}

async function save() {
  error.value = ''
  try {
    settings.value = normalizeReaderSettings(settings.value)
    libraryViewSettings.value = normalizeLibraryViewSettings(libraryViewSettings.value)
    await saveReaderSettings(settings.value)
    await saveLibraryViewSettings(libraryViewSettings.value)
    await refreshExport()
    message.success('设置已保存')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

function applySettingsExport(settingsExport: SettingsExport) {
  settings.value = normalizeReaderSettings(settingsExport.reader)
  libraryViewSettings.value = normalizeLibraryViewSettings(settingsExport.libraryView)
  exportedSettingsJson.value = JSON.stringify(settingsExport, null, 2)
}

async function refreshExport() {
  applySettingsExport(await exportSettings())
}

async function importFromJson() {
  error.value = ''
  const settingsJson = importSettingsJson.value.trim()
  if (!settingsJson) {
    error.value = '请粘贴设置 JSON'
    return
  }

  try {
    applySettingsExport(await importSettingsExport(settingsJson))
    importSettingsJson.value = ''
    message.success('设置已导入')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function restoreDefaults(scope: SettingsRestoreScope) {
  error.value = ''
  try {
    applySettingsExport(await restoreDefaultSettings(scope))
    message.success('默认设置已恢复')
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function applyReaderPreset(preset: ReaderSettingsPreset) {
  error.value = ''
  try {
    settings.value = applyReaderSettingsPreset(settings.value, preset)
    await saveReaderSettings(settings.value)
    await refreshExport()
    message.success(`已应用${preset.label}阅读预设`)
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function applyLibraryViewPreset(preset: LibraryViewSettingsPreset) {
  error.value = ''
  try {
    libraryViewSettings.value = applyLibraryViewSettingsPreset(libraryViewSettings.value, preset)
    await saveLibraryViewSettings(libraryViewSettings.value)
    await refreshExport()
    message.success(`已应用${preset.label}书架预设`)
  } catch (innerError) {
    error.value = String(innerError)
  }
}

async function refreshCacheSummary() {
  cacheSummary.value = await getCacheMaintenanceSummary()
}

async function runCacheCleanup() {
  error.value = ''
  cacheLoading.value = true
  try {
    cacheResult.value = await cleanupThumbnailCache()
    await refreshCacheSummary()
    message.success(cacheResultSummary(cacheResult.value))
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    cacheLoading.value = false
  }
}

async function runThumbnailRebuild() {
  error.value = ''
  cacheLoading.value = true
  try {
    cacheResult.value = await rebuildMissingThumbnails()
    await refreshCacheSummary()
    message.success(cacheResultSummary(cacheResult.value))
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    cacheLoading.value = false
  }
}

function cacheResultSummary(result: CacheMaintenanceResult) {
  if (result.operation === 'cleanupThumbnailCache') {
    return `已清理 ${result.removedFiles} 个缩略图缓存，失败 ${result.failed.length} 项`
  }
  return `已重建 ${result.rebuiltThumbnails} 个缩略图，失败 ${result.failed.length} 项`
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`
  return `${(value / 1024 / 1024).toFixed(1)} MB`
}

async function runDatabaseBackup() {
  error.value = ''
  const backupPath = await saveDialog({
    title: '保存数据库备份',
    defaultPath: `inkreader-backup-${new Date().toISOString().slice(0, 10)}.sqlite3`,
    filters: [{ name: 'SQLite 数据库', extensions: ['sqlite3', 'db'] }],
  })
  if (!backupPath) return

  backupLoading.value = true
  try {
    backupResult.value = await createDatabaseBackup(backupPath)
    message.success(`数据库备份已创建：${backupResult.value.backupPath}`)
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    backupLoading.value = false
  }
}

async function chooseRestoreBackup() {
  error.value = ''
  const selected = await open({
    title: '选择数据库备份',
    multiple: false,
    filters: [{ name: 'SQLite 数据库', extensions: ['sqlite3', 'db'] }],
  })
  if (!selected || Array.isArray(selected)) return
  selectedRestorePath.value = selected
}

async function runDatabaseRestore() {
  if (!selectedRestorePath.value) {
    error.value = '请先选择数据库备份文件'
    return
  }

  error.value = ''
  backupLoading.value = true
  try {
    restoreResult.value = await restoreDatabaseBackup(selectedRestorePath.value)
    await loadSettings()
    message.success('数据库已恢复，当前设置已重新加载')
  } catch (innerError) {
    error.value = String(innerError)
  } finally {
    backupLoading.value = false
  }
}

onMounted(loadSettings)
</script>

<template>
  <section class="page-section">
    <NPageHeader>
      <template #title>设置</template>
      <template #subtitle>配置默认阅读体验和书架显示。</template>
      <template #extra>
        <NButton type="primary" @click="save">保存设置</NButton>
      </template>
    </NPageHeader>

    <NAlert v-if="error" type="error" class="state-block" :show-icon="false">
      {{ error }}
    </NAlert>

    <NCard title="阅读偏好" :bordered="false">
      <NForm label-placement="top">
        <NFormItem label="阅读预设">
          <NSpace>
            <NButton
              v-for="preset in readerSettingsPresets"
              :key="preset.id"
              size="small"
              @click="applyReaderPreset(preset)"
            >
              {{ preset.label }}
            </NButton>
          </NSpace>
        </NFormItem>

        <NGrid :cols="3" :x-gap="18" :y-gap="8" responsive="screen">
          <NGridItem>
            <NFormItem label="阅读模式" path="mode">
              <NSelect v-model:value="settings.mode" :options="readerModeSelectOptions" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="图片适配" path="fit">
              <NSelect v-model:value="settings.fit" :options="readerFitSelectOptions" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="阅读方向" path="direction">
              <NSelect v-model:value="settings.direction" :options="readerDirectionSelectOptions" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="翻页动画" path="pageAnimation">
              <NSelect v-model:value="settings.pageAnimation" :options="readerPageAnimationSelectOptions" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="阅读背景" path="background">
              <NColorPicker v-model:value="settings.background" :show-alpha="false" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="亮度" path="brightness">
              <NInputNumber v-model:value="settings.brightness" :min="readerSettingRanges.brightness.min" :max="readerSettingRanges.brightness.max" :step="0.05" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="对比度" path="contrast">
              <NInputNumber v-model:value="settings.contrast" :min="readerSettingRanges.contrast.min" :max="readerSettingRanges.contrast.max" :step="0.05" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="Space 单次滚动比例" path="spaceScrollRatio">
              <NInputNumber v-model:value="settings.spaceScrollRatio" :min="readerSettingRanges.spaceScrollRatio.min" :max="readerSettingRanges.spaceScrollRatio.max" :step="0.01" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="Space 长按速度" path="spaceHoldSpeedRatio">
              <NInputNumber v-model:value="settings.spaceHoldSpeedRatio" :min="readerSettingRanges.spaceHoldSpeedRatio.min" :max="readerSettingRanges.spaceHoldSpeedRatio.max" :step="0.1" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="自动滚动速度" path="autoScrollSpeed">
              <NInputNumber v-model:value="settings.autoScrollSpeed" :min="readerSettingRanges.autoScrollSpeed.min" :max="readerSettingRanges.autoScrollSpeed.max" :step="10" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="自动滚动启动延迟" path="autoScrollStartDelay">
              <NInputNumber v-model:value="settings.autoScrollStartDelay" :min="readerSettingRanges.autoScrollStartDelay.min" :max="readerSettingRanges.autoScrollStartDelay.max" :step="0.5" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="手动操作停止自动滚动" path="autoScrollStopOnManualScroll">
              <NSwitch v-model:value="settings.autoScrollStopOnManualScroll" />
            </NFormItem>
          </NGridItem>

          <NGridItem>
            <NFormItem label="预加载缓存上限" path="preloadCacheLimit">
              <NInputNumber v-model:value="settings.preloadCacheLimit" :min="readerSettingRanges.preloadCacheLimit.min" :max="readerSettingRanges.preloadCacheLimit.max" :step="10" />
            </NFormItem>
          </NGridItem>
        </NGrid>
      </NForm>
    </NCard>

    <NCard title="书架显示" :bordered="false">
      <div class="settings-preset-group">
        <NText depth="3">书架预设</NText>
        <NSpace>
          <NButton
            v-for="preset in libraryViewSettingsPresets"
            :key="preset.id"
            size="small"
            @click="applyLibraryViewPreset(preset)"
          >
            {{ preset.label }}
          </NButton>
        </NSpace>
      </div>

      <LibraryViewSettingsPanel v-model="libraryViewSettings" />
    </NCard>

    <NCard title="缓存维护" :bordered="false">
      <NForm label-placement="top">
        <NGrid :cols="4" :x-gap="18" :y-gap="8" responsive="screen">
          <NGridItem>
            <NFormItem label="缩略图缓存文件">
              <NText strong>{{ cacheSummary?.thumbnailFiles ?? 0 }}</NText>
            </NFormItem>
          </NGridItem>
          <NGridItem>
            <NFormItem label="缩略图缓存大小">
              <NText strong>{{ formatBytes(cacheSummary?.thumbnailBytes ?? 0) }}</NText>
            </NFormItem>
          </NGridItem>
          <NGridItem>
            <NFormItem label="已关联缩略图">
              <NText strong>{{ cacheSummary?.booksWithThumbnails ?? 0 }}</NText>
            </NFormItem>
          </NGridItem>
          <NGridItem>
            <NFormItem label="可重建缩略图">
              <NText strong>{{ cacheSummary?.missingThumbnails ?? 0 }}</NText>
            </NFormItem>
          </NGridItem>
        </NGrid>

        <NFormItem label="缓存目录">
          <NText depth="3">{{ cacheSummary?.thumbnailCacheDir || '检查中...' }}</NText>
        </NFormItem>

        <NSpace>
          <NPopconfirm @positive-click="runCacheCleanup">
            <template #trigger>
              <NButton type="warning" :loading="cacheLoading">清理缩略图缓存</NButton>
            </template>
            只会删除 InkReader 管理的缩略图缓存，并清空对应数据库引用；不会删除或修改本地原始漫画文件。
          </NPopconfirm>

          <NPopconfirm @positive-click="runThumbnailRebuild">
            <template #trigger>
              <NButton :loading="cacheLoading">重建缺失缩略图</NButton>
            </template>
            将从已有封面重新生成缺失缩略图；不会删除或修改本地原始漫画文件。
          </NPopconfirm>

          <NButton :loading="cacheLoading" @click="refreshCacheSummary">刷新缓存统计</NButton>
        </NSpace>

        <NAlert v-if="cacheResult" type="info" class="cache-result-block" :show-icon="false">
          {{ cacheResultSummary(cacheResult) }}。原始漫画文件影响：{{ cacheResult.sourceFilesAffected ? '是' : '否' }}。
          <template v-if="cacheResult.failed.length">
            失败明细：{{ cacheResult.failed.map((item) => item.title || item.path).join('、') }}
          </template>
        </NAlert>
      </NForm>
    </NCard>

    <NCard title="数据库备份与恢复" :bordered="false">
      <NForm label-placement="top">
        <NSpace>
          <NButton type="primary" :loading="backupLoading" @click="runDatabaseBackup">
            创建数据库备份
          </NButton>
          <NButton :loading="backupLoading" @click="chooseRestoreBackup">
            选择恢复文件
          </NButton>
          <NPopconfirm @positive-click="runDatabaseRestore">
            <template #trigger>
              <NButton type="warning" :disabled="!selectedRestorePath" :loading="backupLoading">
                恢复数据库
              </NButton>
            </template>
            恢复前会校验备份文件并保留当前数据库回滚副本；不会删除或修改本地原始漫画文件。恢复后当前 InkReader 数据会替换为备份内容。
          </NPopconfirm>
        </NSpace>

        <NFormItem label="待恢复文件">
          <NText depth="3">{{ selectedRestorePath || '尚未选择' }}</NText>
        </NFormItem>

        <NAlert v-if="backupResult" type="success" class="maintenance-result-block" :show-icon="false">
          备份已创建：{{ backupResult.backupPath }}，大小 {{ formatBytes(backupResult.bytes) }}。原始漫画文件影响：{{ backupResult.sourceFilesAffected ? '是' : '否' }}。
        </NAlert>

        <NAlert v-if="restoreResult" type="info" class="maintenance-result-block" :show-icon="false">
          已从 {{ restoreResult.restoredFrom }} 恢复数据库。回滚副本：{{ restoreResult.rollbackPath }}。原始漫画文件影响：{{ restoreResult.sourceFilesAffected ? '是' : '否' }}。
        </NAlert>
      </NForm>
    </NCard>

    <NCard title="设置管理" :bordered="false">
      <NForm label-placement="top">
        <NGrid :cols="2" :x-gap="18" :y-gap="8" responsive="screen">
          <NGridItem>
            <NFormItem label="导出 JSON">
              <NInput
                v-model:value="exportedSettingsJson"
                type="textarea"
                readonly
                :autosize="{ minRows: 8, maxRows: 16 }"
              />
            </NFormItem>
            <NButton @click="refreshExport">刷新导出</NButton>
          </NGridItem>

          <NGridItem>
            <NFormItem label="导入 JSON">
              <NInput
                v-model:value="importSettingsJson"
                type="textarea"
                :autosize="{ minRows: 8, maxRows: 16 }"
              />
            </NFormItem>
            <NPopconfirm @positive-click="importFromJson">
              <template #trigger>
                <NButton type="primary">导入设置</NButton>
              </template>
              导入后会替换当前阅读和书架显示设置。
            </NPopconfirm>
          </NGridItem>
        </NGrid>

        <NSpace class="settings-actions">
          <NPopconfirm @positive-click="restoreDefaults('reader')">
            <template #trigger>
              <NButton>恢复阅读默认</NButton>
            </template>
            恢复阅读设置默认值？
          </NPopconfirm>

          <NPopconfirm @positive-click="restoreDefaults('libraryView')">
            <template #trigger>
              <NButton>恢复书架默认</NButton>
            </template>
            恢复书架显示设置默认值？
          </NPopconfirm>

          <NPopconfirm @positive-click="restoreDefaults('all')">
            <template #trigger>
              <NButton type="warning">恢复全部默认</NButton>
            </template>
            恢复全部设置默认值？
          </NPopconfirm>
        </NSpace>
      </NForm>
    </NCard>

    <NText depth="3" class="settings-status">Rust command: {{ status || '检查中...' }}</NText>
  </section>
</template>

<style scoped>
.maintenance-result-block,
.cache-result-block {
  margin-top: 16px;
}
</style>
