import { describe, expect, it } from 'vitest'
import type { TraceStep } from './api'
import { stateAt } from './playback'

const step = (t_us: number, pos: number, event: TraceStep['event'] = null): TraceStep => ({
  t_us,
  raw: pos,
  pos,
  event,
  detect_to_trigger_us: null,
})

const trace = {
  steps: [
    step(10_000, 100),
    step(20_000, 600, 'down'),
    step(30_000, 300, 'up'),
    step(100_000, 700, 'rapid_down'),
    step(110_000, 100, 'up'),
  ],
  triggers: [
    { at_us: 20_500, onset_us: 21_000, intensity: 200 },
    { at_us: 100_500, onset_us: 101_000, intensity: 200 },
  ],
}

describe('stateAt', () => {
  it('is idle before the first step', () => {
    expect(stateAt(trace, 9_999)).toEqual({
      position: 0,
      pressed: false,
      lastEvent: null,
      counts: { down: 0, rapid_down: 0, up: 0 },
      hapticActive: false,
    })
  })

  it('includes an event at exactly its own time', () => {
    const s = stateAt(trace, 20_000)
    expect(s.position).toBe(600)
    expect(s.pressed).toBe(true)
    expect(s.lastEvent).toBe('down')
    expect(s.counts).toEqual({ down: 1, rapid_down: 0, up: 0 })
    expect(stateAt(trace, 19_999).counts.down).toBe(0)
  })

  it('holds the last step and totals after the end', () => {
    const s = stateAt(trace, 5_000_000)
    expect(s.position).toBe(100)
    expect(s.pressed).toBe(false)
    expect(s.lastEvent).toBe('up')
    expect(s.counts).toEqual({ down: 1, rapid_down: 1, up: 2 })
    expect(s.hapticActive).toBe(false)
  })

  it('counts rapid_down separately from down and keeps the pad pressed', () => {
    const s = stateAt(trace, 100_000)
    expect(s.counts).toEqual({ down: 1, rapid_down: 1, up: 1 })
    expect(s.lastEvent).toBe('rapid_down')
    expect(s.pressed).toBe(true)
  })

  it('flags haptics only from onset_us for 20 ms', () => {
    expect(stateAt(trace, 20_999).hapticActive).toBe(false)
    expect(stateAt(trace, 21_000).hapticActive).toBe(true)
    expect(stateAt(trace, 40_999).hapticActive).toBe(true)
    expect(stateAt(trace, 41_000).hapticActive).toBe(false)
    expect(stateAt(trace, 101_000).hapticActive).toBe(true)
  })
})
