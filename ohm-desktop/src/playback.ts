import type { RunResult, TraceEvent } from './api'

export const PAD_FLASH_US = 20_000

export interface PlaybackState {
  position: number
  pressed: boolean
  lastEvent: TraceEvent | null
  counts: Record<TraceEvent, number>
  hapticActive: boolean
}

export function stateAt(trace: Pick<RunResult, 'steps' | 'triggers'>, t_us: number): PlaybackState {
  let position = 0
  let pressed = false
  let lastEvent: TraceEvent | null = null
  const counts: Record<TraceEvent, number> = { down: 0, rapid_down: 0, up: 0 }
  for (const s of trace.steps) {
    if (s.t_us > t_us) break
    position = s.pos
    if (s.event) {
      lastEvent = s.event
      counts[s.event]++
      pressed = s.event !== 'up'
    }
  }
  const hapticActive = trace.triggers.some((g) => t_us >= g.onset_us && t_us < g.onset_us + PAD_FLASH_US)
  return { position, pressed, lastEvent, counts, hapticActive }
}
