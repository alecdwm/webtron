import React from 'react'
import useSocket from 'hooks/useSocket'

import MainMenu from 'components/states/MainMenu'
import Connect from 'components/states/Connect'
import Lobby from 'components/states/Lobby'
const gameStates = { MainMenu, Connect, Lobby }

export default function Webtron({ store }) {
  const [connect, send] = useSocket()

  const GameState = gameStates[store.gameState] || null
  if (GameState === null) {
    const validStates = Object.keys(gameStates).join(', ')
    console.error(`No state by name '${store.gameState}' exists! Valid states: ${validStates}`)
    return null
  }

  return (
    <>
      <GameState store={store} connect={connect} send={send} />
    </>
  )
}
