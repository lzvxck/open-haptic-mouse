import type { ProfileDto, RunResult, TraceEvent, TraceStep, TraceTrigger } from './api'

// Browser-only stand-in for the Rust simulator so `pnpm dev` renders without Tauri. Not the real model.
type Seg = readonly [from: number, to: number, durationUs: number]

const press = (hold: number): Seg[] => [[0, 1000, 20_000], [1000, 1000, hold], [1000, 0, 20_000]]

const SCENARIOS: Record<string, { segs: Seg[]; noise: number }> = {
  single_press: { segs: [[0, 0, 10_000], ...press(10_000), [0, 0, 10_000]], noise: 0 },
  double_press_fast: { segs: [[0, 0, 5_000], [0, 1000, 5_000], [1000, 0, 6_000], [0, 0, 3_000], [0, 1000, 5_000], [1000, 0, 6_000]], noise: 0 },
  double_press_slow: { segs: [[0, 0, 10_000], ...press(1_000), [0, 0, 20_000], ...press(1_000)], noise: 0 },
  double_press_edge: { segs: [[0, 0, 10_000], ...press(1_000), [0, 0, 18_000], ...press(1_000)], noise: 0 },
  noisy_hold: { segs: [[0, 0, 50_000], [0, 450, 100_000], [450, 450, 380_000]], noise: 15 },
  long_hold: { segs: [[0, 0, 10_000], ...press(1_000_000)], noise: 0 },
}

const STEP_US = 1000
const RATE_HZ = 10_000
const TAIL_US = 300_000
const DELAY_US = 1500
const REST = 30_000
const SPAN = 30_000

export const listScenarios = (): string[] => Object.keys(SCENARIOS)

export const defaultProfile = (): ProfileDto => ({
  actuation: 500,
  release_hysteresis: 50,
  rapid_trigger_window_ms: 20,
  haptic_intensity: 128,
})

function posAt(segs: readonly Seg[], t: number): number {
  let start = 0
  for (const [from, to, d] of segs) {
    if (t < start + d) return from + ((to - from) * (t - start)) / d
    start += d
  }
  return 0
}

export function runScenario(name: string, p: ProfileDto): RunResult {
  const scenario = SCENARIOS[name]
  if (!scenario) throw new Error(`unknown scenario "${name}"`)
  if (p.release_hysteresis > p.actuation) {
    throw new Error(`release hysteresis (${p.release_hysteresis}) cannot exceed actuation (${p.actuation})`)
  }
  const total = scenario.segs.reduce((s, [, , d]) => s + d, 0)
  const release = p.actuation - p.release_hysteresis
  const steps: TraceStep[] = []
  const triggers: TraceTrigger[] = []
  let down = false
  let lastUp: number | null = null
  for (let i = 0; i * STEP_US < total; i++) {
    const t = i * STEP_US
    const noise = Math.round(Math.sin(i * 12.9898) * scenario.noise)
    const pos = Math.min(Math.max(Math.round(posAt(scenario.segs, t)) + noise, 0), 1000)
    let event: TraceEvent | null = null
    if (!down && pos >= p.actuation) {
      down = true
      event = lastUp !== null && t - lastUp < p.rapid_trigger_window_ms * 1000 ? 'rapid_down' : 'down'
      triggers.push({ at_us: t, onset_us: t + DELAY_US, intensity: p.haptic_intensity })
    } else if (down && pos < release) {
      down = false
      lastUp = t
      event = 'up'
    }
    steps.push({
      t_us: t,
      raw: REST + Math.round((pos * SPAN) / 1000),
      pos,
      event,
      detect_to_trigger_us: event === 'down' || event === 'rapid_down' ? DELAY_US : null,
    })
  }
  const duration = total + TAIL_US
  const waveform = Array.from({ length: Math.floor((duration * RATE_HZ) / 1_000_000) }, (_, i) => {
    const t = (i * 1_000_000) / RATE_HZ
    return triggers.reduce((sum, { onset_us, intensity }) => {
      if (t < onset_us) return sum
      const dt = (t - onset_us) / 1e6
      const env = (Math.exp(-dt / 0.08) - Math.exp(-dt / 0.05)) / 0.171
      return sum + (intensity / 255) * env * Math.sin(2 * Math.PI * 180 * dt)
    }, 0)
  })
  return {
    scenario: name,
    duration_us: duration,
    actuation: p.actuation,
    release,
    trigger_delay_us: DELAY_US,
    steps,
    triggers,
    waveform_rate_hz: RATE_HZ,
    waveform,
  }
}
