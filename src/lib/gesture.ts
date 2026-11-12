// Fluid-interface math (WWDC 2018 "Designing Fluid Interfaces").

/** Distance a flick travels before it stops, for a release velocity in px/s. */
export function project(velocity: number, decelerationRate = 0.998): number {
  return ((velocity / 1000) * decelerationRate) / (1 - decelerationRate)
}

/** Resistance past a boundary: follows less the further you pull. */
export function rubberband(overshoot: number, dimension: number, constant = 0.55): number {
  return (overshoot * dimension * constant) / (dimension + constant * Math.abs(overshoot))
}

/** Velocity (px/s) from the last ~100 ms of pointer samples. */
export function velocity(samples: { t: number; y: number }[]): number {
  const last = samples[samples.length - 1]
  const first = samples.find((s) => last.t - s.t <= 100) ?? samples[0]
  const dt = last.t - first.t
  return dt > 0 ? ((last.y - first.y) / dt) * 1000 : 0
}
