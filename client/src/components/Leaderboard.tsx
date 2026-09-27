import type { CSSProperties } from 'react'

import type { Color, Leaderboard as LeaderboardType, LeaderboardEntry } from '@/types'
import { colorToCss } from '@/utils/colors'
import resolveClassName from '@/utils/resolveClassName'

import styles from './Leaderboard.module.css'

// Entries per column: ranks 1-5 go in the first and 6-10 in the second.
const COLUMN_LENGTH = 5

type LeaderboardProps = {
  leaderboard: LeaderboardType
  // Shown for the viewer's own entry when the leaderboard has no colour for it.
  playerColor: Color
}

export default function Leaderboard({ leaderboard: { top, player }, playerColor }: LeaderboardProps) {
  const playerInTop = player !== null && top.some((entry) => entry.name === player.name)
  const columns = []
  for (let i = 0; i < top.length; i += COLUMN_LENGTH) columns.push(top.slice(i, i + COLUMN_LENGTH))

  return (
    <div className={styles.leaderboard}>
      <div className={styles.title}>Leaderboard</div>
      {columns.length > 0 ? (
        <div className={styles.columns}>
          {columns.map((column) => (
            <ol key={column[0].name} className={styles.column}>
              <li className={styles.header}>
                <span className={styles.stat}>Wins</span>
                <span className={styles.stat}>Kills</span>
              </li>
              {column.map((entry) => (
                <Entry key={entry.name} entry={entry} self={entry.name === player?.name} />
              ))}
            </ol>
          ))}
        </div>
      ) : null}
      {player !== null && !playerInTop ? (
        <ol className={resolveClassName([styles.column, styles.own])}>
          <Entry entry={{ ...player, color: player.color ?? playerColor }} self />
        </ol>
      ) : null}
    </div>
  )
}

function Entry({ entry: { rank, name, wins, kills, color }, self }: { entry: LeaderboardEntry; self: boolean }) {
  const style = color ? ({ '--entry': colorToCss(color) } as CSSProperties) : undefined
  return (
    <li className={resolveClassName([styles.entry, self && styles.self])} style={style}>
      <span className={styles.rank}>{rank ?? '–'}</span>
      <span className={styles.name}>{name}</span>
      <span className={styles.stat}>{wins}</span>
      <span className={styles.stat}>{kills}</span>
    </li>
  )
}
