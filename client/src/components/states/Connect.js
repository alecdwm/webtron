import { setGameState } from '/actions'
import useClassName from '/hooks/useClassName'
import { SocketStates } from '/hooks/useSocket'
import useStoreDispatch from '/hooks/useStoreDispatch'
import React, { useEffect } from 'react'

import styles from './Connect.module.css'

export default function Connect({ store: { playerName, playerColor, socketState }, send }) {
  const dispatch = useStoreDispatch()
  useEffect(() => {
    if (socketState !== SocketStates.OPEN) return

    const joinGameId = window.location.hash.slice(1)

    send({ Matchmaking: { ConfigurePlayer: { name: playerName, color: playerColor } } })
    send({ Matchmaking: { JoinGame: joinGameId || null } })
    // dispatch(setGameState('Lobby'))
  }, [socketState, playerName, playerColor, send, dispatch])

  const ConnectingText = useClassName(styles.connectingText)

  return (
    <>
      <ConnectingText>CONNECTING</ConnectingText>
    </>
  )
}
