/// <reference path="lib/jquery.d.ts" />
/// <reference path="lib/phaser.d.ts" />

// Later we might want to make use of requirejs
// import Phaser = require('phaser')
import $ = require('jquery')

export module Webtron
{
	var playerName: string = "",
		playerColor: string = "",
		socket: WebSocket,
		uiFont: string = '"Courier New", Courier, monospace',
		colors: string[] = [
			"orange",
			"blue",
			"green",
			"purple",
			"red",
			"white"
		],
		colorsToHexString: {} = {
			'blue':   "#00c2cc",
			'green':  "#2ee5c7",
			'orange': "#f2d91a",
			'purple': "#8a2ee5",
			'red':    "#e5482e",
			'white':  "#e5feff",
		},
		colorsToHex: {} = {
			'blue':   0x00c2cc,
			'green':  0x2ee5c7,
			'orange': 0xf2d91a,
			'purple': 0x8a2ee5,
			'red':    0xe5482e,
			'white':  0xe5feff,
		}

	export class Game extends Phaser.Game
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

			this.state.add("mainmenu", MainMenu, false)
			this.state.add("connect", Connect, false)
			this.state.add("gamemenu", GameMenu, false)
			this.state.add("ingame", InGame, false)

			this.state.start("mainmenu")
		}
	}

	export class MainMenu extends Phaser.State
	{
		nameMaxLength: number
		nameField: Phaser.Text
		nameButton: Phaser.Button
		nameTypeSound: Phaser.Sound

		colorText: Phaser.Text

		colorPrevText: Phaser.Text
		colorNextText: Phaser.Text
		colorSelectText: Phaser.Text
		colorSelectPrevButton: Phaser.Button
		colorSelectNextButton: Phaser.Button
		colorSelectSound: Phaser.Sound

		enterGameButton: Phaser.Button
		enterGameText: Phaser.Text

		preload() {
			this.game.load.image("button_name", "img/button_name.png")
			this.game.load.image("button_color", "img/button_color.png")
			this.game.load.image("button_join", "img/button_join.png")

			// TODO: Use an audio sprite
			this.game.load.audio("scifi5", ["sfx/scifi5.mp3"])
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
					playerName += "_"
					break;

				case "\n":
				case "\r":
					break;

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

		keyDown(event) {
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
			$('#webtron canvas').css('border', '3px solid ' + colorsToHexString[playerColor])
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

	export class Connect extends Phaser.State
	{
		connectingText: Phaser.Text

		create() {
			this.connectingText = this.game.add.text(
				this.game.width / 2,
				this.game.height / 2,
				"CONNECTING",
				null)
			this.connectingText.anchor.set(0.5, 0.5)
			this.connectingText.setStyle({
				"font": "30px " + uiFont,
				"fill": colorsToHexString[playerColor]
			})

			var protocol = (window.location.protocol == "https:") ? "wss:" : "ws:",
			    hostname = window.location.hostname,
			    port     = window.location.port,
			    path     = "/ws",
			    address  = protocol + "//" + hostname + ":" + port + path

			var state = this
			socket = new WebSocket(address)
			// TODO: Show connection error / disconnect messages on client
			socket.onerror = function(event) {
				state.game.state.start("mainmenu")
			}
			socket.onclose = function(event) {
				state.game.state.start("mainmenu")
			}
			socket.onopen = function(event) {
				state.game.state.start("gamemenu")
			}
		}
	}

	export class GameMenu extends Phaser.State
	{
		create() {
			// connect to the server
			socket.onmessage = this.socketmessage
			socket.send("HELO " + playerName)
		}

		socketmessage(event) {
			console.log(event)
		}
	}

	export class InGame extends Phaser.State
	{
		// socket
		// bikes
		// names
		// trails
		// accumulator
		// keybinds
		// altKeybinds
		// playerData
		// textStyle
		// colorToHex
		// connected: boolean

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

		// create() {

		// 	this.colorToHex = {
		// 		'blue': 0x00c2cc,
		// 		'green': 0x2ee5c7,
		// 		'orange': 0xf2d91a,
		// 		'purple': 0x8a2ee5,
		// 		'red': 0xe5482e,
		// 		'white': 0xe5feff,
		// 	};

		// 	this.game.add.image(0, 0, "background")

		// 	// Input
		// 	this.keybinds = this.game.input.keyboard.addKeys({
		// 		'spawn': Phaser.Keyboard.SPACEBAR,
		// 		'up': Phaser.Keyboard.W,
		// 		'left': Phaser.Keyboard.A,
		// 		'down': Phaser.Keyboard.S,
		// 		'right': Phaser.Keyboard.D,
		// 	})
		// 	this.altKeybinds = this.game.input.keyboard.addKeys({
		// 		'up': Phaser.Keyboard.UP,
		// 		'left': Phaser.Keyboard.LEFT,
		// 		'down': Phaser.Keyboard.DOWN,
		// 		'right': Phaser.Keyboard.RIGHT,
		// 	})

		// 	this.game.input.keyboard.addKeyCapture([Phaser.Keyboard.SPACEBAR]);

		// 	var state = this

		// 	// this.keybinds.spawn.onDown.add(function() { this.socket.send("SPAWN:" + playerData.name + ":" + playerData.colour) })
		// 	this.keybinds.spawn.onDown.add(function() { state.socket.send("SPAWN:" + "FRED" + ":" + "orange") })
		// 	this.keybinds.up.onDown.add(function() { state.socket.send("TURN:UP") })
		// 	this.altKeybinds.up.onDown.add(function() { state.socket.send("TURN:UP") })
		// 	this.keybinds.left.onDown.add(function() { state.socket.send("TURN:LEFT") })
		// 	this.altKeybinds.left.onDown.add(function() { state.socket.send("TURN:LEFT") })
		// 	this.keybinds.down.onDown.add(function() { state.socket.send("TURN:DOWN") })
		// 	this.altKeybinds.down.onDown.add(function() { state.socket.send("TURN:DOWN") })
		// 	this.keybinds.right.onDown.add(function() { state.socket.send("TURN:RIGHT") })
		// 	this.altKeybinds.right.onDown.add(function() { state.socket.send("TURN:RIGHT") })

		// 	this.bikes = {}
		// 	this.names = {}
		// 	this.trails = {}
		// 	this.accumulator = 0

		// 	// Socket
		// 	var state = this
		// 	document.getElementById("socketmessages").textContent = "Connecting\n"
		// 	this.socket = glue(null, {
		// 		baseURL: "/",
		// 		forceSocketType: "WebSocket",
		// 		reconnect: false,
		// 	});

		// 	this.socket.onMessage(function(data) {
		// 		state.processNetwork(state, data)
		// 	})

		// 	this.socket.on("disconnected", function() {
		// 		document.getElementById("socketmessages").textContent = "Disconnected\n"
		// 		console.log("Disconnected")
		// 		state.socket.close()
		// 		state.game.state.clearCurrentState()
		// 		state.game.state.start("mainmenu")
		// 	})
		// }

		// update() {
		// 	if (this.connected) {
		// 		this.accumulator += this.game.time.physicsElapsed
		// 		if (this.accumulator > 1 / 10) {
		// 			this.socket.send("REQUEST_STATE")
		// 			this.accumulator -= 1 / 10
		// 		}
		// 	}
		// }

		// goToMenu() {
		// 	this.game.state.clearCurrentState()
		// 	this.game.state.start("mainmenu")
		// }

		// processNetwork(state, data) {
		// 	var components = data.split(":")

		// 	switch (components[0]) {
		// 		case "CONNECTED":
		// 			document.getElementById("socketmessages").textContent = "Connected\n"
		// 			state.connected = true
		// 			break;

		// 		case "GAME_FULL":
		// 			document.getElementById("socketmessages").textContent = "Game Full\n"
		// 			state.goToMenu()
		// 			break;

		// 		case "NEW_STATE":
		// 			var json = data.replace("NEW_STATE:", "")
		// 			if (json == "") {
		// 				break;
		// 			}
		// 			var newState = JSON.parse(json)

		// 			for (var i = 0; i < newState.BIKES.length; i++) {
		// 				if (state.bikes[i] == null || state.bikes[i] == undefined) {
		// 					state.bikes[i] = state.game.add.sprite(newState.state.bikes[i].X, newState.state.bikes[i].Y, "gridbike-" + newState.state.bikes[i].COLOUR)
		// 					state.bikes[i].anchor = new Phaser.Point(0.5, 0.5)
		// 					state.bikes[i].rotation = newState.state.bikes[i].ROT
		// 					state.names[i] = state.game.add.text(newState.state.bikes[i].X, newState.state.bikes[i].Y - 20, newState.state.bikes[i].NAME, state.textStyle)
		// 					state.names[i].anchor = new Phaser.Point(0.5, 0.5)
		// 				} else {
		// 					if (newState.state.bikes[i].STATE == "dead") {
		// 						state.bikes[i].alpha = 0.2
		// 						state.names[i].alpha = 0.2
		// 					}
		// 					state.bikes[i].x = newState.state.bikes[i].X
		// 					state.bikes[i].y = newState.state.bikes[i].Y
		// 					state.bikes[i].rotation = newState.state.bikes[i].ROT
		// 					state.names[i].x = state.bikes[i].x
		// 					state.names[i].y = state.bikes[i].y - 20
		// 				}
		// 			}
		// 			for (var i = 0; i < newState.TRAILS.length; i++) {
		// 				if (state.trails[i] == null || state.trails[i] == undefined) {
		// 					state.trails[i] = state.game.add.graphics(0, 0)
		// 				}
		// 				state.trails[i].clear()
		// 				if (newState.TRAILS[i].STATE == "inactive") {
		// 					state.trails[i].lineStyle(1, state.colorToHex[newState.TRAILS[i].COLOUR], 0.2)
		// 				} else {
		// 					state.trails[i].lineStyle(2, state.colorToHex[newState.TRAILS[i].COLOUR])
		// 				}
		// 				state.trails[i].moveTo(newState.TRAILS[i].STARTX, newState.TRAILS[i].STARTY)
		// 				for (var v = 0; v < newState.TRAILS[i].VERTS.length; v++) {
		// 					state.trails[i].lineTo(
		// 						newState.TRAILS[i].VERTS[v].X,
		// 						newState.TRAILS[i].VERTS[v].Y)
		// 				}
		// 				state.trails[i].lineTo(
		// 					newState.TRAILS[i].ENDX,
		// 					newState.TRAILS[i].ENDY)
		// 			}
		// 			break;

		// 		case "DISPLAY_MESSAGE":
		// 			document.getElementById("socketmessages").textContent = data.replace("DISPLAY_MESSAGE:", "") + "\n"
		// 			break;

		// 		default:
		// 			console.log("unknown command:" + components[0])
		// 			break;
		// 	}
		// }

		shutdown() {

		}
	}
}
