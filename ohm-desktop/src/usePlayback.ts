import { useCallback, useEffect, useRef, useState } from 'react'
import type { RunResult } from './api'

export const SPEEDS = [0.25, 1, 4] as const
export type Speed = (typeof SPEEDS)[number]

export function usePlayback(trace: RunResult | null) {
  const duration = trace?.duration_us ?? 0
  const [t, setT] = useState(0)
  const [playing, setPlaying] = useState(false)
  const [speed, setSpeed] = useState<Speed>(0.25)
  const [loop, setLoop] = useState(false)
  const tRef = useRef(0)

  const seek = useCallback(
    (us: number) => {
      tRef.current = Math.min(Math.max(us, 0), duration)
      setT(tRef.current)
    },
    [duration],
  )

  const [shown, setShown] = useState(trace)
  if (shown !== trace) {
    setShown(trace)
    setT(0)
    setPlaying(false)
  }

  useEffect(() => {
    tRef.current = t
  }, [t])

  useEffect(() => {
    if (!playing || duration <= 0) return
    let last = performance.now()
    let id = 0
    const frame = (now: number) => {
      let next = tRef.current + Math.max(0, now - last) * 1000 * speed
      last = now
      if (next >= duration) {
        if (loop) {
          next %= duration
        } else {
          seek(duration)
          setPlaying(false)
          return
        }
      }
      seek(next)
      id = requestAnimationFrame(frame)
    }
    id = requestAnimationFrame(frame)
    return () => cancelAnimationFrame(id)
  }, [playing, speed, loop, duration, seek])

  const play = () => {
    if (tRef.current >= duration) seek(0)
    setPlaying(true)
  }
  const restart = () => {
    seek(0)
    setPlaying(true)
  }

  return { t, duration, playing, speed, loop, setSpeed, setLoop, seek, play, pause: () => setPlaying(false), restart }
}
