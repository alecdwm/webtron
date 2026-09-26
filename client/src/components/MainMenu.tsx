import { type ComponentType, type CSSProperties, type MouseEvent, useCallback } from 'react'

import { connect, setControlScheme, setPlayerColor, setPlayerName } from '@/actions'
import { playConnect, playHover, playSelect, playType } from '@/audio/sound'
import CaretLeft from '@/components/CaretLeft'
import CaretRight from '@/components/CaretRight'
import DirectionalIcon from '@/components/DirectionalIcon'
import MenuButton from '@/components/MenuButton'
import MenuInput from '@/components/MenuInput'
import SteeringIcon from '@/components/SteeringIcon'
import useEventListener from '@/hooks/useEventListener'
import usePreloadImages from '@/hooks/usePreloadImages'
import useStore from '@/hooks/useStore'
import useStoreDispatch from '@/hooks/useStoreDispatch'
import type { Color, ControlScheme } from '@/types'
import colors, { colorToCss } from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'
import resolveClassName from '@/utils/resolveClassName'
import statusFromSocketState from '@/utils/statusFromSocketState'

import styles from './MainMenu.module.css'

const MAX_PLAYER_NAME_LENGTH = 10

const controlSchemes: { scheme: ControlScheme; label: string; Icon: ComponentType }[] = [
  { scheme: 'directional', label: 'Directional', Icon: DirectionalIcon },
  { scheme: 'steering', label: 'Steering', Icon: SteeringIcon },
]

export default function MainMenu() {
  usePreloadImages(Object.values(lightcycleImages))

  const joinError = useStore((store) => store.joinError)
  const player = useStore((store) => store.player)
  const controlScheme = useStore((store) => store.controlScheme)
  const socketState = useStore((store) => store.socketState)
  const dispatch = useStoreDispatch()

  const onConnect = useCallback(() => {
    playConnect()
    dispatch(connect())
  }, [dispatch])

  const selectColor = useCallback(
    (color: Color) => {
      playSelect(colors.indexOf(color))
      dispatch(setPlayerColor(color))
    },
    [dispatch],
  )

  const handlePlayerNameChange = useCallback(
    (name: string) => {
      const playerName = name.slice(0, MAX_PLAYER_NAME_LENGTH).toLowerCase().replace(/ /g, '_').replace(/\s/g, '')
      if (playerName !== player.name) playType()
      dispatch(setPlayerName(playerName))
    },
    [dispatch, player.name],
  )

  const selectControlScheme = useCallback(
    (scheme: ControlScheme) => {
      playSelect(controlSchemes.findIndex((option) => option.scheme === scheme))
      dispatch(setControlScheme(scheme))
    },
    [dispatch],
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
    ({ key }: KeyboardEvent) => {
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

  const focusNameInput = useCallback(
    ({ currentTarget }: MouseEvent<HTMLElement>) => currentTarget.querySelector('input')?.focus(),
    [],
  )

  const status = statusFromSocketState(socketState)

  return (
    <div className={styles.mainMenu} style={{ '--player': colorToCss(player.color) } as CSSProperties}>
      {joinError ? (
        <div className={styles.errorText} role="alert">
          {joinError}
        </div>
      ) : status ? (
        <div className={styles.statusText}>{status}</div>
      ) : null}

      <div className={styles.title}>Enter the grid</div>

      <div className={styles.field}>
        <div className={styles.label}>Name</div>
        <div className={styles.nameBox} onClick={focusNameInput}>
          <MenuInput
            className={styles.nameInput}
            aria-label="Name"
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
              style={{ '--swatch': colorToCss(color) } as CSSProperties}
              onClick={() => selectColor(color)}
              aria-label={color}
              aria-pressed={color === player.color}
            />
          ))}
        </div>
      </div>

      <div className={styles.field}>
        <div className={styles.label}>Controls</div>
        <div className={styles.controlSchemes} role="group" aria-label="Controls">
          {controlSchemes.map(({ scheme, label, Icon }) => (
            <button
              key={scheme}
              className={resolveClassName([
                styles.controlScheme,
                scheme === controlScheme && styles.controlSchemeSelected,
              ])}
              onClick={() => selectControlScheme(scheme)}
              onMouseEnter={playHover}
              aria-pressed={scheme === controlScheme}
            >
              <Icon />
              {label}
            </button>
          ))}
        </div>
      </div>

      <MenuButton className={styles.connectButton} onClick={onConnect}>
        CONNECT
      </MenuButton>
    </div>
  )
}
