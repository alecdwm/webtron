import { type CSSProperties, type RefObject, useCallback, useRef } from 'react'

import { start, turn } from '@/actions'
import { playClick } from '@/audio/sound'
import ArenaGrid from '@/components/ArenaGrid'
import Countdown from '@/components/Countdown'
import Hole from '@/components/Hole'
import Leaderboard from '@/components/Leaderboard'
import Lightcycle from '@/components/Lightcycle'
import Lightribbon from '@/components/Lightribbon'
import MenuButton from '@/components/MenuButton'
import Notifications from '@/components/Notifications'
import useArenaAnimation, { ArenaAnimationContext } from '@/hooks/useArenaAnimation'
import useArenaSounds from '@/hooks/useArenaSounds'
import useEventListener from '@/hooks/useEventListener'
import usePreloadImages from '@/hooks/usePreloadImages'
import useStore from '@/hooks/useStore'
import useStoreDispatch from '@/hooks/useStoreDispatch'
import backgroundPanel from '@/img/background-panel.svg'
import type { Arena as ArenaType, Color, Direction } from '@/types'
import { colorToHexString } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'

import styles from './Arena.module.css'

export default function Arena() {
  usePreloadImages([backgroundPanel, ...Object.values(lightcycleImages)])
  const arena = useStore((store) => store.arena)
  return arena ? <ArenaView arena={arena} /> : null
}

function ArenaView({ arena }: { arena: ArenaType }) {
  const leaderboard = useStore((store) => store.leaderboard)
  const player = useStore((store) => store.player)
  const serverTimeOffset = useStore((store) => store.serverTimeOffset)
  useArenaSounds(arena, player.id, serverTimeOffset)
  const animation = useArenaAnimation(arena, serverTimeOffset)
  const dispatch = useStoreDispatch()

  const onStart = useCallback(() => dispatch(start()), [dispatch])

  const arenaRef = useRef<HTMLDivElement>(null)
  const inRound = arena.started !== null
  useKeyControls(inRound)
  useTouchControls(inRound, arenaRef)

  const { width, height } = arena
  const colorOf = (id: string): Color => arena.players[id]?.color ?? 'white'
  const winner = arena.winner ? arena.players[arena.winner] : undefined
  const playerCount = Object.keys(arena.players).length
  // The player count is only meaningful before the first round: afterwards
  // it also counts NPCs from the round that just ended.
  const roundPlayed = Object.keys(arena.lightcycles).length > 0
  // Once every human is out, anyone can start the next round without
  // waiting for the NPCs to finish this one.
  const humanAlive = Object.entries(arena.lightcycles).some(([id, { dead }]) => !dead && !arena.players[id]?.npc)
  const canCutShort = inRound && roundPlayed && !humanAlive
  const showLobby = !inRound || canCutShort
  useStartKeys(showLobby, onStart)

  return (
    <ArenaAnimationContext.Provider value={animation}>
      <div ref={arenaRef} className={styles.arena}>
        <ArenaGrid startAt={arena.started} serverTimeOffset={serverTimeOffset} />

        {arena.holes.map(({ player_id, position, radius }) => (
          <Hole
            key={player_id}
            color={colorOf(player_id)}
            position={position}
            radius={radius}
            width={width}
            height={height}
          />
        ))}
        {Object.entries(arena.lightribbons).map(([id, { fixed }]) => (
          <Lightribbon key={id} id={id} color={colorOf(id)} fixed={fixed} width={width} height={height} />
        ))}
        {Object.entries(arena.derezzing).map(([id, { fixed, tip }]) => (
          <Lightribbon
            key={`derez-${id}`}
            color={colorOf(id)}
            fixed={fixed}
            tip={tip}
            width={width}
            height={height}
            derezzing
          />
        ))}
        {Object.entries(arena.lightcycles).map(([id, { direction, speed, dead }]) => (
          <Lightcycle
            key={id}
            id={id}
            name={arena.players[id]?.name ?? 'DISCONNECTED'}
            color={colorOf(id)}
            direction={direction}
            speed={speed}
            dead={dead}
            isSelf={id === player.id}
            inLobby={!inRound}
          />
        ))}

        <Notifications arena={arena} />

        {arena.started !== null ? (
          <Countdown key={arena.started} startAt={arena.started} serverTimeOffset={serverTimeOffset} />
        ) : null}

        {winner || showLobby ? (
          <div className={styles.overlay}>
            {winner && (
              <div className={styles.winner} style={{ '--winner': colorToHexString(winner.color) } as CSSProperties}>
                <div className={styles.winnerLabel}>Winner</div>
                <div className={styles.winnerName}>{winner.name.toUpperCase()}</div>
                {leaderboard ? <Leaderboard leaderboard={leaderboard} playerColor={player.color} /> : null}
              </div>
            )}

            {showLobby ? (
              <div className={styles.lobby}>
                {!winner && !canCutShort && <div className={styles.arenaName}>{arena.name}</div>}
                <MenuButton className={styles.startButton} onClick={onStart}>
                  {winner || canCutShort ? 'REMATCH' : 'START'}
                </MenuButton>
                {!roundPlayed && (
                  <div className={styles.lobbyInfo}>
                    {playerCount}/{arena.max_players} players
                  </div>
                )}
                <div className={styles.controlsHint}>Arrows / WASD / tap to steer</div>
              </div>
            ) : null}
          </div>
        ) : null}
      </div>
    </ArenaAnimationContext.Provider>
  )
}

