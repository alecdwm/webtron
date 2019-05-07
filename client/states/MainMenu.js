class MainMenu {
	constructor() {
		this.preload = [
			'img/gridbike-blue.png',
			'img/gridbike-green.png',
			'img/gridbike-orange.png',
			'img/gridbike-purple.png',
			'img/gridbike-red.png',
			'img/gridbike-white.png',
		]
	}

	onEnter() {
		const { loader, scene } = this.game

		this.playerName = this.game.globalstate.playerName || ''
		this.playerColor = this.game.globalstate.playerColor || Webtron.randomColor()

		const uiTextProperties = { fontFamily: Webtron.uiFontFamily, fontSize: Webtron.uiFontSize, fill: 'white' }
		const uiBoldTextProperties = Object.assign({}, uiTextProperties, { fontWeight: 'bold' })

		this.nameLabel = new PIXI.Text('NAME:', uiTextProperties)
		this.nameLabel.anchor.set(1.0, 0.5)
		this.nameLabel.position.set(240, 210)
		scene.addChild(this.nameLabel)

		this.playerNamePreview = new PIXI.Text(`${this.playerName}_`, uiBoldTextProperties)
		this.playerNamePreview.anchor.set(0.0, 0.5)
		this.playerNamePreview.position.set(280, 210)
		scene.addChild(this.playerNamePreview)

		this.colorLabel = new PIXI.Text('COLOR:', uiTextProperties)
		this.colorLabel.anchor.set(1.0, 0.5)
		this.colorLabel.position.set(240, 270)
		scene.addChild(this.colorLabel)

		this.colorButtonLeft = new PIXI.Text('<', uiBoldTextProperties)
		this.colorButtonLeft.anchor.set(0.0, 0.5)
		this.colorButtonLeft.position.set(280, 270)
		this.colorButtonLeft.interactive = true
		this.colorButtonLeft.buttonMode = true
		this.colorButtonLeft.on('click', this.setPreviousPlayerColor.bind(this))
		scene.addChild(this.colorButtonLeft)

		this.playerColorPreview = new PIXI.Sprite(loader.resources[`img/gridbike-${this.playerColor}.png`].texture)
		this.playerColorPreview.anchor.set(0.5, 0.5)
		this.playerColorPreview.position.set(340, 270)
		this.playerColorPreview.rotation = (3 * Math.PI) / 2
		scene.addChild(this.playerColorPreview)

		this.colorButtonRight = new PIXI.Text('>', uiBoldTextProperties)
		this.colorButtonRight.anchor.set(1.0, 0.5)
		this.colorButtonRight.position.set(400, 270)
		this.colorButtonRight.interactive = true
		this.colorButtonRight.buttonMode = true
		this.colorButtonRight.on('click', this.setNextPlayerColor.bind(this))
		scene.addChild(this.colorButtonRight)

		this.enterButton = new PIXI.Text('CONNECT', uiBoldTextProperties)
		this.enterButton.anchor.set(0.5, 0.5)
		this.enterButton.position.set(280, 350)
		this.enterButton.interactive = true
		this.enterButton.buttonMode = true
		this.enterButton.on('click', this.connect.bind(this))
		scene.addChild(this.enterButton)

		this.statusText = new PIXI.Text(
			this.game.globalstate.statusText || '',
			Object.assign({}, uiBoldTextProperties, { fill: Webtron.colorToHexString['orange'] }),
		)
		this.statusText.anchor.set(0.5, 0.5)
		this.statusText.position.set(280, 105)
		scene.addChild(this.statusText)

		this.ready = true
	}

	//
	// event callbacks
	//

	onKeyDown(key) {
		switch (key) {
			case 'Enter':
			case 'Return':
				this.connect()
				break

			case 'Backspace':
				this.removeKeyFromPlayerName()
				break

			case 'ArrowLeft':
				this.setPreviousPlayerColor()
				break

			case 'ArrowRight':
				this.setNextPlayerColor()
				break
		}
	}

	onKeyPress(key) {
		switch (key) {
			case 'Enter':
			case 'Return':
				break

			case ' ':
				this.addKeyToPlayerName('_')
				break

			default:
				this.addKeyToPlayerName(key.toLowerCase())
				break
		}
	}

	//
	// helper methods
	//

	addKeyToPlayerName(key) {
		this.playerName = (this.playerName + key).slice(0, Webtron.maxPlayerNameLength)
		this.playerNamePreview.text = `${this.playerName}_`
	}
	removeKeyFromPlayerName() {
		this.playerName = this.playerName.slice(0, -1)
		this.playerNamePreview.text = `${this.playerName}_`
	}

	setNextPlayerColor() {
		const { loader } = this.game
		this.playerColor = Webtron.colors[(Webtron.colors.indexOf(this.playerColor) + 1) % Webtron.colors.length]
		this.playerColorPreview.texture = loader.resources[`img/gridbike-${this.playerColor}.png`].texture
	}
	setPreviousPlayerColor() {
		const { loader } = this.game
		const currentIndex = Webtron.colors.indexOf(this.playerColor)
		this.playerColor = Webtron.colors[(currentIndex > 0 ? currentIndex : Webtron.colors.length) - 1]
		this.playerColorPreview.texture = loader.resources[`img/gridbike-${this.playerColor}.png`].texture
	}

	connect() {
		this.game.setGlobalState({ playerName: this.playerName || 'ANON', playerColor: this.playerColor })
		this.game.changeState('Connect')
	}
}

window.states = window.states || {}
window.states.MainMenu = MainMenu
