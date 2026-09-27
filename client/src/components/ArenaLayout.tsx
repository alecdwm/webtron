import { type CSSProperties, memo, useEffect, useRef, useState } from 'react'

import { playLayoutMorph } from '@/audio/sound'
import type { Round } from '@/types'
import { serverNow } from '@/utils/clock'
import { mulberry32 } from '@/utils/random'
import {
  LAYOUT_CELLS,
  SHRINK_DELAY_SECONDS,
  SHRINK_MIN_SIZE,
  SHRINK_RATE,
  cellKey,
  hasMutator,
  wallOutline,
} from '@/utils/round'

import styles from './ArenaLayout.module.css'

// How long one cell takes to rise or sink, and the longest the whole morph
// from one round's layout to the next takes, fitted into the countdown.
const CELL_MORPH_MS = 450
const MAX_MORPH_MS = 1600
// Below this much countdown left, the layout appears without morphing.
const MIN_MORPH_MS = 300

type ArenaLayoutProps = {
  round: Round
  previousBlocked: [number, number][]
  // Unix time (ms) on the server's clock at which the round starts, or null
  // between rounds.
  startAt: number | null
  serverTimeOffset: number
  width: number
}

// The round's walled-off cells, which sit on the grid beneath the trails, and
// the edges that the Shrink and Wrap mutators change. Blocks and ShrinkZone
// are both keyed by the round's start, so their keys need distinct prefixes:
// siblings sharing a key can leave one round's cells behind in the next.
function ArenaLayout({ round, previousBlocked, startAt, serverTimeOffset, width }: ArenaLayoutProps) {
  return (
    <div className={styles.layout}>
      <Blocks
        key={`blocks-${startAt ?? 'idle'}`}
        round={round}
        previous={previousBlocked}
        startAt={startAt}
        serverTimeOffset={serverTimeOffset}
      />
      {startAt !== null && hasMutator(round, 'Shrink') ? (
        <ShrinkZone key={`shrink-${startAt}`} startAt={startAt} serverTimeOffset={serverTimeOffset} width={width} />
      ) : null}
      {startAt !== null && hasMutator(round, 'Wrap') ? <div className={styles.wrapEdges} /> : null}
    </div>
  )
}

export default memo(ArenaLayout)

type BlocksProps = {
  round: Round
  previous: [number, number][]
  startAt: number | null
  serverTimeOffset: number
}

// Over the start of the countdown, cells new to this round rise out of the
// grid and cells walled off last round sink back into it, one after another
// in an order seeded by the start time, so every player sees the same one.
// Then the outline of the walls glows on.
function Blocks({ round, previous, startAt, serverTimeOffset }: BlocksProps) {
  const [{ cells, morph }] = useState(() => planMorph(round.blocked, previous, startAt, serverTimeOffset))

  // Tracks whether the morph was sounded, so StrictMode's repeated effect
  // runs don't sound it twice.
  const sounded = useRef(false)
  useEffect(() => {
    if (sounded.current || !morph || cells.every(({ change }) => change === 'kept')) return
    sounded.current = true
    playLayoutMorph(morph.totalMs / 1000)
  }, [cells, morph])
  const outline = wallOutline(round.blocked)
  const size = 100 / LAYOUT_CELLS

  return (
    <>
      {cells.map(({ cell: [x, y], change, delayMs }) => {
        const style: CSSProperties = {
          left: `${x * size}%`,
          bottom: `${y * size}%`,
          width: `${size}%`,
          height: `${size}%`,
        }
        if (morph && change !== 'kept') {
          style.animationDelay = `${delayMs}ms`
          style.animationDuration = `${morph.cellMs}ms`
        }
        const className = change === 'added' ? styles.rise : change === 'removed' ? styles.sink : styles.block
        return <div key={`${change}-${x},${y}`} className={className} style={style} />
      })}
      <svg
        className={morph ? styles.outlineOn : styles.outline}
        style={morph ? { animationDelay: `${morph.totalMs}ms` } : undefined}
        viewBox={`0 0 ${LAYOUT_CELLS} ${LAYOUT_CELLS}`}
        preserveAspectRatio="none"
      >
        <path
          d={outline.map(([x1, y1, x2, y2]) => `M${x1} ${LAYOUT_CELLS - y1}L${x2} ${LAYOUT_CELLS - y2}`).join('')}
          vectorEffect="non-scaling-stroke"
        />
      </svg>
    </>
  )
}

type Change = 'kept' | 'added' | 'removed'

// Which cells rise, sink or stay, and when each moves, fitted into the
// countdown left. Nothing moves between rounds or when too little is left.
function planMorph(
  blocked: [number, number][],
  previous: [number, number][],
  startAt: number | null,
  serverTimeOffset: number,
) {
  const remainingMs = startAt === null ? 0 : startAt - serverNow(serverTimeOffset)
  if (startAt === null || remainingMs < MIN_MORPH_MS) {
    return { cells: blocked.map((cell) => ({ cell, change: 'kept' as Change, delayMs: 0 })), morph: null }
  }

  const totalMs = Math.min(MAX_MORPH_MS, remainingMs * 0.55)
  const cellMs = Math.min(CELL_MORPH_MS, totalMs / 2)
  const random = mulberry32(startAt)
  const before = new Set(previous.map(cellKey))
  const after = new Set(blocked.map(cellKey))
  const cells = [
    ...blocked.map((cell) => ({ cell, change: (before.has(cellKey(cell)) ? 'kept' : 'added') as Change })),
    ...previous.filter((cell) => !after.has(cellKey(cell))).map((cell) => ({ cell, change: 'removed' as Change })),
  ].map((entry) => ({ ...entry, delayMs: random() * (totalMs - cellMs) }))
  return { cells, morph: { totalMs, cellMs } }
}

// The walls closing in, drawn by a CSS animation timed from the round's start
// the same way the server moves them, so it needs no updates. It is keyed by
// the start, so a round started straight after another restarts it.
function ShrinkZone({
  startAt,
  serverTimeOffset,
  width,
}: {
  startAt: number
  serverTimeOffset: number
  width: number
}) {
  const [delayMs] = useState(() => startAt + SHRINK_DELAY_SECONDS * 1000 - serverNow(serverTimeOffset))
  const maxMargin = Math.max(0, (width - SHRINK_MIN_SIZE) / 2)
  const style = {
    '--zone-inset': `${(maxMargin / width) * 100}%`,
    animationDelay: `${delayMs}ms, ${delayMs - 3000}ms, ${delayMs}ms`,
    animationDuration: `${(maxMargin / SHRINK_RATE) * 1000}ms, 3000ms, 1000ms`,
  } as CSSProperties
  return <div className={styles.zone} style={style} />
}
