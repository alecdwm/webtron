import { type CSSProperties, useCallback } from 'react'

import { connect, setPlayerColor, setPlayerName } from '@/actions'
import { playConnect, playHover, playSelect, playType } from '@/audio/sound'
import CaretLeft from '@/components/CaretLeft'
import CaretRight from '@/components/CaretRight'
import MenuButton from '@/components/MenuButton'
import MenuInput from '@/components/MenuInput'
import useEventListener from '@/hooks/useEventListener'
import usePreloadImages from '@/hooks/usePreloadImages'
import useStore from '@/hooks/useStore'
import useStoreDispatch from '@/hooks/useStoreDispatch'
import colors, { colorToHexString } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'
import resolveClassName from '@/utils/resolveClassName'
import statusFromSocketState from '@/utils/statusFromSocketState'

import styles from './MainMenu.module.css'

const MAX_PLAYER_NAME_LENGTH = 10

export default function MainMenu() {
  usePreloadImages(Object.values(lightcycleImages))

  const { player, socketState } = useStore()
  const dispatch = useStoreDispatch()

  const onConnect = useCallback(() => {
    playConnect()
    dispatch(connect())
  }, [dispatch])

  const selectColor = useCallback(
    (color) => {
      playSelect(colors.indexOf(color))
      dispatch(setPlayerColor(color))
    },
    [dispatch],
  )

  const handlePlayerNameChange = useCallback(
    (name) => {
      const playerName = name.slice(0, MAX_PLAYER_NAME_LENGTH).toLowerCase().replace(/ /g, '_').replace(/\s/g, '')
      if (playerName !== player.name) playType()
      dispatch(setPlayerName(playerName))
    },
    [dispatch, player.name],
  )

  const setNextPlayerColor = useCallback(
    () => selectColor(colors[(colors.indexOf(player.color) + 1) % colors.length]),
    [selectColor, player.color],
  )
  const setPreviousPlayerColor = useCallback(
    () => selectColor(colors[(colors.indexOf(player.color) + colors.length - 1) % colors.length]),
    [selectColor, player.color],
  )

  const onKeyDown = useCallback(
    ({ key }) => {
      switch (key) {
        case 'ArrowLeft':
          return setPreviousPlayerColor()
        case 'ArrowRight':
          return setNextPlayerColor()
      }
    },
    [setPreviousPlayerColor, setNextPlayerColor],
  )
  useEventListener('keydown', onKeyDown)

  const focusNameInput = useCallback(({ currentTarget }) => currentTarget.querySelector('input')?.focus(), [])

  const status = statusFromSocketState(socketState)

  return (
    <div className={styles.mainMenu} style={{ '--player': colorToHexString(player.color) } as CSSProperties}>
      {status ? <div className={styles.statusText}>{status}</div> : null}

      <div className={styles.title}>Enter the grid</div>

      <div className={styles.field}>
        <div className={styles.label}>Name</div>
        <div className={styles.nameBox} onClick={focusNameInput}>
          <MenuInput
            className={styles.nameInput}
            focusOnMount
            onChange={handlePlayerNameChange}
            onSubmit={onConnect}
            value={player.name}
          />
        </div>
      </div>

      <div className={styles.field}>
        <div className={styles.label}>Color</div>
        <div className={styles.colorControl}>
          <button
            className={styles.colorButton}
            onClick={setPreviousPlayerColor}
            onMouseEnter={playHover}
            aria-label="Previous color"
          >
            <CaretLeft />
          </button>
          <div className={styles.colorPreview}>
            <div className={styles.previewTrail} />
            <img className={styles.previewCycle} src={lightcycleImages[player.color]} alt="" />
          </div>
          <button
            className={styles.colorButton}
            onClick={setNextPlayerColor}
            onMouseEnter={playHover}
            aria-label="Next color"
          >
            <CaretRight />
          </button>
        </div>
        <div className={styles.swatches}>
          {colors.map((color) => (
            <button
              key={color}
              className={resolveClassName([styles.swatch, color === player.color && styles.swatchSelected])}
              style={{ '--swatch': colorToHexString(color) } as CSSProperties}
              onClick={() => selectColor(color)}
              aria-label={color}
              aria-pressed={color === player.color}
            />
          ))}
        </div>
      </div>

      <MenuButton className={styles.connectButton} onClick={onConnect}>
        CONNECT
      </MenuButton>

      <div className={styles.hint}>
        <kbd>Enter</kbd> connect &nbsp;·&nbsp; <kbd>←</kbd>
        <kbd>→</kbd> color
      </div>
    </div>
  )
}
