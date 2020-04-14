import { start, turn } from 'actions'
import Lightcycle from 'components/Lightcycle'
import Lightribbon from 'components/Lightribbon'
import MenuButton from 'components/MenuButton'
import useClassName from 'hooks/useClassName'
import useEventListener from 'hooks/useEventListener'
import usePreloadImages from 'hooks/usePreloadImages'
import useStore from 'hooks/useStore'
import useStoreDispatch from 'hooks/useStoreDispatch'
import backgroundPanel from 'img/background-panel.svg'
import React, { useCallback, useRef } from 'react'
import lightcycleImages from 'utils/lightcycleImages'

import styles from './Arena.module.css'

export default function Arena() {
  usePreloadImages([backgroundPanel, ...Object.values(lightcycleImages)])

  const { arena } = useStore()
  const dispatch = useStoreDispatch()

  const onStart = useCallback(() => dispatch(start()), [dispatch])

  const arenaRef = useRef()
  useKeyControls(arena.started)
  useTouchControls(arena.started, arenaRef)

  const Arena = useClassName(styles.arena)
  const Background = useClassName([styles.background, arena.started !== null && styles.backgroundStarted])
  const StartButton = useClassName(styles.startButton, MenuButton)

  return (
    <Arena ref={arenaRef}>
      <Background style={{ backgroundImage: `url(${backgroundPanel})` }} />

      {Object.entries(arena.lightribbons).map(([id, { points }]) => (
        <Lightribbon key={id} color={arena.players[id] ? arena.players[id].color : 'white'} points={points} />
      ))}
      {Object.entries(arena.lightcycles).map(([id, { position, direction, speed, dead }]) => (
        <Lightcycle
          key={id}
          name={arena.players[id] ? arena.players[id].name : 'NULL'}
          color={arena.players[id] ? arena.players[id].color : 'white'}
          position={position}
          direction={direction}
          speed={speed}
          dead={dead}
        />
      ))}

      {arena.started === null ? <StartButton onClick={onStart}>START</StartButton> : null}
    </Arena>
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

  useEventListener('touchstart', onTouchStart, arenaRef.current)
  useEventListener('mousedown', onMouseDown, arenaRef.current)

  return arenaRef
}
