import { type CSSProperties, useCallback, useRef } from 'react'

import { start, turn } from '@/actions'
import Countdown from '@/components/Countdown'
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
import resolveClassName from '@/utils/resolveClassName'

import styles from './Arena.module.css'

export default function Arena() {
  usePreloadImages([backgroundPanel, ...Object.values(lightcycleImages)])

  const { arena, player } = useStore()
  useArenaSounds(arena, player.id)
  const dispatch = useStoreDispatch()

  const onStart = useCallback(() => dispatch(start()), [dispatch])

  const arenaRef = useRef<HTMLDivElement>(null)
  useKeyControls(arena.started)
  useTouchControls(arena.started, arenaRef)

  const winner = arena.winner && arena.players[arena.winner]
  const playerCount = Object.keys(arena.players).length

  return (
    <div ref={arenaRef} className={styles.arena}>
      <div
        className={resolveClassName([styles.background, arena.started !== null && styles.backgroundStarted])}
        style={{ backgroundImage: `url("${backgroundPanel}")` }}
      />

      {Object.entries(arena.lightribbons).map(([id, { points }]) => (
        <Lightribbon key={id} color={arena.players[id] ? arena.players[id].color : 'white'} points={points} />
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

      {winner || arena.started === null ? (
        <div className={styles.overlay}>
          {winner && (
            <div className={styles.winner} style={{ '--winner': colorToHexString(winner.color) } as CSSProperties}>
              <div className={styles.winnerLabel}>Winner</div>
              <div className={styles.winnerName}>{winner.name.toUpperCase()}</div>
            </div>
          )}

          {arena.started === null ? (
            <div className={styles.lobby}>
              {!winner && <div className={styles.arenaName}>{arena.name}</div>}
              <MenuButton className={styles.startButton} onClick={onStart}>
                {winner ? 'REMATCH' : 'START'}
              </MenuButton>
              <div className={styles.lobbyInfo}>
                {playerCount}/{arena.max_players} players
              </div>
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
