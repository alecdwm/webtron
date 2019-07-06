import React, { useEffect } from 'react'
import { setGameState } from 'actions'
import useClassName from 'hooks/useClassName'
import { SocketStates } from 'hooks/useSocket'
import useStoreDispatch from 'hooks/useStoreDispatch'
import styles from './Connect.module.css'

export default function Connect({ store: { playerName, playerColor, socketState }, send }) {
  const dispatch = useStoreDispatch()
  useEffect(() => {
    if (socketState !== SocketStates.OPEN) return

    send({ Lobby: { ConfigurePlayer: { name: playerName, color: playerColor } } })
    dispatch(setGameState('Lobby'))
  }, [socketState, playerName, playerColor, send, dispatch])

  const ConnectingText = useClassName(styles.connectingText)

  return (
    <>
      <ConnectingText>CONNECTING</ConnectingText>
    </>
  )
}
