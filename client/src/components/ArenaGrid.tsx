import { type CSSProperties, memo, useState } from 'react'

import backgroundPanel from '@/img/background-panel.svg'
import { serverNow } from '@/utils/clock'

import styles from './ArenaGrid.module.css'

// The grid is 7 by 7 copies of the background panel.
const GRID_SIZE = 7
const SQUARE_POWER_UP_MS = 650
const SWEEP_MS = 900
// Below this much countdown left, the grid appears without powering up.
const MIN_POWER_UP_MS = 300

type ArenaGridProps = {
  // Unix time (ms) on the server's clock at which the round starts, or null
  // between rounds.
  startAt: number | null
  serverTimeOffset: number
}

function ArenaGrid({ startAt, serverTimeOffset }: ArenaGridProps) {
  if (startAt === null) {
    return <div className={styles.idle} style={{ backgroundImage: `url("${backgroundPanel}")` }} />
  }
  return <PoweredGrid key={startAt} startAt={startAt} serverTimeOffset={serverTimeOffset} />
}

export default memo(ArenaGrid)

// The grid during a round. Over the countdown each square flickers on in
// turn, in an order seeded by the start time so every player sees the same
// one; then the grid dims to its in-play level as a scanline sweeps down it,
// finishing as the round starts. A countdown already partly over is played
// in the time left.
function PoweredGrid({ startAt, serverTimeOffset }: { startAt: number; serverTimeOffset: number }) {
  const [timings] = useState(() => powerUpTimings(startAt, startAt - serverNow(serverTimeOffset)))

  const squares = Array.from({ length: GRID_SIZE * GRID_SIZE }, (_, index) => {
    const style = {
      backgroundImage: `url("${backgroundPanel}")`,
      ...(timings && {
        animationDelay: `${timings.squareDelays[index]}ms`,
        animationDuration: `${timings.squareMs}ms`,
      }),
    } as CSSProperties
    return <div key={index} className={timings ? styles.powerUp : styles.square} style={style} />
  })

  const dim = timings && { animationDelay: `${timings.dimAtMs}ms`, animationDuration: `${timings.sweepMs}ms` }

  return (
    <>
      <div className={timings ? styles.dimming : styles.powered} style={dim || undefined}>
        {squares}
      </div>
      {dim ? <div className={styles.sweep} style={dim} /> : null}
    </>
  )
}

function powerUpTimings(seed: number, remainingMs: number) {
  if (remainingMs < MIN_POWER_UP_MS) return null

  // Fit the power-up into the countdown left, keeping its proportions.
  const scale = Math.min(1, remainingMs / 3000)
  const squareMs = SQUARE_POWER_UP_MS * scale
  const sweepMs = SWEEP_MS * scale
  const spreadMs = Math.max(0, remainingMs - sweepMs - squareMs)

  // Spread the squares evenly over that time in a shuffled order, with a
  // little jitter so they don't come on at a steady beat.
  const random = mulberry32(seed)
  const count = GRID_SIZE * GRID_SIZE
  const order = Array.from({ length: count }, (_, index) => index)
  for (let index = count - 1; index > 0; index--) {
    const other = Math.floor(random() * (index + 1))
    ;[order[index], order[other]] = [order[other], order[index]]
  }
  const step = spreadMs / count
  const squareDelays: number[] = []
  order.forEach((square, turn) => {
    squareDelays[square] = Math.min(spreadMs, (turn + random()) * step)
  })

  return { squareMs, sweepMs, squareDelays, dimAtMs: remainingMs - sweepMs }
}

// A small seeded pseudorandom number generator, returning numbers in [0, 1).
function mulberry32(seed: number) {
  let state = seed >>> 0
  return () => {
    state = (state + 0x6d2b79f5) >>> 0
    let t = state
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
