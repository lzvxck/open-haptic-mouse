import type { TraceEvent } from './api'

export const EVENT_LABEL: Record<TraceEvent, string> = { down: 'Down', rapid_down: 'RapidDown', up: 'Up' }

export type Scale = (v: number) => number

export function linear(d0: number, d1: number, r0: number, r1: number): Scale {
  if (d0 === d1) return () => r0
  return (v) => r0 + ((v - d0) / (d1 - d0)) * (r1 - r0)
}

export function timeToX(tUs: number, durationUs: number, left: number, width: number): number {
  return linear(0, durationUs, left, left + width)(tUs)
}

export function xToTime(x: number, durationUs: number, left: number, width: number): number {
  return Math.min(Math.max(linear(left, left + width, 0, durationUs)(x), 0), durationUs)
}

export function valueToY(v: number, max: number, top: number, height: number): number {
  return linear(0, max, top + height, top)(Math.min(Math.max(v, 0), max))
}

const TIME_STEPS_MS = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000]

export function timeTicksMs(durationUs: number, maxTicks = 8): number[] {
  const durationMs = durationUs / 1000
  const step = TIME_STEPS_MS.find((s) => durationMs / s <= maxTicks) ?? TIME_STEPS_MS[TIME_STEPS_MS.length - 1]
  const ticks: number[] = []
  for (let t = 0; t <= durationMs; t += step) ticks.push(t)
  return ticks
}

export function linePoints(points: ReadonlyArray<readonly [number, number]>): string {
  return points.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(' ')
}

export function minMaxBuckets(samples: ArrayLike<number>, buckets: number): Array<[number, number]> {
  const n = Math.min(Math.max(Math.floor(buckets), 1), samples.length)
  const out: Array<[number, number]> = []
  for (let b = 0; b < n; b++) {
    const from = Math.floor((b * samples.length) / n)
    const to = Math.max(Math.floor(((b + 1) * samples.length) / n), from + 1)
    let lo = Infinity
    let hi = -Infinity
    for (let i = from; i < to; i++) {
      lo = Math.min(lo, samples[i])
      hi = Math.max(hi, samples[i])
    }
    out.push([lo, hi])
  }
  return out
}
