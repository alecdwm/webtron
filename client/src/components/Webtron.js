import ArenaSelect from 'components/ArenaSelect'
import Connect from 'components/Connect'
import MainMenu from 'components/MainMenu'
import useSocket from 'hooks/useSocket'
import useStore from 'hooks/useStore'
import React from 'react'

const stages = { MainMenu, Connect, ArenaSelect }

export default function Webtron() {
  const { stage } = useStore()
  const [connect, disconnect, send] = useSocket()

  const Stage = stages[stage] || null
  if (Stage === null) {
    const validStages = Object.keys(stages).join(', ')
    console.error(`No stage by name '${stage}' exists! Valid stages: ${validStages}`)
    return null
  }

  return <Stage connect={connect} disconnect={disconnect} send={send} />
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
