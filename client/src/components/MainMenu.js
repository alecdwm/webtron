import { connect, setPlayerColor, setPlayerName } from 'actions'
import CaretLeft from 'components/CaretLeft'
import CaretRight from 'components/CaretRight'
import MenuButton from 'components/MenuButton'
import MenuInput from 'components/MenuInput'
import useClassName from 'hooks/useClassName'
import useEventListener from 'hooks/useEventListener'
import usePreloadImages from 'hooks/usePreloadImages'
import useStore from 'hooks/useStore'
import useStoreDispatch from 'hooks/useStoreDispatch'
import React, { useCallback } from 'react'
import colors from 'utils/colors'
import lightcycleImages from 'utils/lightcycleImages'
import statusFromSocketState from 'utils/statusFromSocketState'

import styles from './MainMenu.module.css'

const MAX_PLAYER_NAME_LENGTH = 10

export default function MainMenu() {
  usePreloadImages(Object.values(lightcycleImages))

  const { player, socketState } = useStore()
  const dispatch = useStoreDispatch()

  const onConnect = useCallback(() => dispatch(connect()), [dispatch])

  const handlePlayerNameChange = useCallback(
    (name) => dispatch(setPlayerName(name.slice(0, MAX_PLAYER_NAME_LENGTH).toLowerCase().replace(/ /g, '_'))),
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

  const MainMenu = useClassName(styles.mainMenu)
  const StatusText = useClassName(styles.statusText)
  const NameLabel = useClassName(styles.nameLabel)
  const NameInput = useClassName(styles.nameInput, MenuInput)
  const ColorLabel = useClassName(styles.colorLabel)
  const ColorButtonLeft = useClassName(styles.colorButtonLeft)
  const ColorPreview = useClassName(styles.colorPreview, 'img')
  const ColorButtonRight = useClassName(styles.colorButtonRight)
  const ConnectButton = useClassName(styles.connectButton, MenuButton)

  return (
    <MainMenu>
      <StatusText>{statusFromSocketState(socketState)}</StatusText>

      <NameLabel>NAME:</NameLabel>
      <NameInput focusOnMount onChange={handlePlayerNameChange} onSubmit={onConnect} value={player.name} />

      <ColorLabel>COLOR:</ColorLabel>
      <ColorButtonLeft onClick={setPreviousPlayerColor}>
        <CaretLeft />
      </ColorButtonLeft>
      <ColorPreview src={lightcycleImages[player.color]} />
      <ColorButtonRight onClick={setNextPlayerColor}>
        <CaretRight />
      </ColorButtonRight>

      <ConnectButton onClick={onConnect}>CONNECT</ConnectButton>
    </MainMenu>
  )
}
