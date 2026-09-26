import { type CSSProperties, useCallback, useRef } from 'react'

import { start, turn } from '@/actions'
import ArenaGrid from '@/components/ArenaGrid'
import Countdown from '@/components/Countdown'
import Hole from '@/components/Hole'
import Leaderboard from '@/components/Leaderboard'
import Lightcycle from '@/components/Lightcycle'
import Lightribbon from '@/components/Lightribbon'
import MenuButton from '@/components/MenuButton'
import useArenaSounds from '@/hooks/useArenaSounds'
import useEventListener from '@/hooks/useEventListener'
import usePreloadImages from '@/hooks/usePreloadImages'
import useStore from '@/hooks/useStore'
import useStoreDispatch from '@/hooks/useStoreDispatch'
import backgroundPanel from '@/img/background-panel.svg'
import { colorToHexString } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'

import styles from './Arena.module.css'

export default function Arena() {
  usePreloadImages([backgroundPanel, ...Object.values(lightcycleImages)])

  const { arena, leaderboard, player } = useStore()
  useArenaSounds(arena, player.id)
  const dispatch = useStoreDispatch()

  const onStart = useCallback(() => dispatch(start()), [dispatch])

  const arenaRef = useRef<HTMLDivElement>(null)
  useKeyControls(arena.started)
  useTouchControls(arena.started, arenaRef)

  const winner = arena.winner && arena.players[arena.winner]
  const playerCount = Object.keys(arena.players).length
  // The player count is only meaningful before the first round: afterwards
  // it also counts NPCs from the round that just ended.
  const roundPlayed = Object.keys(arena.lightcycles).length > 0
  // Once every human is out, anyone can start the next round without
  // waiting for the NPCs to finish this one.
  const humanAlive = Object.entries(arena.lightcycles).some(([id, { dead }]) => !dead && !arena.players[id]?.npc)
  const canCutShort = arena.started !== null && roundPlayed && !humanAlive
  const showLobby = arena.started === null || canCutShort

  return (
    <div ref={arenaRef} className={styles.arena}>
      <ArenaGrid startAt={arena.started === null ? null : arena.started.valueOf()} />

      {arena.holes.map(({ player_id, position, radius }) => (
        <Hole
          key={player_id}
          color={arena.players[player_id] ? arena.players[player_id].color : 'white'}
          position={position}
          radius={radius}
        />
      ))}
      {Object.entries(arena.lightribbons).map(([id, { paths }]) => (
        <Lightribbon key={id} color={arena.players[id] ? arena.players[id].color : 'white'} paths={paths} />
      ))}
      {Object.entries(arena.derezzing).map(([id, { paths }]) => (
        <Lightribbon
          key={`derez-${id}`}
          color={arena.players[id] ? arena.players[id].color : 'white'}
          paths={paths}
          derezzing
        />
      ))}
      {Object.entries(arena.lightcycles).map(([id, { position, direction, speed, dead }]) => (
        <Lightcycle
          key={id}
          name={arena.players[id] ? arena.players[id].name : 'DISCONNECTED'}
          color={arena.players[id] ? arena.players[id].color : 'white'}
          position={position}
          direction={direction}
          speed={speed}
          dead={dead}
          isSelf={id === player.id}
        />
      ))}

      {arena.started !== null ? <Countdown key={arena.started.valueOf()} startAt={arena.started.valueOf()} /> : null}

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
  )
}

function useTurnCallback() {
  const dispatch = useStoreDispatch()
  return useCallback((direction) => dispatch(turn(direction)), [dispatch])
}

function useKeyControls(started) {
  const onTurn = useTurnCallback()

  const onKeyDown = useCallback(
    (event) => {
      if (!started) return

      switch (event.key) {
        case 'ArrowUp':
        case 'w':
          event.preventDefault()
          return onTurn('up')
        case 'ArrowLeft':
        case 'a':
          event.preventDefault()
          return onTurn('left')
        case 'ArrowRight':
        case 'd':
          event.preventDefault()
          return onTurn('right')
        case 'ArrowDown':
        case 's':
          event.preventDefault()
          return onTurn('down')
      }
    },
    [started, onTurn],
  )
  useEventListener('keydown', onKeyDown)
}

function useTouchControls(started, arenaRef) {
  const onTurn = useTurnCallback()

  const turnFromCenterOffset = useCallback(
    (xFromCenter, yFromCenter) => {
      const xDistance = Math.abs(xFromCenter)
      const yDistance = Math.abs(yFromCenter)

      if (xDistance < yDistance) {
        if (yFromCenter < 0) {
          return onTurn('up')
        } else {
          return onTurn('down')
        }
      } else {
        if (xFromCenter < 0) {
          return onTurn('left')
        } else {
          return onTurn('right')
        }
      }
    },
    [onTurn],
  )

  const onTouchStart = useCallback(
    (event) => {
      if (!started) return

      const touch = event.changedTouches[0]
      if (!touch) return

      event.preventDefault()

      const rect = event.currentTarget.getBoundingClientRect()
      const x = touch.clientX - rect.left
      const y = touch.clientY - rect.top
      const xFromCenter = x - rect.width / 2
      const yFromCenter = y - rect.height / 2

      turnFromCenterOffset(xFromCenter, yFromCenter)
    },
    [started, turnFromCenterOffset],
  )

  const onMouseDown = useCallback(
    (event) => {
      if (!started) return

      const rect = event.currentTarget.getBoundingClientRect()

      const x = event.clientX - rect.left
      const y = event.clientY - rect.top
      const xFromCenter = x - rect.width / 2
      const yFromCenter = y - rect.height / 2

      turnFromCenterOffset(xFromCenter, yFromCenter)
    },
    [started, turnFromCenterOffset],
  )

  useEventListener('touchstart', onTouchStart, arenaRef)
  useEventListener('mousedown', onMouseDown, arenaRef)
}
