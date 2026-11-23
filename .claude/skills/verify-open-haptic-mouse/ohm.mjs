#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process'
import { createRequire } from 'node:module'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { delimiter, join, resolve } from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'

const root = resolve(import.meta.dirname, '..', '..', '..')
const desktop = join(root, 'ohm-desktop')
const verifyDir = join(root, '.verify')
const stateFile = join(verifyDir, 'state.json')
const evidenceDir = join(verifyDir, 'evidence')
const exe = join(process.env.CARGO_TARGET_DIR ?? join(root, 'target'), 'debug', 'ohm-desktop.exe')
const cargoBin = join(process.env.USERPROFILE ?? '', '.cargo', 'bin')
const env = { ...process.env, PATH: `${cargoBin}${delimiter}${process.env.PATH}` }
const puppeteer = createRequire(join(desktop, 'package.json'))('puppeteer-core')

const SOURCES = [
  'ohm-desktop/src', 'ohm-desktop/index.html', 'ohm-desktop/src-tauri/src', 'ohm-desktop/src-tauri/Cargo.toml',
  'ohm-desktop/src-tauri/tauri.conf.json', 'ohm-core/src', 'ohm-core/Cargo.toml', 'ohm-simulator/src', 'ohm-simulator/scenarios',
  'ohm-simulator/Cargo.toml',
].map((p) => join(root, p))

const newest = (p) => {
  if (!existsSync(p)) return 0
  const s = statSync(p)
  return s.isDirectory() ? Math.max(0, ...readdirSync(p).map((f) => newest(join(p, f)))) : s.mtimeMs
}
const stale = () => !existsSync(exe) || statSync(exe).mtimeMs < Math.max(...SOURCES.map(newest))
const readState = () => (existsSync(stateFile) ? JSON.parse(readFileSync(stateFile, 'utf8')) : null)
const alive = (pid) => spawnSync('tasklist', ['/FI', `PID eq ${pid}`, '/NH'], { encoding: 'utf8' }).stdout.includes(String(pid))
const freePort = () =>
  new Promise((ok, fail) => {
    const s = createServer().once('error', fail).listen(0, '127.0.0.1', () => {
      const { port } = s.address()
      s.close(() => ok(port))
    })
  })

async function connect() {
  const st = readState()
  if (!st) throw new Error('no instance started by this skill: run `start` first')
  const browser = await puppeteer.connect({ browserURL: `http://127.0.0.1:${st.port}`, defaultViewport: null })
  const page = (await browser.pages()).find((p) => p.url().startsWith('http'))
  if (!page) throw new Error('app page not found over CDP')
  return { browser, page, st }
}

async function start() {
  if (readState() && alive(readState().pid)) throw new Error(`instance already running (pid ${readState().pid}); run \`stop\` first`)
  if (stale()) {
    console.log('building debug app (exe missing or older than sources)')
    const r = spawnSync('pnpm tauri build --debug --no-bundle', { cwd: desktop, env, shell: true, stdio: 'inherit' })
    if (r.status !== 0 || stale()) throw new Error(`tauri build failed (exit ${r.status}); expected ${exe}`)
  }
  mkdirSync(evidenceDir, { recursive: true })
  const port = await freePort()
  const profile = mkdtempSync(join(tmpdir(), 'ohm-verify-'))
  const app = spawn(exe, [], {
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`, WEBVIEW2_USER_DATA_FOLDER: profile },
    stdio: 'ignore',
    detached: true,
  })
  app.unref()
  writeFileSync(stateFile, JSON.stringify({ pid: app.pid, port, profile, exe, startedAt: new Date().toISOString() }, null, 2))
  const end = Date.now() + 30_000
  while (Date.now() < end) {
    try {
      const pages = await (await fetch(`http://127.0.0.1:${port}/json`)).json()
      if (pages.some((t) => t.type === 'page' && t.url.startsWith('http'))) break
    } catch {}
    await sleep(200)
  }
  const { browser, page } = await connect()
  await page.waitForSelector('.panel .mode', { timeout: 15_000 })
  await browser.disconnect()
  console.log(`ready: pid=${app.pid} cdp=http://127.0.0.1:${port} evidence=${evidenceDir}`)
}

async function doctor() {
  const st = readState()
  const out = { instance: Boolean(st), pidAlive: false, cdp: false, tauriBridge: false, mode: null, exeStale: stale() }
  if (st) {
    out.pidAlive = alive(st.pid)
    if (out.pidAlive) {
      try {
        const { browser, page } = await connect()
        out.cdp = true
        out.tauriBridge = await page.evaluate(() => '__TAURI_INTERNALS__' in window)
        out.mode = await page.$eval('.panel .mode', (e) => e.textContent.trim())
        await browser.disconnect()
      } catch (e) {
        out.error = e.message
      }
    }
  }
  console.log(JSON.stringify(out, null, 2))
  const good = out.instance && out.pidAlive && out.cdp && out.tauriBridge && out.mode === 'Simulator mode' && !out.exeStale
  if (!good) process.exitCode = 1
}

async function withPage(fn) {
  const { browser, page } = await connect()
  try {
    return await fn(page)
  } finally {
    await browser.disconnect()
  }
}

async function stop() {
  const st = readState()
  if (!st) return console.log('nothing to stop')
  spawnSync('taskkill', ['/PID', String(st.pid), '/T', '/F'], { stdio: 'ignore' })
  await sleep(500)
  try {
    rmSync(st.profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 })
  } catch {}
  rmSync(stateFile, { force: true })
  console.log(`stopped pid ${st.pid}; evidence kept in ${evidenceDir}`)
}

const [cmd, ...args] = process.argv.slice(2)
const commands = {
  start,
  doctor,
  stop,
  eval: () => withPage(async (p) => console.log(JSON.stringify(await p.evaluate(args.join(' '))))),
  click: () =>
    withPage((p) =>
      p.evaluate((l) => {
        const b = [...document.querySelectorAll('button')].find((x) => x.textContent.trim() === l)
        if (!b) throw new Error(`no button labelled "${l}"`)
        b.click()
      }, args.join(' ')),
    ),
  select: () => withPage((p) => p.select('select', args[0])),
  realclick: () =>
    withPage(async (p) => {
      const target = await p.$('.real-target')
      await target.scrollIntoView()
      const box = await target.boundingBox()
      const at = { x: box.x + box.width / 2, y: box.y + box.height / 2 }
      const cdp = await p.createCDPSession()
      const mouse = (type, extra = {}) => cdp.send('Input.dispatchMouseEvent', { type, ...at, button: 'left', ...extra })
      await mouse('mouseMoved', { button: 'none' })
      for (let i = 0; i < Number(args[0] ?? 1); i++) {
        await mouse('mousePressed', { buttons: 1, clickCount: 1 })
        await mouse('mouseReleased', { buttons: 0, clickCount: 1 })
      }
    }),
  shot: () =>
    withPage(async (p) => {
      const path = join(evidenceDir, `${args[0] ?? 'shot'}.png`)
      mkdirSync(evidenceDir, { recursive: true })
      await p.screenshot({ path, fullPage: true })
      console.log(path)
    }),
}
if (!commands[cmd]) {
  console.error('usage: ohm.mjs <start|doctor|eval|click|select|realclick|shot|stop> [args]')
  process.exit(2)
}
try {
  await commands[cmd]()
} catch (e) {
  console.error(e.message)
  process.exitCode = 1
}
