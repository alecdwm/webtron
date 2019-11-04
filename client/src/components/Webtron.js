import React, { useEffect } from 'react'
import useSocket from 'hooks/useSocket'

import MainMenu from 'components/states/MainMenu'
import Connect from 'components/states/Connect'
import Lobby from 'components/states/Lobby'
const gameStates = { MainMenu, Connect, Lobby }

export default function Webtron({ store }) {
  const [connect, disconnect, send] = useSocket()

  useEffect(() => {
    connect()
  }, [])

  const GameState = gameStates[store.gameState] || null
  if (GameState === null) {
    const validStates = Object.keys(gameStates).join(', ')
    console.error(`No state by name '${store.gameState}' exists! Valid states: ${validStates}`)
    return null
  }

  return (
    <>
      <GameState store={store} connect={connect} disconnect={disconnect} send={send} />
    </>
  )
}

//	update() {
//		this.updateNow = Date.now()
//		const dt = (this.updateNow - this.updateThen) / 1000.0
//		this.updateThen = this.updateNow

//		if (this.state && this.state.onUpdate && this.state.ready) {
//			this.state.onUpdate.call(this.state, dt)
//		}
//		this.renderer.render(this.scene)

//		window.requestAnimationFrame(this.update.bind(this))
//	}
