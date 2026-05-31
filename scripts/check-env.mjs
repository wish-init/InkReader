import { spawnSync } from 'node:child_process'

const checks = [
  {
    name: 'Node.js',
    command: 'node',
    args: ['--version'],
    install: 'Install Node.js LTS from https://nodejs.org/',
  },
  {
    name: 'pnpm',
    command: 'pnpm',
    args: ['--version'],
    install: 'Enable Corepack with `corepack enable`, or install pnpm from https://pnpm.io/installation',
  },
  {
    name: 'Rust compiler',
    command: 'rustc',
    args: ['--version'],
    install: 'Install Rust with `winget install Rustlang.Rustup`, then reopen PowerShell.',
  },
  {
    name: 'Cargo',
    command: 'cargo',
    args: ['--version'],
    install: 'Install Rust with `winget install Rustlang.Rustup`, then reopen PowerShell.',
  },
]

let hasError = false

for (const check of checks) {
  const result = spawnSync(check.command, check.args, {
    encoding: 'utf8',
    shell: process.platform === 'win32',
  })

  if (result.error || result.status !== 0) {
    hasError = true
    console.error(`\n[InkReader] Missing ${check.name}.`)
    console.error(`  Command: ${check.command} ${check.args.join(' ')}`)
    console.error(`  Fix: ${check.install}`)
    continue
  }

  const version = (result.stdout || result.stderr).trim()
  console.log(`[InkReader] ${check.name}: ${version}`)
}

if (hasError) {
  console.error('\n[InkReader] Environment check failed. Fix the missing tools above and run the command again.')
  process.exit(1)
}
