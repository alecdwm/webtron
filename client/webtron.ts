// External libraries
/// <reference path="lib/phaser.d.ts" />

// Internal libraries
/// <reference path="modules/msg.ts" />
import Msg from 'modules/Msg'

// Game state classes
/// <reference path="states/mainmenu.ts" />
/// <reference path="states/connect.ts" />
/// <reference path="states/lobby.ts" />
/// <reference path="states/ingame.ts" />
import MainMenu from 'states/MainMenu'
import Connect from 'states/Connect'
import Lobby from 'states/Lobby'
import InGame from 'states/InGame'

// Module-level variables
export var playerName: string = "",
	playerColor: string = "",
	socket: WebSocket,
	serverMsg: string = "",
	uiFont: string = '"Courier New", Courier, monospace',
	colors: string[] = [
		"orange",
		"blue",
		"green",
		"purple",
		"red",
		"white"
	],
	colorsToHexString: {[key:string]: string} = {
		'blue':   "#00c2cc",
		'green':  "#2ee53d",
		'orange': "#f2d91a",
		'purple': "#8a2ee5",
		'red':    "#e5482e",
		'white':  "#e5feff",
	},
	colorsToHex: {[key:string]: number} = {
		'blue':   0x00c2cc,
		'green':  0x2ee53d,
		'orange': 0xf2d91a,
		'purple': 0x8a2ee5,
		'red':    0xe5482e,
		'white':  0xe5feff,
	}

// Game class
export class Game {
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
		this.state.add("lobby", Lobby, false)
		this.state.add("ingame", InGame, false)

		this.state.start("mainmenu")
	}
}
