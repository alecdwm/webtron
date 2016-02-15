/* jshint asi:true */
"use strict";

// Variables
var	webtron,         // global reference to phaser instance
	socket,          // global reference to socket connection
	players = {},    // global array of connected players
	playerData = {}, // data from main menu form
	colorToHex = {   // convert color name to hex value
		'blue': 0x00c2cc,
		'green': 0x2ee5c7,
		'orange': 0xf2d91a,
		'purple': 0x8a2ee5,
		'red': 0xe5482e,
		'white': 0xe5feff,
	},
	dirToRot = { // convert direction name to rotation value
		'UP':    0,
		'DOWN':  Math.PI,
		'RIGHT': Math.PI / 2,
		'LEFT':  3 * Math.PI / 2,
	};

// Initialize
// TODO: Create phaserjs states for menu, ingame, etc
// ALSO: Create socket in more elegant spot
$("#submit").click(function(event) {
	event.preventDefault()

	var data = $("#mainmenu").serializeArray()

	for (var i = 0; i < data.length; i++) {
		playerData[data[i].name] = data[i].value
	}

	if (playerData.name === "" || playerData.name === undefined) {
		playerData.name = "CLU"
	}

	$("#mainmenu").hide()

	// Socket
	$("#socketmessages").append("Connecting<br />")
	socket = glue(null, {
		baseURL: "/",
		forceSocketType: "WebSocket",
		reconnect: false,
	});

	socket.onMessage(function(data) {
		$("#socketmessages").append(data + "<br />")
		console.log("Message Received: " + data)

		processNetwork(data)
	})
});

// Network
function processNetwork(data) {
	var instructions = data.split(";")
	for (var i = 0; i < instructions.length; i++) {
		var components = instructions[i].split(":")
		// if (components.length != 2) {
		// 	console.log("Number of components: " + components.length + " != 2!")
		// }

		switch (components[0]) {
			case "CONNECTED":
			webtron = new Phaser.Game(560, 560, Phaser.AUTO, "webtron", {
				preload: preload,
				create: create,
				update: update,
			}, true);
			break;

			case "GAME_FULL":
			console.log("Game full")
			socket.close()
			if (webtron) {
				webtron.destroy()
			}
			$("#mainmenu").show()

			case "ID":
			console.log("Your ID is " + components[1])
			break;

			case "NAME":
			console.log("A client's NAME is " + components[1])
			break;

			default:
			console.log(components[0])
		}
	}
}

// Callbacks
function preload() {
	// Prevent pausing on focus loss (bad for multiplayer)
	webtron.stage.disableVisibilityChange = false

	// Load Assets
	webtron.load.image("background", "img/gridBG.png")
	webtron.load.image("lightbike-blue", "img/lightbike-blue.png")
	webtron.load.image("lightbike-green", "img/lightbike-green.png")
	webtron.load.image("lightbike-orange", "img/lightbike-orange.png")
	webtron.load.image("lightbike-purple", "img/lightbike-purple.png")
	webtron.load.image("lightbike-red", "img/lightbike-red.png")
	webtron.load.image("lightbike-white", "img/lightbike-white.png")
}

function create() {
	// Background
	webtron.add.image(0, 0, "background")

	// // Player
	// players[0] = new playerBike(250, 500, playerData.colour)
	//
	// // Player Input
	// var keybinds = webtron.input.keyboard.addKeys({
	// 	'up': Phaser.Keyboard.W,
	// 	'left': Phaser.Keyboard.A,
	// 	'down': Phaser.Keyboard.S,
	// 	'right': Phaser.Keyboard.D,
	// })
	// var altKeybinds = webtron.input.keyboard.addKeys({
	// 	'up': Phaser.Keyboard.UP,
	// 	'left': Phaser.Keyboard.LEFT,
	// 	'down': Phaser.Keyboard.DOWN,
	// 	'right': Phaser.Keyboard.RIGHT,
	// })
	//
	// keybinds.up.onDown.add(players[0].turnUp, players[0])
	// keybinds.left.onDown.add(players[0].turnLeft, players[0])
	// keybinds.down.onDown.add(players[0].turnDown, players[0])
	// keybinds.right.onDown.add(players[0].turnRight, players[0])
	//
	// altKeybinds.up.onDown.add(players[0].turnUp, players[0])
	// altKeybinds.left.onDown.add(players[0].turnLeft, players[0])
	// altKeybinds.down.onDown.add(players[0].turnDown, players[0])
	// altKeybinds.right.onDown.add(players[0].turnRight, players[0])
}

function update() {
	// players[0].update(webtron.time.physicsElapsed);
}

