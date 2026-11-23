import { describe, expect, it } from 'vitest'
import { linePoints, linear, minMaxBuckets, timeTicksMs, timeToX, valueToY, xToTime } from './scale'

describe('linear', () => {
  it('maps the domain ends to the range ends and interpolates between', () => {
    const s = linear(0, 10, 100, 200)
    expect([s(0), s(5), s(10)]).toEqual([100, 150, 200])
  })

  it('supports inverted ranges', () => {
    expect(linear(0, 1, 50, 0)(0.5)).toBe(25)
  })

  it('collapses a zero-width domain to the range start instead of dividing by zero', () => {
    expect(linear(3, 3, 7, 99)(3)).toBe(7)
  })
})

describe('timeToX', () => {
  it('places 0 at the left edge and the duration at the right edge', () => {
    expect(timeToX(0, 70_000, 40, 800)).toBe(40)
    expect(timeToX(70_000, 70_000, 40, 800)).toBe(840)
    expect(timeToX(35_000, 70_000, 40, 800)).toBe(440)
  })
})

describe('xToTime', () => {
  it('inverts timeToX and clamps outside the plot', () => {
    expect(xToTime(440, 70_000, 40, 800)).toBe(35_000)
    expect(xToTime(timeToX(12_500, 70_000, 40, 800), 70_000, 40, 800)).toBeCloseTo(12_500)
    expect(xToTime(0, 70_000, 40, 800)).toBe(0)
    expect(xToTime(9999, 70_000, 40, 800)).toBe(70_000)
  })
})

describe('valueToY', () => {
  it('puts 0 at the bottom and max at the top of the lane', () => {
    expect(valueToY(0, 1000, 10, 200)).toBe(210)
    expect(valueToY(1000, 1000, 10, 200)).toBe(10)
    expect(valueToY(500, 1000, 10, 200)).toBe(110)
  })

  it('clamps values outside the range', () => {
    expect(valueToY(-5, 1000, 10, 200)).toBe(210)
    expect(valueToY(1200, 1000, 10, 200)).toBe(10)
  })
})

describe('timeTicksMs', () => {
  it('picks a round step that keeps the tick count bounded', () => {
    expect(timeTicksMs(70_000)).toEqual([0, 10, 20, 30, 40, 50, 60, 70])
    expect(timeTicksMs(1_330_000).length).toBeLessThanOrEqual(9)
    expect(timeTicksMs(1_330_000)[1]).toBe(200)
  })
})

describe('linePoints', () => {
  it('formats an SVG points attribute', () => {
    expect(linePoints([[0, 1], [2.25, 3]])).toBe('0.0,1.0 2.3,3.0')
  })
})

describe('minMaxBuckets', () => {
  it('keeps the extremes of each bucket so oscillations survive downsampling', () => {
    expect(minMaxBuckets([0, 1, -1, 0, 5, -5], 2)).toEqual([[-1, 1], [-5, 5]])
  })

  it('never makes more buckets than samples', () => {
    expect(minMaxBuckets([1, 2], 10)).toEqual([[1, 1], [2, 2]])
    expect(minMaxBuckets([], 4)).toEqual([])
  })
})
