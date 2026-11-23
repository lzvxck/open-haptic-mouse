import { useState } from 'react'
import type { CSSProperties, PointerEvent } from 'react'
import type { TraceEvent } from './api'
import type { PlaybackState } from './playback'
import { recordPointer } from './pointerLog'
import type { PointerPhase, PointerRecord } from './pointerLog'
import { EVENT_LABEL } from './scale'

const COUNTERS: TraceEvent[] = ['down', 'rapid_down', 'up']
const LOG_LINES = 8

export function SimulatedPad({ state }: { state: PlaybackState }) {
  const { position, pressed, lastEvent, counts, hapticActive } = state
  const travel = Math.min(Math.max(position, 0), 1000) / 1000
  return (
    <section className="card">
      <h2>Click test pad</h2>
      <p className="modelled">SIMULATED firmware output, not real mouse events</p>
      <div className="pad-stage">
        <div className={`pad-key${pressed ? ' pressed' : ''}${hapticActive ? ' buzz' : ''}`} style={{ '--travel': travel } as CSSProperties}>
          {pressed ? 'PRESSED' : 'released'}
        </div>
        <div className="pad-base" />
      </div>
      <div className="pad-badges">
        <span className={`badge ev ${lastEvent ?? ''}`}>{lastEvent ? EVENT_LABEL[lastEvent] : 'no event yet'}</span>
        <span className={`badge haptic${hapticActive ? ' on' : ''}`}>haptic click (modelled)</span>
      </div>
      <dl className="counters">
        {COUNTERS.map((k) => (
          <div key={k} className={`ev ${k}`}>
            <dt>{EVENT_LABEL[k]}</dt>
            <dd>{counts[k]}</dd>
          </div>
        ))}
      </dl>
    </section>
  )
}

export function RealClickCheck() {
  const [events, setEvents] = useState<readonly PointerRecord[]>([])
  const downs = events.filter((e) => e.kind === 'down').length
  const ups = events.length - downs
  const t0 = events[0]?.at ?? 0
  const shown = events.slice(-LOG_LINES)

  const log = (phase: PointerPhase, e: PointerEvent<HTMLDivElement>) => {
    const at = e.timeStamp
    setEvents((prev) => recordPointer(prev, phase, at))
  }
  const press = (e: PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return
    e.currentTarget.setPointerCapture(e.pointerId)
    log('press', e)
  }
  const releaseButton = (e: PointerEvent<HTMLDivElement>) => {
    if (e.button === 0) log('release', e)
  }
  const releasePointer = (e: PointerEvent<HTMLDivElement>) => log('release', e)

  return (
    <section className="card">
      <h2>Real OS click check</h2>
      <p className="modelled real">real mouse events received by this window (left button)</p>
      <div className="real-target" onPointerDown={press} onPointerUp={releaseButton} onPointerCancel={releasePointer} onLostPointerCapture={releasePointer}>click here</div>
      <dl className="counters">
        <div className="ev down"><dt>pointerdown</dt><dd>{downs}</dd></div>
        <div className="ev up"><dt>pointerup</dt><dd>{ups}</dd></div>
      </dl>
      <ol className="real-log">
        {shown.map((e, i) => (
          <li key={events.length - shown.length + i}>{e.kind} at +{(e.at - t0).toFixed(1)} ms</li>
        ))}
      </ol>
      <button type="button" className="ghost" onClick={() => setEvents([])}>Reset counters</button>
      <p className="hint">Works with any mouse today. Once the STM32 board enumerates as a USB mouse, its clicks land here, so compare these counts with the simulated ones.</p>
    </section>
  )
}
