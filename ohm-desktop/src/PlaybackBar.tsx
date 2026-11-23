import { SPEEDS, usePlayback } from './usePlayback'

export function PlaybackBar({ pb }: { pb: ReturnType<typeof usePlayback> }) {
  return (
    <div className="playback">
      <button type="button" disabled={pb.duration <= 0} onClick={pb.playing ? pb.pause : pb.play}>
        {pb.playing ? 'Pause' : 'Play'}
      </button>
      <button type="button" className="ghost" disabled={pb.duration <= 0} onClick={pb.restart}>Restart</button>
      <span className="segmented" role="group" aria-label="Playback speed">
        {SPEEDS.map((s) => (
          <button key={s} type="button" className="ghost" aria-pressed={pb.speed === s} onClick={() => pb.setSpeed(s)}>{s}x</button>
        ))}
      </span>
      <label className="loop">
        <input type="checkbox" checked={pb.loop} onChange={(e) => pb.setLoop(e.target.checked)} /> Loop
      </label>
      <output className="clock">{(pb.t / 1000).toFixed(1)} / {pb.duration / 1000} ms</output>
    </div>
  )
}