function useTurnCallback() {
  const dispatch = useStoreDispatch()
  return useCallback((direction: Direction) => dispatch(turn(direction)), [dispatch])
}

// Enter or Space presses the START / REMATCH button while it is shown.
function useStartKeys(enabled: boolean, onStart: () => void) {
  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (!enabled || event.repeat || (event.key !== 'Enter' && event.key !== ' ')) return
      // A focused button or field already handles these keys itself.
      if (event.target instanceof HTMLElement && event.target.closest('button, input, textarea, select')) return
      event.preventDefault()
      playClick()
      onStart()
    },
    [enabled, onStart],
  )
  useEventListener('keydown', onKeyDown)
}

// Keyed by the physical key, so WASD works with Caps Lock or Shift held and on
// any keyboard layout.
const turnKeys: Record<string, Direction> = {
  ArrowUp: 'up',
  KeyW: 'up',
  ArrowLeft: 'left',
  KeyA: 'left',
  ArrowRight: 'right',
  KeyD: 'right',
  ArrowDown: 'down',
  KeyS: 'down',
}

function useKeyControls(inRound: boolean) {
  const onTurn = useTurnCallback()

  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (!inRound) return

      const direction = turnKeys[event.code]
      if (!direction) return
      event.preventDefault()
      // A held key would otherwise send the same turn over and over.
      if (event.repeat) return
      onTurn(direction)
    },
    [inRound, onTurn],
  )
  useEventListener('keydown', onKeyDown)
}

// Presses on a button are left to it: cancelling the touch would also cancel
// the button's click.
function isOnButton(target: EventTarget | null) {
  return target instanceof Element && target.closest('button') !== null
}

// A press turns towards whichever side of the arena's centre it lands on.
function useTouchControls(inRound: boolean, arenaRef: RefObject<HTMLDivElement | null>) {
  const onTurn = useTurnCallback()

  const turnTowards = useCallback(
    (clientX: number, clientY: number) => {
      const arena = arenaRef.current
      if (!arena) return
      const rect = arena.getBoundingClientRect()
      const xFromCenter = clientX - rect.left - rect.width / 2
      const yFromCenter = clientY - rect.top - rect.height / 2

      if (Math.abs(xFromCenter) < Math.abs(yFromCenter)) onTurn(yFromCenter < 0 ? 'up' : 'down')
      else onTurn(xFromCenter < 0 ? 'left' : 'right')
    },
    [arenaRef, onTurn],
  )

  const onTouchStart = useCallback(
    (event: TouchEvent) => {
      if (!inRound || isOnButton(event.target)) return
      const touch = event.changedTouches[0]
      if (!touch) return
      event.preventDefault()
      turnTowards(touch.clientX, touch.clientY)
    },
    [inRound, turnTowards],
  )

  const onMouseDown = useCallback(
    (event: MouseEvent) => {
      if (!inRound || isOnButton(event.target)) return
      turnTowards(event.clientX, event.clientY)
    },
    [inRound, turnTowards],
  )

  useEventListener('touchstart', onTouchStart, arenaRef)
  useEventListener('mousedown', onMouseDown, arenaRef)
}
