import { spawn, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, statSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { createServer } from 'node:net'
import { delimiter, join, resolve } from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import puppeteer from 'puppeteer-core'

const desktop = resolve(import.meta.dirname, '..')
const root = resolve(desktop, '..')
const exe = join(process.env.CARGO_TARGET_DIR ?? join(root, 'target'), 'debug', 'ohm-desktop.exe')
const shots = resolve(process.env.E2E_SHOTS ?? join(import.meta.dirname, 'shots'))
const cargoBin = join(process.env.USERPROFILE ?? '', '.cargo', 'bin')
const env = { ...process.env, PATH: `${cargoBin}${delimiter}${process.env.PATH}` }

const newest = (p) => {
  if (!existsSync(p)) return 0
  const s = statSync(p)
  if (!s.isDirectory()) return s.mtimeMs
  return Math.max(0, ...readdirSync(p).map((f) => newest(join(p, f))))
}
const SOURCES = [
  'ohm-desktop/src', 'ohm-desktop/index.html', 'ohm-desktop/src-tauri/src', 'ohm-desktop/src-tauri/Cargo.toml',
  'ohm-desktop/src-tauri/tauri.conf.json', 'ohm-core/src', 'ohm-core/Cargo.toml', 'ohm-simulator/src', 'ohm-simulator/scenarios',
  'ohm-simulator/Cargo.toml',
].map((p) => join(root, p))

function ensureBuilt() {
  if (existsSync(exe) && statSync(exe).mtimeMs >= Math.max(...SOURCES.map(newest))) return
  console.log('building debug app (exe missing or stale)')
  const r = spawnSync('pnpm tauri build --debug --no-bundle', { cwd: desktop, env, shell: true, stdio: 'inherit' })
  if (r.status !== 0 || !existsSync(exe)) throw new Error(`tauri build failed (exit ${r.status}), expected ${exe}`)
}

const freePort = () =>
  new Promise((ok, fail) => {
    const s = createServer().once('error', fail).listen(0, '127.0.0.1', () => {
      const { port } = s.address()
      s.close(() => ok(port))
    })
  })

async function waitFor(what, fn, ms = 20_000) {
  const end = Date.now() + ms
  let last
  while (Date.now() < end) {
    try {
      const v = await fn()
      if (v) return v
    } catch (e) {
      last = e
    }
    await sleep(100)
  }
  throw new Error(`timed out waiting for ${what}${last ? `: ${last.message}` : ''}`)
}

const eq = (a, b) => JSON.stringify(a) === JSON.stringify(b)
const assert = (ok, msg) => {
  if (!ok) throw new Error(`assertion failed: ${msg}`)
  console.log(`ok   ${msg}`)
}

let app
const profileDir = mkdtempSync(join(tmpdir(), 'ohm-e2e-'))
const killApp = () => {
  if (app?.pid) spawnSync('taskkill', ['/PID', String(app.pid), '/T', '/F'], { stdio: 'ignore' })
  app = undefined
  try {
    rmSync(profileDir, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 })
  } catch {}
}
process.on('SIGINT', () => {
  killApp()
  process.exit(130)
})

