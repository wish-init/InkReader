import { copyFileSync, existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { readFile } from 'node:fs/promises'
import { basename, join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

const root = resolve(import.meta.dirname, '..')
const packageJson = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
const tauriConfig = JSON.parse(await readFile(join(root, 'src-tauri', 'tauri.conf.json'), 'utf8'))

const productName = tauriConfig.productName || 'InkReader'
const version = tauriConfig.version || packageJson.version || '0.0.0'
const platform = process.platform === 'win32' ? 'windows' : process.platform
const arch = process.arch === 'x64' ? 'x64' : process.arch
const releaseDir = join(root, 'src-tauri', 'target', 'release')
const exeName = process.platform === 'win32' ? `${productName}.exe` : productName
const executablePath = join(releaseDir, exeName)
const portableRoot = join(releaseDir, 'portable')
const portableName = `${productName}-portable-${version}-${platform}-${arch}`
const stagingDir = join(portableRoot, portableName)
const zipPath = join(portableRoot, `${portableName}.zip`)

if (!existsSync(executablePath)) {
  console.error(`[InkReader] Release executable not found: ${executablePath}`)
  console.error('[InkReader] Run `pnpm tauri:build:portable` so Tauri builds the executable before packaging.')
  process.exit(1)
}

rmSync(stagingDir, { recursive: true, force: true })
rmSync(zipPath, { force: true })
mkdirSync(stagingDir, { recursive: true })
mkdirSync(join(stagingDir, 'data'), { recursive: true })

copyFileSync(executablePath, join(stagingDir, basename(executablePath)))
writeFileSync(
  join(stagingDir, 'README-portable.txt'),
  [
    `${productName} portable package`,
    '',
    'Run InkReader.exe directly from this folder.',
    'Application data is stored in ./data/inkreader.sqlite3 next to the executable.',
    'Keep this folder in a writable location. To keep data off C:, extract this folder to another drive.',
    'Windows WebView2 Runtime must be installed on the system for Tauri apps to run.',
    '',
  ].join('\n'),
  'utf8',
)

if (process.platform !== 'win32') {
  console.log(`[InkReader] Portable folder created: ${stagingDir}`)
  console.log('[InkReader] Zip creation is currently implemented with PowerShell Compress-Archive on Windows.')
  process.exit(0)
}

const archiveCommand = [
  '-NoProfile',
  '-Command',
  `Compress-Archive -LiteralPath '${stagingDir.replaceAll("'", "''")}' -DestinationPath '${zipPath.replaceAll("'", "''")}' -Force`,
]
const result = spawnSync('powershell.exe', archiveCommand, {
  encoding: 'utf8',
  stdio: 'pipe',
})

if (result.error || result.status !== 0) {
  console.error('[InkReader] Failed to create portable zip with PowerShell Compress-Archive.')
  if (result.error) console.error(result.error.message)
  if (result.stderr) console.error(result.stderr.trim())
  console.error(`[InkReader] Portable folder is still available: ${stagingDir}`)
  process.exit(1)
}

console.log(`[InkReader] Portable folder created: ${stagingDir}`)
console.log(`[InkReader] Portable zip created: ${zipPath}`)
