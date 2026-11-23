import { describe, expect, it } from 'vitest'
import { defaultProfile, listScenarios, runScenario } from './mock'

describe('browser mock', () => {
  it('runs every scenario with the default profile', () => {
    for (const name of listScenarios()) {
      const r = runScenario(name, defaultProfile())
      expect(r.steps.length).toBeGreaterThan(0)
      expect(r.waveform.length).toBe(Math.floor((r.duration_us * r.waveform_rate_hz) / 1_000_000))
    }
  })

  it('emits Down then Up for a single press and a RapidDown inside the default window', () => {
    const events = (n: string, p = defaultProfile()) => runScenario(n, p).steps.flatMap((s) => (s.event ? [s.event] : []))
    expect(events('single_press')).toEqual(['down', 'up'])
    expect(events('double_press_fast')).toEqual(['down', 'up', 'rapid_down', 'up'])
    expect(events('double_press_fast', { ...defaultProfile(), rapid_trigger_window_ms: 0 })).toEqual(['down', 'up', 'down', 'up'])
  })

  it('rejects hysteresis at or above actuation with a readable message', () => {
    expect(() => runScenario('single_press', { ...defaultProfile(), actuation: 100, release_hysteresis: 150 })).toThrow(
      'release hysteresis (150) must be below actuation (100)',
    )
    expect(() => runScenario('single_press', { ...defaultProfile(), actuation: 100, release_hysteresis: 100 })).toThrow(
      'release hysteresis (100) must be below actuation (100)',
    )
  })

  it('rejects zero actuation with a readable message', () => {
    expect(() => runScenario('single_press', { ...defaultProfile(), actuation: 0, release_hysteresis: 0 })).toThrow(
      'actuation must be at least 1',
    )
  })
})
