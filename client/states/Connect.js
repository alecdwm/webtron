class Connect {
	onEnter() {
		const { scene } = this.game

		const uiBoldTextProperties = {
			fontFamily: Webtron.uiFontFamily,
			fontWeight: 'bold',
			fontSize: Webtron.uiFontSize,
			fill: 'white',
		}

		this.connectingText = new PIXI.Text('CONNECTING', uiBoldTextProperties)
		this.connectingText.anchor.set(0.5, 0.5)
		this.connectingText.position.set(this.game.renderer.width / 2, this.game.renderer.height / 2)
		scene.addChild(this.connectingText)

		this.game.createWebsocketConnection()

		this.ready = true
	}

	onSocketOpen() {
		const { socket, playerName, playerColor } = this.game.globalstate

		socket.send(JSON.stringify({ ConfigurePlayer: { name: playerName, color: playerColor } }))

		this.game.changeState('Lobby')
	}
}

window.states = window.states || {}
window.states.Connect = Connect
