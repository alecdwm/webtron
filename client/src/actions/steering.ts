import { send } from '@/actions/socket'
import type { ArenaUpdate, Direction, Side, Thunk } from '@/types'
import { serverNow } from '@/utils/clock'

// From src/server/arena.rs: how many turns the server holds for a player, and
// how long before a round starts it begins accepting them.
const MAX_QUEUED_TURNS = 3
const EARLY_TURN_MILLISECONDS = 100

// A sent turn the server hasn't been seen to take by then was refused or
// lost, and every turn after it was steered from the wrong direction.
const SENT_TURN_TIMEOUT_MILLISECONDS = 1000

const leftOf: Record<Direction, Direction> = { up: 'left', left: 'down', down: 'right', right: 'up' }
const rightOf: Record<Direction, Direction> = { up: 'right', right: 'down', down: 'left', left: 'up' }

// Turns sent to the server that no update has shown it taking yet, oldest
// first. The server queues turns and takes one per update, so a steer made
// before earlier ones are taken has to turn from the last one sent, not from
// the direction the lightcycle is shown going: three quick lefts from up are
// sent as left, down, right.
let sentTurns: { direction: Direction; sentAt: number }[] = []

// Turns the player's lightcycle to its left or right.
export function steer(side: Side): Thunk {
  return (_, getState) => {
    const { arena, player, serverTimeOffset } = getState()
    const lightcycle = player.id !== null ? arena?.lightcycles[player.id] : undefined
    if (!arena || arena.started === null || !lightcycle || lightcycle.dead) return

    // The server refuses turns made before this, so they can't be steered
    // from.
    const now = serverNow(serverTimeOffset)
    if (now < arena.started - EARLY_TURN_MILLISECONDS) return

    const started = arena.started
    if (sentTurns.some(({ sentAt }) => now - Math.max(sentAt, started) > SENT_TURN_TIMEOUT_MILLISECONDS)) {
      sentTurns = []
    }
    if (sentTurns.length >= MAX_QUEUED_TURNS) return

    const from = sentTurns[sentTurns.length - 1]?.direction ?? lightcycle.direction
    const direction = side === 'left' ? leftOf[from] : rightOf[from]
    sentTurns.push({ direction, sentAt: now })
    send({ Turn: direction })
  }
}

// Each turn the server takes for the player comes back as a change of
// direction. A new or removed lightcycle ends whatever turns it had queued.
export function trackSentTurns(statePatch: ArenaUpdate[], playerId: string | null) {
  for (const update of statePatch) {
    if (typeof update !== 'object') continue
    if ('UpdateLightcycleDirection' in update && update.UpdateLightcycleDirection[0] === playerId) sentTurns.shift()
    if (
      ('AddLightcycle' in update && update.AddLightcycle[0] === playerId) ||
      ('RemoveLightcycle' in update && update.RemoveLightcycle === playerId) ||
      ('UpdateLightcycleApplyDeath' in update && update.UpdateLightcycleApplyDeath[0] === playerId)
    ) {
      sentTurns = []
    }
  }
}

export function forgetSentTurns() {
  sentTurns = []
}