// Entities
function playerBike(x, y, color, speed, direction) {
	switch (color) {
		case "blue":
		case "green":
		case "orange":
		case "purple":
		case "red":
		case "white":
			this.color = color
			break;

		case undefined:
		case null:
			this.color = "orange"
			break;

		default:
			console.log("Error: " + color + " is not a player color!")
			break;
	}
	this.speed = (speed !== undefined && speed !== null) ? speed : 120;
	switch (direction) {
		case "UP":
		case "LEFT":
		case "DOWN":
		case "RIGHT":
			this.direction = direction
			break;

		case undefined:
		case null:
			this.direction = "UP"
			break;

		default:
			console.log("Error: " + direction + " is not a player direction!")
			break;
	}
	//
	// Phaser.Sprite.call(this, x, y, "lightbike-" + this.color)
	// this.anchor = new Phaser.Point(0.5, 0.5)
	// this.rotation = dirToRot[this.direction]

	this.sprite = webtron.add.sprite(x, y, "lightbike-" + this.color)
	this.sprite.anchor = new Phaser.Point(0.5, 0.5);
	this.sprite.rotation = dirToRot[this.direction]

	this.trail = new lightTrail(this)

	this.justTurned = true
	this.alive = true
}
playerBike.prototype.update = function(dt) {
	if (!this.alive) {
		return
	}
	if (this.direction === "UP") {
		this.sprite.y -= this.speed * dt;
	} else if (this.direction === "RIGHT") {
		this.sprite.x += this.speed * dt;
	} else if (this.direction === "DOWN") {
		this.sprite.y += this.speed * dt;
	} else if (this.direction === "LEFT") {
		this.sprite.x -= this.speed * dt;
	}
	if (this.sprite.x < 0 || this.sprite.x > webtron.width ||
		this.sprite.y < 0 || this.sprite.y > webtron.height) {
		this.die();
	}
	// needs to check all trails, not just this bike's
	if (!this.justTurned && this.trail.detectPtCollision(this.frontCollidePt())) {
		this.die();
	}
	this.trail.update(this)
	this.justTurned = false
}
playerBike.prototype.turnUp = function() {
	if (this.alive && this.direction != "DOWN" && this.direction != "UP") {
		this.trail.setTurn(this)
		this.direction = "UP";
		this.sprite.rotation = dirToRot[this.direction]
		this.justTurned = true
	}
}
playerBike.prototype.turnLeft = function() {
	if (this.alive && this.direction != "RIGHT" && this.direction != "LEFT") {
		this.trail.setTurn(this)
		this.direction = "LEFT";
		this.sprite.rotation = dirToRot[this.direction]
		this.justTurned = true
	}
}
playerBike.prototype.turnDown = function() {
	if (this.alive && this.direction != "UP" && this.direction != "DOWN") {
		this.trail.setTurn(this)
		this.direction = "DOWN";
		this.sprite.rotation = dirToRot[this.direction]
		this.justTurned = true
	}
}
playerBike.prototype.turnRight = function() {
	if (this.alive && this.direction != "LEFT" && this.direction != "RIGHT") {
		this.trail.setTurn(this)
		this.direction = "RIGHT";
		this.sprite.rotation = dirToRot[this.direction]
		this.justTurned = true
	}
}
playerBike.prototype.frontCollidePt = function() {
	switch (this.direction) {
		case "UP":
			return new Phaser.Point(this.sprite.x, this.sprite.y - 4)
			break;
		case "LEFT":
			return new Phaser.Point(this.sprite.x - 4, this.sprite.y)
			break;
		case "RIGHT":
			return new Phaser.Point(this.sprite.x + 4, this.sprite.y)
			break;
		case "DOWN":
			return new Phaser.Point(this.sprite.x, this.sprite.y + 4)
			break;
	}
	console.log("Player direction '" + this.direction + " is not valid")
	return new Phaser.Point(this.sprite.x, this.sprite.y)
}
playerBike.prototype.die = function() {
	this.alive = false
	this.speed = 0;
}

function lightTrail(bike) {
	switch (bike.color) {
		case "blue":
		case "green":
		case "orange":
		case "purple":
		case "red":
		case "white":
			this.color = colorToHex[bike.color]
			break;

		case null:
			this.color = colorToHex.orange
			break;

		default:
			console.log("Error: " + bike.color + " is not a trail color!")
			break;
	}
	this.startX = bike.sprite.x
	this.startY = bike.sprite.y
	this.lines = [new Phaser.Line(
		this.startX, this.startY, this.startX, this.startY
	)]
	this.graphics = webtron.add.graphics()
	this.graphics.lineStyle(2, this.color)
	this.graphics.moveTo(this.startX, this.startY)
}

lightTrail.prototype.update = function(bike) {
	this.lines[this.lines.length - 1].
	setTo(this.startX, this.startY, bike.sprite.x, bike.sprite.y)

	this.graphics.lineStyle(2, this.color)
	this.graphics.lineTo(
		this.lines[this.lines.length - 1].end.x,
		this.lines[this.lines.length - 1].end.y)
}

lightTrail.prototype.setTurn = function(bike) {
	this.update(bike)
	this.startX = bike.sprite.x
	this.startY = bike.sprite.y
	this.lines.push(new Phaser.Line(
		this.startX, this.startY, this.startX, this.startY
	))
}

lightTrail.prototype.detectPtCollision = function(point) {
	for (var i = 0; i < this.lines.length; i++) {
		if (this.lines[i].pointOnSegment(point.x, point.y)) {
			return true
		}
	}
	return false
}
