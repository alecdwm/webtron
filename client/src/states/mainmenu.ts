/// <reference path="../webtron.ts" />

namespace Webtron {
	export class MainMenu extends Phaser.State {
		nameMaxLength: number
		nameField: Phaser.Text
		nameButton: Phaser.Button
		nameTypeSound: Phaser.Sound

		serverMsgText: Phaser.Text
		colorText: Phaser.Text

		colorPrevText: Phaser.Text
		colorNextText: Phaser.Text
		colorSelectText: Phaser.Text
		colorSelectPrevButton: Phaser.Button
		colorSelectNextButton: Phaser.Button
		colorSelectSound: Phaser.Sound

		enterGameButton: Phaser.Button
		enterGameText: Phaser.Text
		enterGameSound: Phaser.Sound

		preload() {
			this.game.load.image("button_name", "img/button_name.png")
			this.game.load.image("button_color", "img/button_color.png")
			this.game.load.image("button_join", "img/button_join.png")

			// TODO: Use an audio sprite
			this.game.load.audio("television_clicks", ["sfx/television_clicks.mp3"])
			this.game.load.audio("keyboard_key", ["sfx/keyboard_key.mp3"])
		}

		create() {
			// default player settings
			playerName = (playerName == "") ? "" : playerName
			playerColor = (playerColor == "") ? colors[0] : playerColor
			this.nameMaxLength = 10

			// setup input callbacks for the menu
			this.game.input.keyboard.callbackContext = this
			this.game.input.keyboard.onPressCallback = this.keyPress
			this.game.input.keyboard.onDownCallback = this.keyDown

			// create the buttons
			this.nameButton = this.game.add.button(0, 0, "button_name")
			this.colorSelectPrevButton = this.game.add.button(0, 200, "button_color", this.colorSelectPrev, this)
			this.colorSelectNextButton = this.game.add.button(280, 200, "button_color", this.colorSelectNext, this)
			this.enterGameButton = this.game.add.button(0, 460, "button_join", this.enterGame, this)

			// create the text field(s)
			this.nameField = this.game.add.text(
				this.game.width / 2,
				100,
				(playerName == "") ? "_TYPE_NAME_" : playerName,
				null)
			this.nameField.anchor.set(0.5, 0.5)
			this.colorSelectText = this.game.add.text(
				this.game.width / 2,
				306,
				"SELECT_COLOUR",
				null)
			this.colorSelectText.anchor.set(0.5, 0.5)
			this.colorPrevText = this.game.add.text(
				100,
				300,
				"←",
				null)
			this.colorPrevText.anchor.set(0.5, 0.5)
			this.colorNextText = this.game.add.text(
				this.game.width - 100,
				300,
				"→",
				null)
			this.colorNextText.anchor.set(0.5, 0.5)
			this.enterGameText = this.game.add.text(
				this.game.width / 2,
				510,
				"ENTER_THE_GRID",
				null)
			this.enterGameText.anchor.set(0.5, 0.5)
			this.serverMsgText = this.game.add.text(
				this.game.width / 2,
				130,
				serverMsg,
				null)
			this.serverMsgText.anchor.set(0.5, 0.5)

			// setup audio
			this.nameTypeSound = this.game.add.audio("keyboard_key")
			this.nameTypeSound.allowMultiple = true
			this.colorSelectSound = this.game.add.audio("television_clicks")
			this.colorSelectSound.allowMultiple = true
			this.enterGameSound = this.game.add.audio("television_clicks")
			this.enterGameSound.allowMultiple = true

			// set their colors based on the player's color
			this.updateMenuTextColors()
		}

		keyPress(char: string) {
			switch (char) {
				// spaces to underscores
				case " ":
					playerName += "_"
					break;

				// no newlines
				case "\n":
				case "\r":
					return;

				// allow other characters
				default:
					playerName += char
					break;
			}
			if (playerName.length <= this.nameMaxLength) {
				this.nameTypeSound.play()
			}
			playerName = playerName.substring(0, this.nameMaxLength)
			this.nameField.setText(playerName)
		}

		keyDown(event: any) {
			switch (event.code) {
				case "Backspace":
					event.preventDefault()
					if (playerName.length > 0) {
						this.nameTypeSound.play()
					}
					playerName = (playerName.length > 0) ? playerName.substring(0, playerName.length - 1) : ""
					this.nameField.setText(playerName)
					break;

				case "ArrowLeft":
					this.colorSelectPrev()
					break;

				case "ArrowRight":
					this.colorSelectNext()
					break;

				case "Enter":
				case "Return":
					this.enterGameSound.play()
					this.enterGame()
					break;
			}
		}

		colorSelectPrev() {
			this.colorSelectSound.play()
			playerColor = colors[(colors.indexOf(playerColor) - 1 >= 0) ? colors.indexOf(playerColor) - 1 : colors.length - 1]
			this.updateMenuTextColors()
		}

		colorSelectNext() {
			this.colorSelectSound.play()
			playerColor = colors[(colors.indexOf(playerColor) + 1 < colors.length) ? colors.indexOf(playerColor) + 1 : 0]
			this.updateMenuTextColors()
		}

		updateMenuTextColors() {
			this.nameField.setStyle({
				"font": "30px " + uiFont,
				"fill": colorsToHexString[playerColor]
			})
			this.colorSelectText.setStyle({
				"font": "30px " + uiFont,
				"fill": colorsToHexString[playerColor]
			})
			this.colorPrevText.setStyle({
				"font": "50px " + uiFont,
				"fill": colorsToHexString[colors[(colors.indexOf(playerColor) - 1 >= 0) ? colors.indexOf(playerColor) - 1 : colors.length - 1]]
			})
			this.colorNextText.setStyle({
				"font": "50px " + uiFont,
				"fill": colorsToHexString[colors[(colors.indexOf(playerColor) + 1 < colors.length) ? colors.indexOf(playerColor) + 1 : 0]]
			})
			this.enterGameText.setStyle({
				"font": "30px " + uiFont,
				"fill": colorsToHexString[playerColor]
			})
			this.serverMsgText.setStyle({
				"font": "30px " + uiFont,
				"fill": colorsToHexString[playerColor]
			})
		}

		enterGame() {
			playerName = (playerName == "") ? "ANON" : playerName
			this.game.state.start("connect")
		}

		shutdown() {
			this.game.input.keyboard.callbackContext = null
			this.game.input.keyboard.onPressCallback = null
			this.game.input.keyboard.onDownCallback = null
		}
	}
}