async function main() {
  ensureBuilt()
  mkdirSync(shots, { recursive: true })
  const port = await freePort()
  app = spawn(exe, [], {
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
      WEBVIEW2_USER_DATA_FOLDER: profileDir,
    },
    stdio: 'ignore',
  })
  app.once('exit', (code) => {
    if (app) console.error(`app exited early with code ${code}`)
  })
  const endpoint = `http://127.0.0.1:${port}`
  await waitFor('CDP endpoint', async () => (await (await fetch(`${endpoint}/json`)).json()).some((t) => t.type === 'page'))

  const browser = await puppeteer.connect({ browserURL: endpoint, defaultViewport: null })
  const page = await waitFor('app page', async () => (await browser.pages()).find((p) => p.url().startsWith('http')))
  const problems = []
  page.on('console', (m) => m.type() === 'error' && problems.push(`console.error: ${m.text()}`))
  page.on('pageerror', (e) => problems.push(`pageerror: ${e.message}`))
  await page.reload({ waitUntil: 'load' })

  const shot = async (name) => {
    const path = join(shots, `${name}.png`)
    await page.screenshot({ path, fullPage: true })
    console.log(`shot ${path}`)
  }
  const text = (sel) => page.$eval(sel, (el) => el.textContent.trim())
  const eventRows = () => page.$$eval('tbody tr', (rows) => rows.map((r) => r.children[1]?.textContent.trim()).filter(Boolean))
  const counters = (card) =>
    page.$$eval(`.checks .card:nth-child(${card}) .counters > div`, (ds) =>
      Object.fromEntries(ds.map((d) => [d.querySelector('dt').textContent, Number(d.querySelector('dd').textContent)])),
    )
  const clickButton = (label) =>
    page.evaluate((l) => [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === l).click(), label)

  await page.waitForSelector('.panel .mode')
  const mode = await text('.panel .mode')
  assert(!mode.includes('browser mock'), `sidebar says "${mode}", not the browser mock`)
  assert(await page.evaluate(() => '__TAURI_INTERNALS__' in window), 'Tauri IPC bridge is present')

  await page.select('select', 'double_press_fast')
  await waitFor('double_press_fast events', async () => eq(await eventRows(), ['Down', 'Up', 'RapidDown', 'Up']))
  assert(eq(await eventRows(), ['Down', 'Up', 'RapidDown', 'Up']), 'event table is Down, Up, RapidDown, Up at the 20 ms default')
  assert((await text('.control:nth-of-type(4) output')) === '20 ms', 'rapid trigger window defaults to 20 ms')
  await shot('1-initial')

  await clickButton('Play')
  await waitFor('playback past the first Down', async () => (await counters(1)).Down === 1)
  await shot('2-mid-playback')
  await clickButton('Pause')
  await clickButton('4x')
  await clickButton('Restart')
  await waitFor('pad counters Down=1 RapidDown=1 Up=2 at 4x', async () => {
    const c = await counters(1)
    return c.Down === 1 && c.RapidDown === 1 && c.Up === 2
  })
  assert(true, 'pad counters read Down=1, RapidDown=1, Up=2 after 4x playback (the trace has Down, Up, RapidDown, Up)')
  await waitFor('playback end', async () => (await text('.playback button')) === 'Play')
  await shot('3-after-counters')

  const target = await page.$('.real-target')
  await target.scrollIntoView()
  const box = await target.boundingBox()
  const cdp = await page.createCDPSession()
  const at = { x: box.x + box.width / 2, y: box.y + box.height / 2 }
  const mouse = (type, extra = {}) => cdp.send('Input.dispatchMouseEvent', { type, ...at, button: 'left', ...extra })
  await mouse('mouseMoved', { button: 'none' })
  for (let i = 0; i < 3; i++) {
    await mouse('mousePressed', { buttons: 1, clickCount: 1 })
    await mouse('mouseReleased', { buttons: 0, clickCount: 1 })
  }
  await waitFor('real pointer counters', async () => {
    const c = await counters(2)
    return c.pointerdown === 3 && c.pointerup === 3
  })
  assert(true, 'real click box counted 3 pointerdown and 3 pointerup from CDP Input.dispatchMouseEvent')
  await shot('4-real-click')

  const before = await text('.chart text.threshold-label.actuation')
  await page.focus('.control:nth-of-type(2) input[type=range]')
  for (let i = 0; i < 20; i++) await page.keyboard.press('ArrowRight')
  await waitFor('chart re-render after slider change', async () => (await text('.chart text.threshold-label.actuation')) === 'actuation 520')
  assert(before === 'actuation 500', 'chart actuation label was 500 before the slider moved')
  assert((await page.$('.chart.stale')) === null && (await page.$('.error')) === null, 'chart re-rendered at actuation 520 with no error banner')
  await shot('5-slider-changed')

  assert(problems.length === 0, `no console errors${problems.length ? `: ${problems.join(' | ')}` : ''}`)
  await browser.disconnect()
}

try {
  await main()
  console.log('e2e passed')
} catch (e) {
  console.error(e.message)
  process.exitCode = 1
} finally {
  killApp()
}
