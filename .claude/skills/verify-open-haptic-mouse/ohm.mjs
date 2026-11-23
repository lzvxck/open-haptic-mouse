#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process'
import { createRequire } from 'node:module'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { basename, delimiter, join, resolve } from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'

const root = resolve(import.meta.dirname, '..', '..', '..')
const desktop = join(root, 'ohm-desktop')
const verifyDir = join(root, '.verify')
const stateFile = join(verifyDir, 'state.json')
const evidenceDir = join(verifyDir, 'evidence')
const exe = join(process.env.CARGO_TARGET_DIR ?? join(root, 'target'), 'debug', 'ohm-desktop.exe')
const cargoBin = join(process.env.USERPROFILE ?? '', '.cargo', 'bin')
const env = { ...process.env, PATH: `${cargoBin}${delimiter}${process.env.PATH}` }

// Everything that ends up inside the exe: Rust workspace inputs, the frontend, and the Tauri config.
const SOURCES = [
  'Cargo.toml', 'Cargo.lock', 'ohm-core/src', 'ohm-core/Cargo.toml', 'ohm-simulator/src', 'ohm-simulator/scenarios',
  'ohm-simulator/Cargo.toml', 'ohm-desktop/src', 'ohm-desktop/index.html', 'ohm-desktop/public', 'ohm-desktop/package.json',
  'ohm-desktop/vite.config.ts', 'ohm-desktop/src-tauri/src', 'ohm-desktop/src-tauri/Cargo.toml', 'ohm-desktop/src-tauri/build.rs',
  'ohm-desktop/src-tauri/tauri.conf.json', 'ohm-desktop/src-tauri/capabilities',
].map((p) => join(root, p))

const newest = (p) => {
  if (!existsSync(p)) return 0
  const s = statSync(p)
  return s.isDirectory() ? Math.max(0, ...readdirSync(p).map((f) => newest(join(p, f)))) : s.mtimeMs
}
const stale = () => !existsSync(exe) || statSync(exe).mtimeMs < Math.max(...SOURCES.map(newest))
const readState = () => (existsSync(stateFile) ? JSON.parse(readFileSync(stateFile, 'utf8')) : null)
// True only if `pid` is currently an ohm-desktop.exe (guards against PID reuse).
const ours = (pid) =>
  spawnSync('tasklist', ['/FI', `PID eq ${pid}`, '/FI', 'IMAGENAME eq ohm-desktop.exe', '/NH', '/FO', 'CSV'], { encoding: 'utf8' })
    .stdout.includes(`,"${pid}",`)
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
  let puppeteer
  try {
    puppeteer = createRequire(join(desktop, 'package.json'))('puppeteer-core')
  } catch {
    throw new Error('puppeteer-core is missing: run `pnpm install` in ohm-desktop')
  }
  const browser = await puppeteer.connect({ browserURL: `http://127.0.0.1:${st.port}`, defaultViewport: null, protocolTimeout: 15_000 })
  try {
    const page = (await browser.pages()).find((p) => p.url().startsWith('http'))
    if (!page) throw new Error('app page not found over CDP')
    return { browser, page }
  } catch (e) {
    await browser.disconnect()
    throw e
  }
}

async function withPage(fn) {
  const { browser, page } = await connect()
  try {
    return await fn(page)
  } finally {
    await browser.disconnect()
  }
}

async function ready(p) {
  for (const sel of ['.panel .mode', 'select option', '.chart', '.real-target']) await p.waitForSelector(sel, { timeout: 15_000 })
  if (!(await p.evaluate(() => '__TAURI_INTERNALS__' in window))) throw new Error('Tauri IPC bridge missing: page is not running inside the app')
}

function clearState(st) {
  // Only ever delete a profile dir this skill created.
  if (st.profile && basename(st.profile).startsWith('ohm-verify-')) {
    try {
      rmSync(st.profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 })
    } catch {}
  }
  rmSync(stateFile, { force: true })
}

