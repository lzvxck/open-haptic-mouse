import { describe, expect, it } from 'vitest'
import { pendingDowns, recordPointer } from './pointerLog'
import type { PointerPhase, PointerRecord } from './pointerLog'

function play(...steps: [PointerPhase, number][]): readonly PointerRecord[] {
  return steps.reduce<readonly PointerRecord[]>((log, [phase, at]) => recordPointer(log, phase, at), [])
}

const kinds = (log: readonly PointerRecord[]) => log.map((r) => r.kind)

describe('recordPointer', () => {
  it('counts a normal click as one down and one up', () => {
    const log = play(['press', 1], ['release', 2])
    expect(kinds(log)).toEqual(['down', 'up'])
    expect(pendingDowns(log)).toBe(0)
  })

  it('closes a pending down on cancel or lost capture when no pointerup arrives', () => {
    const log = play(['press', 1], ['release', 5])
    expect(log).toEqual([
      { kind: 'down', at: 1 },
      { kind: 'up', at: 5 },
    ])
  })

  it('does not double count a lost capture that follows pointerup', () => {
    const log = play(['press', 1], ['release', 2], ['release', 3])
    expect(kinds(log)).toEqual(['down', 'up'])
  })

  it('does not double count a pointerup that follows a cancel', () => {
    const log = play(['press', 1], ['release', 2], ['release', 3], ['release', 4])
    expect(kinds(log)).toEqual(['down', 'up'])
  })

  it('ignores an up with no pending down', () => {
    expect(play(['release', 1])).toEqual([])
  })
})
