import { useCallback } from 'react'

import { connect, setPlayerColor, setPlayerName } from '@/actions'
import CaretLeft from '@/components/CaretLeft'
import CaretRight from '@/components/CaretRight'
import MenuButton from '@/components/MenuButton'
import MenuInput from '@/components/MenuInput'
import useEventListener from '@/hooks/useEventListener'
import usePreloadImages from '@/hooks/usePreloadImages'
import useStore from '@/hooks/useStore'
import useStoreDispatch from '@/hooks/useStoreDispatch'
import colors from '@/utils/colors'
import lightcycleImages from '@/utils/lightcycleImages'
import statusFromSocketState from '@/utils/statusFromSocketState'

import styles from './MainMenu.module.css'

const MAX_PLAYER_NAME_LENGTH = 10

export default function MainMenu() {
  usePreloadImages(Object.values(lightcycleImages))

  const { player, socketState } = useStore()
  const dispatch = useStoreDispatch()

  const onConnect = useCallback(() => dispatch(connect()), [dispatch])

  const handlePlayerNameChange = useCallback(
    (name) =>
      dispatch(
        setPlayerName(name.slice(0, MAX_PLAYER_NAME_LENGTH).toLowerCase().replace(/ /g, '_').replace(/\s/g, '')),
      ),
    [dispatch],
  )

  const setNextPlayerColor = useCallback(
    () => dispatch(setPlayerColor(colors[(colors.indexOf(player.color) + 1) % colors.length])),
    [dispatch, player.color],
  )
  const setPreviousPlayerColor = useCallback(
    () => dispatch(setPlayerColor(colors[(colors.indexOf(player.color) + colors.length - 1) % colors.length])),
    [dispatch, player.color],
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

  return (
    <div className={styles.mainMenu}>
      <div className={styles.flexSpace} />

      <div className={styles.statusText}>{statusFromSocketState(socketState)}</div>

      {statusFromSocketState(socketState) ? <div className={styles.flexSpace} /> : null}

      <div className={styles.nameLabel}>NAME</div>
      <MenuInput
        className={styles.nameInput}
        focusOnMount
        onChange={handlePlayerNameChange}
        onSubmit={onConnect}
        value={player.name}
      />

      <div className={styles.flexSpace} />

      <div className={styles.colorLabel}>COLOR</div>
      <div className={styles.colorControl}>
        <div className={styles.colorButtonLeft} onClick={setPreviousPlayerColor}>
          <CaretLeft />
        </div>
        <img className={styles.colorPreview} src={lightcycleImages[player.color]} />
        <div className={styles.colorButtonRight} onClick={setNextPlayerColor}>
          <CaretRight />
        </div>
      </div>

      <div className={styles.flexSpace} />

      <MenuButton className={styles.connectButton} onClick={onConnect}>
        CONNECT
      </MenuButton>

      <div className={styles.flexSpace} />
    </div>
  )
}
