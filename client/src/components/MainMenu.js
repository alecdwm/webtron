import { setPlayerColor, setPlayerName } from 'actions'
import CaretLeft from 'components/CaretLeft'
import CaretRight from 'components/CaretRight'
import MenuButton from 'components/MenuButton'
import MenuInput from 'components/MenuInput'
import useClassName from 'hooks/useClassName'
import usePreloadImages from 'hooks/usePreloadImages'
import useStore from 'hooks/useStore'
import useStoreDispatch from 'hooks/useStoreDispatch'
import PropTypes from 'prop-types'
import React, { useCallback, useEffect } from 'react'
import webtronColors from 'utils/colors'
import gridbikeImages from 'utils/gridbikeImages'
import statusFromSocketState from 'utils/statusFromSocketState'

import styles from './MainMenu.module.css'

export const MAX_PLAYER_NAME_LENGTH = 12

export default function MainMenu({ connect }) {
  usePreloadImages(Object.values(gridbikeImages))

  const { player, socketState } = useStore()
  const dispatch = useStoreDispatch()

  const handlePlayerNameChange = useCallback(
    (name) => dispatch(setPlayerName(name.slice(0, MAX_PLAYER_NAME_LENGTH).toLowerCase().replace(/ /g, '_'))),
    [dispatch],
  )

  const setNextPlayerColor = useCallback(
    () => dispatch(setPlayerColor(webtronColors[(webtronColors.indexOf(player.color) + 1) % webtronColors.length])),
    [dispatch, player.color],
  )

  const setPreviousPlayerColor = useCallback(() => {
    dispatch(
      setPlayerColor(
        webtronColors[(webtronColors.indexOf(player.color) + webtronColors.length - 1) % webtronColors.length],
      ),
    )
  }, [dispatch, player.color])

  const onKeyDown = useCallback(
    ({ key }) => {
      switch (key) {
        case 'ArrowLeft':
          setPreviousPlayerColor()
          break

        case 'ArrowRight':
          setNextPlayerColor()
          break
      }
    },
    [setPreviousPlayerColor, setNextPlayerColor],
  )

  useEffect(() => {
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [onKeyDown])

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
      <NameInput focusOnMount onChange={handlePlayerNameChange} onSubmit={connect} value={player.name} />

      <ColorLabel>COLOR:</ColorLabel>
      <ColorButtonLeft onClick={setPreviousPlayerColor}>
        <CaretLeft />
      </ColorButtonLeft>
      <ColorPreview src={gridbikeImages[player.color]} />
      <ColorButtonRight onClick={setNextPlayerColor}>
        <CaretRight />
      </ColorButtonRight>

      <ConnectButton onClick={connect}>CONNECT</ConnectButton>
    </MainMenu>
  )
}
MainMenu.propTypes = {
  connect: PropTypes.func.isRequired,
}
