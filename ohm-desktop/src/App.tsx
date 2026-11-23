import { useCallback, useEffect, useRef, useState } from 'react'
import { defaultProfile, isMock, listScenarios, runScenario } from './api'
import type { ProfileDto, RunResult } from './api'
import { Chart } from './Chart'
import { RealClickCheck, SimulatedPad } from './ClickTest'
import { PlaybackBar } from './PlaybackBar'
import { stateAt } from './playback'
import { EVENT_LABEL } from './scale'
import { usePlayback } from './usePlayback'
import './App.css'

const DEBOUNCE_MS = 150

interface SliderProps {
  label: string
  value: number
  max: number
  unit?: string
  onChange: (v: number) => void
}

function Slider({ label, value, max, unit = '', onChange }: SliderProps) {
  return (
    <label className="control">
      <span className="control-head">
        <span>{label}</span>
        <output>{value}{unit}</output>
      </span>
      <input type="range" min={0} max={max} value={value} onChange={(e) => onChange(Number(e.target.value))} />
    </label>
  )
}

export default function App() {
  const [scenarios, setScenarios] = useState<string[]>([])
  const [scenario, setScenario] = useState('')
  const [profile, setProfile] = useState<ProfileDto | null>(null)
  const [result, setResult] = useState<RunResult | null>(null)
  const [error, setError] = useState<string | null>(null)
  const latest = useRef(0)

  useEffect(() => {
    Promise.all([listScenarios(), defaultProfile()])
      .then(([names, p]) => {
        setScenarios(names)
        setScenario(names[0] ?? '')
        setProfile(p)
      })
      .catch((e) => setError(String(e)))
  }, [])

  const run = useCallback((name: string, p: ProfileDto) => {
    const id = ++latest.current
    runScenario(name, p)
      .then((r) => {
        if (id !== latest.current) return
        setResult(r)
        setError(null)
      })
      .catch((e) => {
        if (id === latest.current) setError(String(e))
      })
  }, [])

  useEffect(() => {
    if (!scenario || !profile) return
    const timer = setTimeout(() => run(scenario, profile), DEBOUNCE_MS)
    return () => clearTimeout(timer)
  }, [scenario, profile, run])

  const set = (patch: Partial<ProfileDto>) => setProfile((p) => (p ? { ...p, ...patch } : p))
  const events = result?.steps.filter((s) => s.event) ?? []
  const pb = usePlayback(result)

  return (
    <div className="app">
      <aside className="panel">
        <h1>Open Haptic Mouse</h1>
        <p className="mode">{isMock ? 'Simulator mode (browser mock, not the Rust simulator)' : 'Simulator mode'}</p>
        <label className="control">
          <span className="control-head"><span>Scenario</span></span>
          <select value={scenario} onChange={(e) => setScenario(e.target.value)}>
            {scenarios.map((s) => <option key={s}>{s}</option>)}
          </select>
        </label>
        {profile && (
          <>
            <Slider label="Actuation" value={profile.actuation} max={1000} onChange={(v) => set({ actuation: v })} />
            <Slider label="Release hysteresis" value={profile.release_hysteresis} max={1000} onChange={(v) => set({ release_hysteresis: v })} />
            <Slider label="Rapid trigger window" value={profile.rapid_trigger_window_ms} max={100} unit=" ms" onChange={(v) => set({ rapid_trigger_window_ms: v })} />
            <Slider label="Haptic intensity" value={profile.haptic_intensity} max={255} onChange={(v) => set({ haptic_intensity: v })} />
          </>
        )}
        {error && <p className="error" role="alert">{error}</p>}
        <button type="button" disabled={!scenario || !profile} onClick={() => scenario && profile && run(scenario, profile)}>Run</button>
      </aside>
      <main className="main">
        {result && (
          <>
            <PlaybackBar pb={pb} />
            <Chart result={result} stale={error !== null} playheadUs={pb.t} onSeek={pb.seek} />
            <div className="checks">
              <SimulatedPad state={stateAt(result, pb.t)} />
              <RealClickCheck />
            </div>
          </>
        )}
        <section>
          <p className="modelled">MODELLED, not measured on hardware</p>
          <table>
            <thead>
              <tr><th>Time (ms)</th><th>Event</th><th>Position</th><th>Detect to trigger (µs)</th></tr>
            </thead>
            <tbody>
              {events.map((s) => (
                <tr key={s.t_us} className={`ev ${s.event}`}>
                  <td>{s.t_us / 1000}</td>
                  <td>{s.event && EVENT_LABEL[s.event]}</td>
                  <td>{s.pos}</td>
                  <td>{s.detect_to_trigger_us ?? '-'}</td>
                </tr>
              ))}
              {result && events.length === 0 && <tr><td colSpan={4}>No events with this profile</td></tr>}
            </tbody>
          </table>
        </section>
      </main>
    </div>
  )
}
