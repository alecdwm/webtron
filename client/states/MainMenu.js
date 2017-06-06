import * as PIXI from 'pixi.js'

export class MainMenu {
	preload = [
		'img/gridBG.png',
		'img/gridbike-orange.png',
	]

	onEnter() {
		const {
			loader,
			scene,
		} = this.game

		const gridBG = new PIXI.Sprite(loader.resources['img/gridBG.png'].texture)
		scene.addChild(gridBG)

		const devSpriteBike = new PIXI.Sprite(loader.resources['img/gridbike-orange.png'].texture)
		devSpriteBike.anchor.set(0.5, 0.5)
		devSpriteBike.position.set(280, 280)
		devSpriteBike.rotation = 3*Math.PI/2
		scene.addChild(devSpriteBike)

		this.devSpriteBike = devSpriteBike
		this.playerName = new PIXI.Text('CLU', {
			fontFamily: '"Courier New", Courier, monospace',
			fontSize: 32,
			fill: 'white',
		})
		this.playerName.anchor.set(0.5, 0.5)
		this.playerName.position.set(280, 120)
		scene.addChild(this.playerName)

		this.ready = true
	}

	onUpdate(/*dt*/) {
		// this.devSpriteBike.y -= 100*dt
	}

	onKeyDown(key) {
		switch (key) {
		case 'Backspace':
			this.playerName.text = this.playerName.text.substring(0, this.playerName.text.length-1)
			break
		case 'ArrowLeft':
			console.log(key)
			break
		case 'ArrowRight':
			console.log(key)
		}
	}

	onKeyPress(key) {
		switch (key) {
		case 'Enter':
			this.game.changeState('Connect')
			break
		default:
			this.playerName.text += key
		}
	}
}
