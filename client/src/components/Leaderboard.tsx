import type { CSSProperties } from 'react'

import type { Color, Leaderboard as LeaderboardType, LeaderboardEntry } from '@/types'
import { colorToHexString } from '@/utils/colors'
import resolveClassName from '@/utils/resolveClassName'

import styles from './Leaderboard.module.css'

type LeaderboardProps = {
  leaderboard: LeaderboardType
  // Shown for the viewer's own entry when the leaderboard has no colour for it.
  playerColor: Color
}

export default function Leaderboard({ leaderboard: { top, player }, playerColor }: LeaderboardProps) {
  const playerInTop = player !== null && top.some((entry) => entry.name === player.name)

  return (
    <div className={styles.leaderboard}>
      <div className={styles.title}>Most wins</div>
      {top.length > 0 ? (
        <ol className={styles.grid}>
          {top.map((entry) => (
            <Entry key={entry.name} entry={entry} self={entry.name === player?.name} />
          ))}
        </ol>
      ) : null}
      {player !== null && !playerInTop ? (
        <ol className={styles.own}>
          <Entry entry={{ ...player, color: player.color ?? playerColor }} self />
        </ol>
      ) : null}
    </div>
  )
}

function Entry({ entry: { rank, name, wins, color }, self }: { entry: LeaderboardEntry; self: boolean }) {
  const style = color ? ({ '--entry': colorToHexString(color) } as CSSProperties) : undefined
  return (
    <li className={resolveClassName([styles.entry, self && styles.self])} style={style}>
      <span className={styles.rank}>{rank ?? '–'}</span>
      <span className={styles.name}>{name}</span>
      <span className={styles.wins}>{wins}</span>
    </li>
  )
}
