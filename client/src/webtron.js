/* jshint asi:true */
"use strict";

// Variables
var	webtron,         // global reference to phaser instance
	socket,          // global reference to socket connection
	slot = -1,       // player slot on server
	playerData = {}, // data from main menu form
	colorToHex = {   // convert color name to hex value
		'blue': 0x00c2cc,
		'green': 0x2ee5c7,
		'orange': 0xf2d91a,
		'purple': 0x8a2ee5,
		'red': 0xe5482e,
		'white': 0xe5feff,
	};

// Player Input
var keybinds = {}
var altKeybinds = {}

// Initialize
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
	document.getElementById("socketmessages").textContent = "Connecting\n"
	socket = glue(null, {
		baseURL: "/",
		forceSocketType: "WebSocket",
		reconnect: false,
	});

	socket.onMessage(function(data) {
		document.getElementById("socketmessages").textContent = data + "\n" + document.getElementById("socketmessages").textContent
		console.log("Message Received: " + data)

		processNetwork(data)
	})
});

// Network
function processNetwork(data) {
	var components = instructions[i].split(":")

	switch (components[0]) {
		case "CONNECTED":
		webtron = new Phaser.Game(560, 560, Phaser.AUTO, "webtron", {
			preload: preload,
			create: create,
		}, true);
		break;

		case "GAME_FULL":
		console.log("Game full")
		socket.close()
		if (webtron) {
			webtron.destroy()
		}
		$("#mainmenu").show()

		case "SLOT":
		console.log("Your slot is " + components[1])
		slot = parseInt(components[1])

		// players[slot] = new playerBike(100 * slot, 500, playerData.color)
		//
		// keybinds.up.onDown.add(players[slot].turnUp, players[slot])
		// keybinds.left.onDown.add(players[slot].turnLeft, players[slot])
		// keybinds.down.onDown.add(players[slot].turnDown, players[slot])
		// keybinds.right.onDown.add(players[slot].turnRight, players[slot])
		//
		// altKeybinds.up.onDown.add(players[slot].turnUp, players[slot])
		// altKeybinds.left.onDown.add(players[slot].turnLeft, players[slot])
		// altKeybinds.down.onDown.add(players[slot].turnDown, players[slot])
		// altKeybinds.right.onDown.add(players[slot].turnRight, players[slot])
		//
		// socket.send("NEWBIKE:" + slot + "-" + 100 * slot + "-500-" + playerData.colour + "-120-UP")
		break;

		case "NAME":
		console.log("A client's NAME is " + components[1])
		break;

		case "NEWBIKE":
		var params = components[1].split("-")
		players[parseInt(params[0])] = new playerBike(
			parseInt(params[1]),
			parseInt(params[2]),
			params[3],
			params[4],
			params[5])

			case "TURNBIKE":
			var params = components[1].split("-")
			switch (params[1]) {
				case "UP":
				players[parseInt(params[0])].turnUp()
				break

				case "LEFT":
				players[parseInt(params[0])].turnLeft()
				break

				case "DOWN":
				players[parseInt(params[0])].turnDown()
				break

				case "RIGHT":
				players[parseInt(params[0])].turnRight()
				break
			}

			default:
				console.log("unknown command:" + components[0])
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

	// Input
	keybinds = webtron.input.keyboard.addKeys({
		'spawn': Phaser.Keyboard.SPACEBAR,
		'up': Phaser.Keyboard.W,
		'left': Phaser.Keyboard.A,
		'down': Phaser.Keyboard.S,
		'right': Phaser.Keyboard.D,
	})
	altKeybinds = webtron.input.keyboard.addKeys({
		'up': Phaser.Keyboard.UP,
		'left': Phaser.Keyboard.LEFT,
		'down': Phaser.Keyboard.DOWN,
		'right': Phaser.Keyboard.RIGHT,
	})

	webtron.input.keyboard.addKeyCapture([ Phaser.Keyboard.SPACEBAR ]);

	keybinds.spawn   .onDown.add(function(){socket.send("SPAWN:80:80")})
	keybinds.up      .onDown.add(function(){socket.send("TURN:UP")})
	altKeybinds.up   .onDown.add(function(){socket.send("TURN:UP")})
	keybinds.left    .onDown.add(function(){socket.send("TURN:LEFT")})
	altKeybinds.left .onDown.add(function(){socket.send("TURN:LEFT")})
	keybinds.down    .onDown.add(function(){socket.send("TURN:DOWN")})
	altKeybinds.down .onDown.add(function(){socket.send("TURN:DOWN")})
	keybinds.right   .onDown.add(function(){socket.send("TURN:RIGHT")})
	altKeybinds.right.onDown.add(function(){socket.send("TURN:RIGHT")})
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
		socket.send("TURNBIKE:" + slot + "-UP")
		this.trail.setTurn(this)
		this.direction = "UP";
		this.sprite.rotation = dirToRot[this.direction]
		this.justTurned = true
	}
}
playerBike.prototype.turnLeft = function() {
	if (this.alive && this.direction != "RIGHT" && this.direction != "LEFT") {
		socket.send("TURNBIKE:" + slot + "-LEFT")
		this.trail.setTurn(this)
		this.direction = "LEFT";
		this.sprite.rotation = dirToRot[this.direction]
		this.justTurned = true
	}
}
playerBike.prototype.turnDown = function() {
	if (this.alive && this.direction != "UP" && this.direction != "DOWN") {
		socket.send("TURNBIKE:" + slot + "-DOWN")
		this.trail.setTurn(this)
		this.direction = "DOWN";
		this.sprite.rotation = dirToRot[this.direction]
		this.justTurned = true
	}
}
playerBike.prototype.turnRight = function() {
	if (this.alive && this.direction != "LEFT" && this.direction != "RIGHT") {
		socket.send("TURNBIKE:" + slot + "-RIGHT")
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
