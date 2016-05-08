/// <reference path="lib/jquery.d.ts" />
/// <reference path="lib/phaser.d.ts" />

// Later we might want to make use of requirejs
// import Phaser = require('phaser')
import $ = require('jquery')

export class Webtron extends Phaser.Game
{
	constructor() {
		var width: number = 560,
			height: number = 560,
			renderer: number = Phaser.WEBGL,
			parent: string = "webtron",
			state: string = null,
			transparent: boolean = true,
			antialias: boolean = true

		super(width, height, renderer, parent, state, transparent, antialias)

		this.state.add("menu", Menu, false)
		this.state.add("game", Game, false)
		this.state.start("menu")
	}
}

export class Menu extends Phaser.State
{
	name: string
	nameMaxLength: number
	nameField: Phaser.Text
	nameButton: Phaser.Button
	nameTypeSound: Phaser.Sound

	colors: string[]
	colorsToHex: {}
	color: string
	colorText: Phaser.Text

	colorPrevText: Phaser.Text
	colorNextText: Phaser.Text
	colorSelectPrevButton: Phaser.Button
	colorSelectNextButton: Phaser.Button
	colorSelectSound: Phaser.Sound

	joinGameButton: Phaser.Button
	joinGameText: Phaser.Text

	preload() {
		this.colors = [
			"orange",
			"blue",
			"green",
			"purple",
			"red",
			"white"
		]
		this.colorsToHex = {
			'blue':   "#00c2cc",
			'green':  "#2ee5c7",
			'orange': "#f2d91a",
			'purple': "#8a2ee5",
			'red':    "#e5482e",
			'white':  "#e5feff",
		};

		this.game.load.image("button_name", "img/button_name.png")
		this.game.load.image("button_color", "img/button_color.png")
		this.game.load.image("button_join", "img/button_join.png")

		this.game.load.audiosprite("scifi5", ["sfx/scifi5.mp3"])
		this.game.load.audiosprite("keyboard_key", ["sfx/keyboard_key.mp3"])
	}

	create() {
		// default player settings
		this.name = ""
		this.nameMaxLength = 10
		this.color = this.colors[0]

		// setup input callbacks for the menu
		this.game.input.keyboard.callbackContext = this
		this.game.input.keyboard.onPressCallback = this.keyPress
		this.game.input.keyboard.onDownCallback = this.keyDown

		// create the buttons
		this.nameButton = this.game.add.button(0, 0, "button_name")
		this.colorSelectPrevButton = this.game.add.button(0, 200, "button_color", this.colorSelectPrev, this)
		this.colorSelectNextButton = this.game.add.button(280, 200, "button_color", this.colorSelectNext, this)
		this.joinGameButton = this.game.add.button(0, 460, "button_join", this.joinGame, this)

		// create the text field(s)
		this.nameField = this.game.add.text(
			this.game.width / 2,
			100,
			"_TYPE_NAME_",
			null)
		this.nameField.anchor.set(0.5, 0.5)
		this.colorPrevText = this.game.add.text(
			140,
			330,
			"←",
			null)
		this.colorPrevText.anchor.set(0.5, 0.5)
		this.colorNextText = this.game.add.text(
			this.game.width - 140,
			330,
			"→",
			null)
		this.colorNextText.anchor.set(0.5, 0.5)
		this.joinGameText = this.game.add.text(
			this.game.width / 2,
			510,
			"ENTER THE GRID",
			null)
		this.joinGameText.anchor.set(0.5, 0.5)

		// setup audio
		this.nameTypeSound = this.game.add.audio("keyboard_key")
		this.nameTypeSound.allowMultiple = true
		this.colorSelectSound = this.game.add.audio("scifi5")
		this.colorSelectSound.allowMultiple = true

		// set their colors based on the player's color
		this.updateMenuTextColors()
	}

	keyPress(char) {
		switch (char) {
			case " ":
				this.name += "_"
				break;

			default:
				this.name += char
				break;
		}
		if (this.name.length <= this.nameMaxLength) {
			this.nameTypeSound.play()
		}
		this.name = this.name.substring(0, this.nameMaxLength)
		this.nameField.setText(this.name)
	}

	keyDown(event) {
		switch (event.code) {
			case "Backspace":
				event.preventDefault()
				if (this.name.length > 0) {
					this.nameTypeSound.play()
				}
				this.name = (this.name.length > 0) ? this.name.substring(0, this.name.length - 1) : ""
				this.nameField.setText(this.name)
				break;

			case "ArrowLeft":
				this.colorSelectPrev()
				break;

			case "ArrowRight":
				this.colorSelectNext()
				break;

			case "Enter":
			case "Return":
				this.joinGame()
				break;
		}
	}

	colorSelectPrev() {
		this.colorSelectSound.play()
		this.color = this.colors[(this.colors.indexOf(this.color) - 1 >= 0) ? this.colors.indexOf(this.color) - 1 : this.colors.length - 1]
		this.updateMenuTextColors()
	}

	colorSelectNext() {
		this.colorSelectSound.play()
		this.color = this.colors[(this.colors.indexOf(this.color) + 1 < this.colors.length) ? this.colors.indexOf(this.color) + 1 : 0]
		this.updateMenuTextColors()
	}

	updateMenuTextColors() {
		$('#webtron canvas').css('border', '3px solid ' + this.colorsToHex[this.color])
		this.nameField.setStyle({
			"font": "30px \"Courier New\", Courier, monospace",
			"fill": this.colorsToHex[this.color]
		})
		this.colorPrevText.setStyle({
			"font": "50px \"Courier New\", Courier, monospace",
			"fill": this.colorsToHex[this.colors[(this.colors.indexOf(this.color) - 1 >= 0) ? this.colors.indexOf(this.color) - 1 : this.colors.length - 1]]
		})
		this.colorNextText.setStyle({
			"font": "50px \"Courier New\", Courier, monospace",
			"fill": this.colorsToHex[this.colors[(this.colors.indexOf(this.color) + 1 < this.colors.length) ? this.colors.indexOf(this.color) + 1 : 0]]
		})
		this.joinGameText.setStyle({
			"font": "30px \"Courier New\", Courier, monospace",
			"fill": this.colorsToHex[this.color]
		})
	}

	joinGame() {
		if (this.name == "") {
			this.name = "CLU"
		}
		this.game.state.clearCurrentState()
		this.game.state.start("game")
	}

	shutdown() {
		this.game.input.keyboard.callbackContext = null
		this.game.input.keyboard.onPressCallback = null
		this.game.input.keyboard.onDownCallback = null
	}
}

export class Game extends Phaser.State
{
	preload() {
		// Load Assets
		this.game.load.image("background", "img/gridBG.png")
		this.game.load.image("gridbike-blue", "img/gridbike-blue.png")
		this.game.load.image("gridbike-green", "img/gridbike-green.png")
		this.game.load.image("gridbike-orange", "img/gridbike-orange.png")
		this.game.load.image("gridbike-purple", "img/gridbike-purple.png")
		this.game.load.image("gridbike-red", "img/gridbike-red.png")
		this.game.load.image("gridbike-white", "img/gridbike-white.png")
	}

	create() {
		this.game.add.image(0, 0, "background")
	}

	shutdown() {

	}
}
