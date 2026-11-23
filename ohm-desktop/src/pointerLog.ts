export interface PointerRecord {
  kind: 'down' | 'up'
  at: number
}

export type PointerPhase = 'press' | 'release'

export function pendingDowns(log: readonly PointerRecord[]): number {
  return log.reduce((open, r) => open + (r.kind === 'down' ? 1 : -1), 0)
}

export function recordPointer(log: readonly PointerRecord[], phase: PointerPhase, at: number): readonly PointerRecord[] {
  if (phase === 'press') return [...log, { kind: 'down', at }]
  return pendingDowns(log) > 0 ? [...log, { kind: 'up', at }] : log
}
