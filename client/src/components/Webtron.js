import Connect from 'components/states/Connect'
import Lobby from 'components/states/Lobby'
import MainMenu from 'components/states/MainMenu'
import useSocket from 'hooks/useSocket'
import useStore from 'hooks/useStore'
import React from 'react'

const gameStates = { MainMenu, Connect, Lobby }

export default function Webtron() {
  const { gameState } = useStore()
  const [connect, disconnect, send] = useSocket()

  const GameState = gameStates[gameState] || null
  if (GameState === null) {
    const validStates = Object.keys(gameStates).join(', ')
    console.error(`No state by name '${gameState}' exists! Valid states: ${validStates}`)
    return null
  }

  return <GameState connect={connect} disconnect={disconnect} send={send} />
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
