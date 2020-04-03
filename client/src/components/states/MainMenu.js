import { setPlayerColor, setPlayerName } from 'actions'
import MenuInput from 'components/MenuInput'
import useClassName from 'hooks/useClassName'
import useCursorBlink from 'hooks/useCursorBlink'
import usePreloadImages from 'hooks/usePreloadImages'
import useStore from 'hooks/useStore'
import useStoreDispatch from 'hooks/useStoreDispatch'
import gridbikeBlue from 'img/gridbike-blue.png'
import gridbikeGreen from 'img/gridbike-green.png'
import gridbikeOrange from 'img/gridbike-orange.png'
import gridbikePurple from 'img/gridbike-purple.png'
import gridbikeRed from 'img/gridbike-red.png'
import gridbikeWhite from 'img/gridbike-white.png'
import PropTypes from 'prop-types'
import React, { useCallback, useEffect } from 'react'
import webtronColors from 'utils/colors'

import styles from './MainMenu.module.css'

const gridbikeImages = {
  blue: gridbikeBlue,
  green: gridbikeGreen,
  orange: gridbikeOrange,
  purple: gridbikePurple,
  red: gridbikeRed,
  white: gridbikeWhite,
}

export const MAX_PLAYER_NAME_LENGTH = 12

export default function MainMenu({ connect }) {
  const { playerName, playerColor, statusText } = useStore()
  const [cursorBlink, resetCursorBlink] = useCursorBlink()
  usePreloadImages([gridbikeBlue, gridbikeGreen, gridbikeOrange, gridbikePurple, gridbikeRed, gridbikeWhite])

  const dispatch = useStoreDispatch()

  const addKeyToPlayerName = useCallback(
    (key) => {
      resetCursorBlink()

      const name = (playerName + key).slice(0, MAX_PLAYER_NAME_LENGTH)
      dispatch(setPlayerName(name))
    },
    [dispatch, resetCursorBlink, playerName],
  )

  const removeKeyFromPlayerName = useCallback(() => {
    resetCursorBlink()

    const name = playerName.slice(0, -1)
    dispatch(setPlayerName(name))
  }, [dispatch, resetCursorBlink, playerName])

  const setNextPlayerColor = useCallback(() => {
    const color = webtronColors[(webtronColors.indexOf(playerColor) + 1) % webtronColors.length]
    dispatch(setPlayerColor(color))
  }, [dispatch, playerColor])

  const setPreviousPlayerColor = useCallback(() => {
    const currentIndex = webtronColors.indexOf(playerColor)
    const color = webtronColors[(currentIndex > 0 ? currentIndex : webtronColors.length) - 1]
    dispatch(setPlayerColor(color))
  }, [dispatch, playerColor])

  const onKeyDown = useCallback(
    ({ key }) => {
      switch (key) {
        case 'Enter':
        case 'Return':
          connect()
          break

        case 'Backspace':
          removeKeyFromPlayerName()
          break

        case 'ArrowLeft':
          setPreviousPlayerColor()
          break

        case 'ArrowRight':
          setNextPlayerColor()
          break
      }
    },
    [connect, removeKeyFromPlayerName, setPreviousPlayerColor, setNextPlayerColor],
  )

  const onKeyPress = useCallback(
    (event) => {
      switch (event.key) {
        case 'Enter':
        case 'Return':
          break

        case ' ':
          event.preventDefault()
          addKeyToPlayerName('_')
          break

        default:
          addKeyToPlayerName(event.key.toLowerCase())
          break
      }
    },
    [addKeyToPlayerName],
  )

  useEffect(() => {
    window.addEventListener('keydown', onKeyDown)
    window.addEventListener('keypress', onKeyPress)

    return () => {
      window.removeEventListener('keydown', onKeyDown)
      window.removeEventListener('keypress', onKeyPress)
    }
  }, [onKeyDown, onKeyPress])

  const MainMenu = useClassName(styles.mainMenu)
  const StatusText = useClassName(styles.statusText)
  const NameLabel = useClassName(styles.nameLabel)
  const NamePreview = useClassName([styles.namePreview, cursorBlink && styles.cursorBlink])
  const ColorLabel = useClassName(styles.colorLabel)
  const ColorButtonLeft = useClassName(styles.colorButtonLeft)
  const ColorPreview = useClassName(styles.colorPreview, 'img')
  const ColorButtonRight = useClassName(styles.colorButtonRight)
  const ConnectButton = useClassName([styles.connectButton])

  return (
    <MainMenu>
      <StatusText>{statusText}</StatusText>

      <MenuInput />

      <NameLabel>NAME:</NameLabel>
      <NamePreview className={playerName === '' && styles.noName}>{playerName}</NamePreview>

      <ColorLabel>COLOR:</ColorLabel>
      <ColorButtonLeft onClick={setPreviousPlayerColor}>{'<'}</ColorButtonLeft>
      <ColorPreview src={gridbikeImages[playerColor]} />
      <ColorButtonRight onClick={setNextPlayerColor}>{'>'}</ColorButtonRight>

      <ConnectButton onClick={connect}>CONNECT</ConnectButton>
    </MainMenu>
  )
}
MainMenu.propTypes = {
  connect: PropTypes.func.isRequired,
}
