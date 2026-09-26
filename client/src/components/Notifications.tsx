import { type CSSProperties, useEffect, useRef, useState } from 'react'

import type { Arena, Color, DeathCause, Player } from '@/types'
import { colorToHexString } from '@/utils/colors'

import styles from './Notifications.module.css'

// How long a notification stays on screen, including fading in and out.
const NOTIFICATION_MS = 4500
const MAX_NOTIFICATIONS = 6

type Part = { text: string; color?: Color }
type Notification = { id: number; color: Color; parts: Part[] }

// A feed of fading one-line notifications in the corner of the arena, for
// deaths and for human players joining and leaving. Each line is in the
// colour of the player it is about, with other players' names in theirs.
export default function Notifications({ arena }: { arena: Arena }) {
  const [notifications, setNotifications] = useState<Notification[]>([])
  const previousArena = useRef(arena)
  const nextId = useRef(0)

  useEffect(() => {
    const before = previousArena.current
    previousArena.current = arena
    // Width is 0 until the first full arena state arrives. Skip that first
    // diff, or joining an arena would announce everyone already in it.
    if (before === arena || before.width === 0) return

    const added = describeChanges(before, arena).map((notification) => ({
      ...notification,
      id: nextId.current++,
    }))
    if (added.length === 0) return

    setNotifications((current) => [...current, ...added].slice(-MAX_NOTIFICATIONS))
    const ids = new Set(added.map(({ id }) => id))
    window.setTimeout(() => setNotifications((current) => current.filter(({ id }) => !ids.has(id))), NOTIFICATION_MS)
  }, [arena])

  return (
    <div className={styles.notifications} aria-live="polite">
      {notifications.map(({ id, color, parts }) => (
        <div
          key={id}
          className={styles.notification}
          style={
            {
              '--notification': colorToHexString(color),
              animationDuration: `${NOTIFICATION_MS}ms`,
            } as CSSProperties
          }
        >
          {parts.map(({ text, color }, index) =>
            color ? (
              <span key={index} className={styles.name} style={{ '--name': colorToHexString(color) } as CSSProperties}>
                {text}
              </span>
            ) : (
              <span key={index}>{text}</span>
            ),
          )}
        </div>
      ))}
    </div>
  )
}

function describeChanges(before: Arena, arena: Arena): Omit<Notification, 'id'>[] {
  const lookup = (id: string): Player | undefined => arena.players[id] ?? before.players[id]
  const name = (id: string): Part => {
    const player = lookup(id)
    return player ? { text: player.name.toUpperCase(), color: player.color } : { text: 'UNKNOWN', color: 'white' }
  }
  const notifications: Omit<Notification, 'id'>[] = []

  for (const [id, player] of Object.entries(arena.players)) {
    if (!(id in before.players) && !player.npc) {
      notifications.push({ color: player.color, parts: [name(id), { text: ' entered the grid' }] })
    }
  }

  for (const [id, cycle] of Object.entries(arena.lightcycles)) {
    const old = before.lightcycles[id]
    if (!old || old.dead || !cycle.dead) continue
    notifications.push({
      color: lookup(id)?.color ?? 'white',
      parts: [name(id), ...deathParts(id, cycle.death_cause, name)],
    })
  }

  for (const [id, player] of Object.entries(before.players)) {
    if (!(id in arena.players) && !player.npc) {
      notifications.push({ color: player.color, parts: [name(id), { text: ' left the grid' }] })
    }
  }

  return notifications
}

function deathParts(id: string, cause: DeathCause | null, name: (id: string) => Part): Part[] {
  if (cause === 'Boundary') return [{ text: ' hit the wall' }]
  if (cause && 'Lightcycle' in cause) return [{ text: ' crashed head-on into ' }, name(cause.Lightcycle)]
  if (cause && 'Trail' in cause) {
    if (cause.Trail === id) return [{ text: ' crashed into their own trail' }]
    return [{ text: ' crashed into ' }, name(cause.Trail), { text: "'s trail" }]
  }
  return [{ text: ' derezzed' }]
}
