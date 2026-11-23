import { describe, expect, it } from 'vitest'
import { defaultProfile, runScenario } from './mock'
import { patchProfile } from './profile'

const base = { ...defaultProfile(), actuation: 500, release_hysteresis: 50 }

describe('patchProfile', () => {
  it('applies a patch that is already valid unchanged', () => {
    expect(patchProfile(base, { actuation: 520 })).toEqual({ ...base, actuation: 520 })
    expect(patchProfile(base, { release_hysteresis: 499 })).toEqual({ ...base, release_hysteresis: 499 })
  })

  it('stops hysteresis one below actuation instead of moving actuation', () => {
    expect(patchProfile(base, { release_hysteresis: 1000 })).toEqual({ ...base, release_hysteresis: 499 })
  })

  it('pulls hysteresis down when actuation is lowered under it', () => {
    expect(patchProfile({ ...base, release_hysteresis: 400 }, { actuation: 300 })).toEqual({ ...base, actuation: 300, release_hysteresis: 299 })
  })

  it('keeps actuation at 1 or more, which forces hysteresis to 0', () => {
    expect(patchProfile(base, { actuation: 0 })).toEqual({ ...base, actuation: 1, release_hysteresis: 0 })
  })

  it('leaves the other fields alone', () => {
    expect(patchProfile(base, { haptic_intensity: 7 })).toEqual({ ...base, haptic_intensity: 7 })
  })

  it('never produces a profile the simulator rejects, for any slider position', () => {
    for (let v = 0; v <= 1000; v += 7) {
      for (const start of [base, { ...base, actuation: 1, release_hysteresis: 0 }, { ...base, actuation: 1000, release_hysteresis: 999 }]) {
        for (const patch of [{ actuation: v }, { release_hysteresis: v }]) {
          expect(() => runScenario('single_press', patchProfile(start, patch))).not.toThrow()
        }
      }
    }
  })
})
