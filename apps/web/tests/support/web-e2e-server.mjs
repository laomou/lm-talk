import { spawn } from 'node:child_process'
import { access, mkdtemp, rm } from 'node:fs/promises'
import { constants } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

const webRoot = resolve(import.meta.dirname, '../..')
const repoRoot = resolve(webRoot, '../..')
const releaseNodeBinary = join(repoRoot, 'target/release/lm_node')
const debugNodeBinary = join(repoRoot, 'target/debug/lm_node')
let nodeBinary = debugNodeBinary
const nodeUrl = 'http://127.0.0.1:8787'
const nodeToken = 'playwright-node-token'
let nodeProcess
let previewProcess
let nodeDataDir
let stopping = false

function run(command, args, options = {}) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(command, args, { stdio: 'inherit', ...options })
    child.once('error', rejectRun)
    child.once('exit', (code, signal) => {
      if (code === 0) resolveRun()
      else rejectRun(new Error(`${command} ${args.join(' ')} exited with ${signal || code}`))
    })
  })
}

async function waitForHealth() {
  const deadline = Date.now() + 30_000
  let lastError
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${nodeUrl}/api/health`)
      if (response.ok) return
      lastError = new Error(`health returned ${response.status}`)
    } catch (error) {
      lastError = error
    }
    await new Promise((resolveWait) => setTimeout(resolveWait, 100))
  }
  throw new Error(`lm_node did not become healthy: ${lastError instanceof Error ? lastError.message : String(lastError)}`)
}

async function stop() {
  if (stopping) return
  stopping = true
  for (const child of [previewProcess, nodeProcess]) {
    if (child && !child.killed) child.kill('SIGTERM')
  }
  if (nodeDataDir) await rm(nodeDataDir, { recursive: true, force: true })
}

for (const signal of ['SIGINT', 'SIGTERM', 'exit']) process.once(signal, stop)

try {
  await access(debugNodeBinary, constants.X_OK)
} catch {
  try {
    await access(releaseNodeBinary, constants.X_OK)
    nodeBinary = releaseNodeBinary
  } catch {
    await run('cargo', ['build', '--locked', '-p', 'lm_node'], { cwd: repoRoot })
  }
}

await run('npm', ['run', 'build'], { cwd: webRoot })
nodeDataDir = await mkdtemp(join(tmpdir(), 'lm-talk-playwright-node-'))
nodeProcess = spawn(nodeBinary, [
  'serve-control',
  '--bind', '127.0.0.1:8787',
  '--control-token', nodeToken,
  '--state-db', join(nodeDataDir, 'node.sqlite3'),
  '--cors-allow-origin', 'http://127.0.0.1:4173',
], { stdio: 'inherit' })
nodeProcess.once('exit', (code, signal) => {
  if (!stopping) {
    console.error(`lm_node exited unexpectedly with ${signal || code}`)
    process.exitCode = 1
  }
})
await waitForHealth()
previewProcess = spawn('npm', ['run', 'preview', '--', '--host', '127.0.0.1', '--port', '4173'], {
  cwd: webRoot,
  stdio: 'inherit',
})
previewProcess.once('exit', (code, signal) => {
  if (!stopping) {
    console.error(`Vite preview exited unexpectedly with ${signal || code}`)
    process.exitCode = 1
  }
})

await new Promise(() => {})
