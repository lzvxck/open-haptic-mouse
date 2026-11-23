import type { RunResult } from './api'
import type { PointerEvent } from 'react'
import { EVENT_LABEL, linePoints, minMaxBuckets, timeTicksMs, timeToX, valueToY, xToTime } from './scale'

const W = 900
const LEFT = 48
const PLOT_W = W - LEFT - 16
const POS = { top: 34, height: 270 }
const WAVE = { top: 350, height: 90 }
const EVENT_LABEL_Y = { down: 14, rapid_down: 28 } as const
const AXIS_Y = WAVE.top + WAVE.height + 20
const H = AXIS_Y + 12
const POS_MAX = 1000

interface ChartProps {
  result: RunResult
  stale: boolean
  playheadUs: number
  onSeek: (us: number) => void
}

export function Chart({ result, stale, playheadUs, onSeek }: ChartProps) {
  const scrub = (e: PointerEvent<SVGSVGElement>) => {
    if (e.buttons !== 1) return
    const box = e.currentTarget.getBoundingClientRect()
    onSeek(xToTime(((e.clientX - box.left) * W) / box.width, result.duration_us, LEFT, PLOT_W))
  }
  const x = (tUs: number) => timeToX(tUs, result.duration_us, LEFT, PLOT_W)
  const yPos = (v: number) => valueToY(v, POS_MAX, POS.top, POS.height)
  const waveMid = WAVE.top + WAVE.height / 2
  const yWave = (v: number) => valueToY(v + 1, 2, WAVE.top, WAVE.height)
  const curve = linePoints(result.steps.map((s) => [x(s.t_us), yPos(s.pos)]))
  const wave = linePoints(
    minMaxBuckets(result.waveform, PLOT_W).flatMap(([lo, hi], i) => [
      [LEFT + i, yWave(hi)],
      [LEFT + i, yWave(lo)],
    ] as const),
  )
  const events = result.steps.flatMap((s) => (s.event ? [{ ...s, event: s.event }] : []))

  return (
    <svg className={stale ? 'chart stale' : 'chart'} viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Position and modelled haptic waveform over time (click or drag to set the playback time)" onPointerDown={scrub} onPointerMove={scrub}>
      {[0, 250, 500, 750, 1000].map((v) => (
        <g key={v}>
          <line className="grid" x1={LEFT} x2={LEFT + PLOT_W} y1={yPos(v)} y2={yPos(v)} />
          <text className="tick" x={LEFT - 6} y={yPos(v) + 4} textAnchor="end">{v}</text>
        </g>
      ))}
      {timeTicksMs(result.duration_us).map((ms) => (
        <g key={ms}>
          <line className="grid" x1={x(ms * 1000)} x2={x(ms * 1000)} y1={POS.top} y2={WAVE.top + WAVE.height} />
          <text className="tick" x={x(ms * 1000)} y={AXIS_Y} textAnchor="middle">{ms}</text>
        </g>
      ))}
      <text className="tick" x={LEFT + PLOT_W} y={AXIS_Y + 12} textAnchor="end">time (ms)</text>
      <text className="lane" x={LEFT + 6} y={POS.top - 8}>filtered position (0 to 1000)</text>
      <text className="lane" x={LEFT + 6} y={WAVE.top - 8}>LRA waveform (MODELLED)</text>

      <line className="threshold actuation" x1={LEFT} x2={LEFT + PLOT_W} y1={yPos(result.actuation)} y2={yPos(result.actuation)} />
      <text className="threshold-label actuation" x={LEFT + PLOT_W} y={yPos(result.actuation) - 4} textAnchor="end">actuation {result.actuation}</text>
      <line className="threshold release" x1={LEFT} x2={LEFT + PLOT_W} y1={yPos(result.release)} y2={yPos(result.release)} />
      <text className="threshold-label release" x={LEFT + PLOT_W} y={yPos(result.release) + 12} textAnchor="end">release {result.release}</text>

      <polyline className="curve" points={curve} />

      {events.map((e) => (
        <g key={e.t_us} className={`ev ${e.event}`}>
          <line x1={x(e.t_us)} x2={x(e.t_us)} y1={POS.top} y2={POS.top + POS.height} />
          <circle cx={x(e.t_us)} cy={yPos(e.pos)} r={4} />
          <text x={x(e.t_us) + 5} y={e.event === 'up' ? POS.top + POS.height - 6 : POS.top + EVENT_LABEL_Y[e.event]}>{EVENT_LABEL[e.event]}</text>
        </g>
      ))}

      <line className="grid" x1={LEFT} x2={LEFT + PLOT_W} y1={waveMid} y2={waveMid} />
      <polyline className="wave" points={wave} />
      {result.triggers.map((t) => (
        <line key={t.at_us} className="onset" x1={x(t.onset_us)} x2={x(t.onset_us)} y1={WAVE.top} y2={WAVE.top + WAVE.height} />
      ))}
      <line className="playhead" x1={x(playheadUs)} x2={x(playheadUs)} y1={POS.top} y2={WAVE.top + WAVE.height} />
    </svg>
  )
}
