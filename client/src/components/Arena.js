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
import React, { useCallback } from 'react'
import lightcycleImages from 'utils/lightcycleImages'

import styles from './Arena.module.css'

export default function Arena() {
  usePreloadImages([backgroundPanel, ...Object.values(lightcycleImages)])

  const { arena } = useStore()
  const dispatch = useStoreDispatch()

  const onStart = useCallback(() => dispatch(start()), [dispatch])
  const onTurn = useCallback((direction) => dispatch(turn(direction)), [dispatch])

  const onKeyDown = useCallback(
    (event) => {
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
    [onTurn],
  )
  useEventListener('keydown', onKeyDown)

  const Arena = useClassName(styles.arena)
  const Background = useClassName([styles.background, arena.started !== null && styles.backgroundStarted])
  const StartButton = useClassName(styles.startButton, MenuButton)

  return (
    <Arena>
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
