import { invoke } from '@tauri-apps/api/core'
import * as mock from './mock'

export interface ProfileDto {
  actuation: number
  release_hysteresis: number
  rapid_trigger_window_ms: number
  haptic_intensity: number
}

export type TraceEvent = 'down' | 'rapid_down' | 'up'

export interface TraceStep {
  t_us: number
  raw: number
  pos: number
  event: TraceEvent | null
  detect_to_trigger_us: number | null
}

export interface TraceTrigger {
  at_us: number
  onset_us: number
  intensity: number
}

export interface RunResult {
  scenario: string
  duration_us: number
  actuation: number
  release: number
  trigger_delay_us: number
  steps: TraceStep[]
  triggers: TraceTrigger[]
  waveform_rate_hz: number
  waveform: number[]
}

export const isMock = !('__TAURI_INTERNALS__' in window)

export const listScenarios = (): Promise<string[]> =>
  isMock ? Promise.resolve(mock.listScenarios()) : invoke('list_scenarios')

export const defaultProfile = (): Promise<ProfileDto> =>
  isMock ? Promise.resolve(mock.defaultProfile()) : invoke('default_profile')

export function runScenario(name: string, profile: ProfileDto): Promise<RunResult> {
  if (!isMock) return invoke('run_scenario', { name, profile })
  try {
    return Promise.resolve(mock.runScenario(name, profile))
  } catch (e) {
    return Promise.reject(e instanceof Error ? e.message : String(e))
  }
}