async function start() {
  const old = readState()
  if (old) {
    if (ours(old.pid)) throw new Error(`instance already running (pid ${old.pid}); run \`stop\` first`)
    clearState(old)
  }
  if (stale()) {
    console.log('building debug app (exe missing or older than sources)')
    const r = spawnSync('pnpm tauri build --debug --no-bundle', { cwd: desktop, env, shell: true, stdio: 'inherit' })
    if (r.status !== 0 || !existsSync(exe)) throw new Error(`tauri build failed (exit ${r.status}); expected ${exe}`)
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
  try {
    const end = Date.now() + 30_000
    while (Date.now() < end) {
      try {
        const pages = await (await fetch(`http://127.0.0.1:${port}/json`)).json()
        if (pages.some((t) => t.type === 'page' && t.url.startsWith('http'))) break
      } catch {}
      await sleep(200)
    }
    await withPage(ready)
  } catch (e) {
    await stop() // do not leave a half-started instance behind
    throw e
  }
  console.log(`ready: pid=${app.pid} cdp=http://127.0.0.1:${port} evidence=${evidenceDir}`)
}

async function doctor() {
  const st = readState()
  const out = { instance: Boolean(st), pidAlive: false, cdp: false, tauriBridge: false, mode: null, exeStale: stale() }
  if (st) {
    out.pidAlive = ours(st.pid)
    if (out.pidAlive) {
      try {
        await withPage(async (p) => {
          out.cdp = true
          out.tauriBridge = await p.evaluate(() => '__TAURI_INTERNALS__' in window)
          out.mode = await p.$eval('.panel .mode', (e) => e.textContent.trim())
        })
      } catch (e) {
        out.error = e.message
      }
    }
  }
  console.log(JSON.stringify(out, null, 2))
  if (!(out.instance && out.pidAlive && out.cdp && out.tauriBridge && out.mode === 'Simulator mode' && !out.exeStale)) process.exitCode = 1
}

async function stop() {
  const st = readState()
  if (!st) return console.log('nothing to stop')
  if (!ours(st.pid)) {
    clearState(st)
    return console.log(`pid ${st.pid} is not ohm-desktop.exe (stale state); cleaned it, killed nothing`)
  }
  spawnSync('taskkill', ['/PID', String(st.pid), '/T', '/F'], { stdio: 'ignore' })
  await sleep(500)
  if (ours(st.pid)) {
    console.error(`taskkill failed: pid ${st.pid} is still running; kept ${stateFile} and ${st.profile}`)
    process.exitCode = 1
    return
  }
  clearState(st)
  console.log(`stopped pid ${st.pid}; evidence kept in ${evidenceDir}`)
}

async function clickButton(p, label) {
  const end = Date.now() + 3000
  for (;;) {
    const r = await p.evaluate((l) => {
      const b = [...document.querySelectorAll('button')].find((x) => x.textContent.trim() === l)
      if (!b) return 'absent'
      if (b.disabled) return 'disabled'
      b.click()
      return 'ok'
    }, label)
    if (r === 'ok') return
    if (Date.now() > end) throw new Error(`button "${label}" is ${r === 'absent' ? 'not on the page' : 'disabled'} after 3 s`)
    await sleep(50)
  }
}

async function selectScenario(p, name) {
  const end = Date.now() + 3000
  let names = []
  while (!names.includes(name) && Date.now() < end) {
    names = await p.$$eval('select option', (o) => o.map((x) => x.value))
    if (!names.includes(name)) await sleep(50)
  }
  if (!names.includes(name)) throw new Error(`unknown scenario "${name}"; options: ${JSON.stringify(names)}`)
  await p.select('select', name)
  const now = await p.$eval('select', (s) => s.value)
  if (now !== name) throw new Error(`select did not take: value is "${now}", wanted "${name}"`)
  await sleep(400) // debounce (150 ms) plus the Tauri round trip
}

const SLIDERS = ['actuation', 'hysteresis', 'window', 'haptic'] // .control:nth-of-type(2..5)
const sliderValues = (p) =>
  p.$$eval('.control output', (o) => o.map((x) => parseInt(x.textContent, 10))).then((v) => Object.fromEntries(SLIDERS.map((k, i) => [k, v[i]])))

// Real browser-level mouse input at viewport point (x, y), via CDP.
async function mouse(p, at, clicks) {
  const cdp = await p.createCDPSession()
  const send = (type, extra = {}) => cdp.send('Input.dispatchMouseEvent', { type, ...at, button: 'left', ...extra })
  await send('mouseMoved', { button: 'none' })
  for (let i = 0; i < clicks; i++) {
    await send('mousePressed', { buttons: 1, clickCount: 1 })
    await send('mouseReleased', { buttons: 0, clickCount: 1 })
  }
}

async function centerOf(p, sel) {
  const el = await p.$(sel)
  if (!el) throw new Error(`${sel} not found (no result rendered yet?)`)
  await el.scrollIntoView()
  const box = await el.boundingBox()
  if (!box) throw new Error(`${sel} has no layout box`)
  return box
}

const int = (s, what, min) => {
  const n = Number(s)
  if (s === undefined || s === '' || !Number.isInteger(n) || n < min) throw new Error(`${what} must be an integer >= ${min}, got "${s}"`)
  return n
}
const needCard = (s) => {
  if (s !== 'pad' && s !== 'real') throw new Error('usage: counters <pad|real>')
  return s === 'pad' ? 1 : 2
}

const [cmd, ...args] = process.argv.slice(2)
const out = (v) => console.log(JSON.stringify(v))
const commands = {
  start,
  doctor,
  stop,
  eval: () => {
    if (!args.length) throw new Error('usage: eval "<js expression>"')
    return withPage(async (p) => out(await p.evaluate(args.join(' '))))
  },
  click: () => withPage((p) => clickButton(p, args.join(' '))),
  select: () => withPage((p) => selectScenario(p, args[0])),
  reset: () => withPage(async (p) => (await p.reload({ waitUntil: 'load' }), await ready(p))),
  set: () =>
    withPage(async (p) => {
      const n = SLIDERS.indexOf(args[0]) + 2
      if (n < 2) throw new Error(`usage: set <${SLIDERS.join('|')}> <integer>`)
      const v = int(args[1], 'slider value', 0)
      await p.evaluate(
        (n, v) => {
          const s = document.querySelector(`.control:nth-of-type(${n}) input[type=range]`)
          if (!s) throw new Error('slider not found')
          Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(s, String(v))
          s.dispatchEvent(new Event('input', { bubbles: true }))
        },
        n,
        v,
      )
      await sleep(400)
      out(await sliderValues(p))
    }),
  events: () =>
    withPage(async (p) =>
      out(await p.$$eval('tbody tr', (rows) => rows.map((r) => r.children[1]?.textContent.trim()).filter(Boolean))),
    ),
  counters: () =>
    withPage(async (p) =>
      out(
        await p.$$eval(`.checks .card:nth-child(${needCard(args[0])}) .counters > div`, (ds) =>
          Object.fromEntries(ds.map((d) => [d.querySelector('dt').textContent, Number(d.querySelector('dd').textContent)])),
        ),
      ),
    ),
  seek: () =>
    withPage(async (p) => {
      const ms = Number(args[0])
      if (args[0] === undefined || !Number.isFinite(ms) || ms < 0) throw new Error('usage: seek <ms>')
      const dur = Number(/\/\s*([\d.]+)\s*ms/.exec(await p.$eval('.clock', (e) => e.textContent))?.[1])
      if (!(ms <= dur)) throw new Error(`seek ${ms} ms is outside the trace (0..${dur} ms)`)
      const box = await centerOf(p, '.chart')
      // Mirrors Chart.tsx: viewBox width 900, plot starts at x=48 and is 836 wide.
      const x = box.x + (((48 + (ms / dur) * 836) / 900) * box.width)
      await mouse(p, { x, y: box.y + box.height / 2 }, 1)
      await sleep(100)
      out(await p.$eval('.clock', (e) => e.textContent.trim()))
    }),
  realclick: () =>
    withPage(async (p) => {
      const n = int(args[0] ?? '1', 'click count', 1)
      const box = await centerOf(p, '.real-target')
      await mouse(p, { x: box.x + box.width / 2, y: box.y + box.height / 2 }, n)
    }),
  shot: () =>
    withPage(async (p) => {
      const path = join(evidenceDir, `${args[0] ?? 'shot'}.png`)
      mkdirSync(evidenceDir, { recursive: true })
      await p.screenshot({ path, fullPage: true })
      console.log(path)
    }),
}

if (!Object.hasOwn(commands, cmd)) {
  console.error(`usage: ohm.mjs <${Object.keys(commands).join('|')}> [args]`)
  process.exitCode = 2
} else {
  try {
    await commands[cmd]()
  } catch (e) {
    console.error(e.message)
    process.exitCode = 1
  }
}
const code = process.exitCode ?? 0
process.stdout.write('', () => process.stderr.write('', () => process.exit(code)))
