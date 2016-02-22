/* jshint asi:true */
"use strict";

// Variables
var	webtron,          // global reference to phaser instance
	socket,           // global reference to socket connection
	bikes = {},       // global reference to drawn bike sprites
	names = {},       // global reference to drawn bike name sprites
	trails = {},      // global reference to drawn trail sprites
	accumulator,      // timing network updates
	keybinds = {},    // player input
	altKeybinds = {}, // player input
	playerData = {},  // data from main menu form
	textStyle = {     // style for in-game text labels
		font: "12px \"Lucida Console\",Monaco,monospace",
		fill: "#ffffff",
	},
	colorToHex = {    // convert color name to hex value
		'blue': 0x00c2cc,
		'green': 0x2ee5c7,
		'orange': 0xf2d91a,
		'purple': 0x8a2ee5,
		'red': 0xe5482e,
		'white': 0xe5feff,
	};

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
		processNetwork(data)
	})

	socket.on("disconnected", function() {
		document.getElementById("socketmessages").textContent = "Disconnected\n"
		console.log("Disconnected")
		socket.close()
		if (webtron != null && webtron != undefined) {
			webtron.destroy()
		}
		$("#mainmenu").show()
	})
});

// Network
function processNetwork(data) {
	var components = data.split(":")

	switch (components[0]) {
	case "CONNECTED":
		document.getElementById("socketmessages").textContent = "Connected\n"
		webtron = new Phaser.Game(560, 560, Phaser.AUTO, "webtron", {
			preload: preload,
			create: create,
			update: update,
		}, true);
		break;

	case "GAME_FULL":
		document.getElementById("socketmessages").textContent = "Game Full\n"
		console.log("Game Full!")
		socket.close()
		if (webtron != null && webtron != undefined) {
			webtron.destroy()
		}
		$("#mainmenu").show()
		break;

	case "NEW_STATE":
		var json = data.replace("NEW_STATE:","")
		if (json == "") {
			break;
		}
		var newState = JSON.parse(json)

		for (var i=0; i<newState.BIKES.length; i++) {
			if (bikes[i] == null || bikes[i] == undefined) {
				bikes[i] = webtron.add.sprite(newState.BIKES[i].X, newState.BIKES[i].Y, "gridbike-" + newState.BIKES[i].COLOUR)
				bikes[i].anchor = new Phaser.Point(0.5, 0.5)
				bikes[i].rotation = newState.BIKES[i].ROT
				names[i] = webtron.add.text(newState.BIKES[i].X, newState.BIKES[i].Y-20, newState.BIKES[i].NAME, textStyle)
				names[i].anchor = new Phaser.Point(0.5, 0.5)
			} else {
				if (newState.BIKES[i].STATE == "dead") {
					bikes[i].alpha = 0.2
					names[i].alpha = 0.2
				}
				bikes[i].x = newState.BIKES[i].X
				bikes[i].y = newState.BIKES[i].Y
				bikes[i].rotation = newState.BIKES[i].ROT
				names[i].x = bikes[i].x
				names[i].y = bikes[i].y-20
			}
		}
		for (var i=0; i<newState.TRAILS.length; i++) {
			if (trails[i] == null || trails[i] == undefined) {
				trails[i] = webtron.add.graphics()
			}
			trails[i].clear()
			if (newState.TRAILS[i].STATE == "inactive") {
				trails[i].lineStyle(1, colorToHex[newState.TRAILS[i].COLOUR], 0.2)
			} else {
				trails[i].lineStyle(2, colorToHex[newState.TRAILS[i].COLOUR])
			}
			trails[i].moveTo(newState.TRAILS[i].STARTX, newState.TRAILS[i].STARTY)
			for (var v=0; v<newState.TRAILS[i].VERTS.length; v++) {
				trails[i].lineTo(
					newState.TRAILS[i].VERTS[v].X,
					newState.TRAILS[i].VERTS[v].Y)
			}
			trails[i].lineTo(
				newState.TRAILS[i].ENDX,
				newState.TRAILS[i].ENDY)
		}
		break;

	case "DISPLAY_MESSAGE":
		document.getElementById("socketmessages").textContent = data.replace("DISPLAY_MESSAGE:","") + "\n"
		break;

	default:
		console.log("unknown command:" + components[0])
		break;
	}
}

// Callbacks
function preload() {
	// Prevent pausing on focus loss (bad for multiplayer)
	webtron.stage.disableVisibilityChange = false

	// Load Assets
	webtron.load.image("background", "img/gridBG.png")
	webtron.load.image("gridbike-blue", "img/gridbike-blue.png")
	webtron.load.image("gridbike-green", "img/gridbike-green.png")
	webtron.load.image("gridbike-orange", "img/gridbike-orange.png")
	webtron.load.image("gridbike-purple", "img/gridbike-purple.png")
	webtron.load.image("gridbike-red", "img/gridbike-red.png")
	webtron.load.image("gridbike-white", "img/gridbike-white.png")
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

	keybinds.spawn   .onDown.add(function(){socket.send("SPAWN:"+playerData.name+":"+playerData.colour)})
	keybinds.up      .onDown.add(function(){socket.send("TURN:UP")})
	altKeybinds.up   .onDown.add(function(){socket.send("TURN:UP")})
	keybinds.left    .onDown.add(function(){socket.send("TURN:LEFT")})
	altKeybinds.left .onDown.add(function(){socket.send("TURN:LEFT")})
	keybinds.down    .onDown.add(function(){socket.send("TURN:DOWN")})
	altKeybinds.down .onDown.add(function(){socket.send("TURN:DOWN")})
	keybinds.right   .onDown.add(function(){socket.send("TURN:RIGHT")})
	altKeybinds.right.onDown.add(function(){socket.send("TURN:RIGHT")})

	bikes = {}
	names = {}
	trails = {}
	accumulator = 0
}

function update() {
	accumulator += webtron.time.physicsElapsed
	if (accumulator > 1/10) {
		socket.send("REQUEST_STATE")
		accumulator -= 1/10
	}
}

// old code, left for reference when implementing client-side prediction
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
	// Phaser.Sprite.call(this, x, y, "gridbike-" + this.color)
	// this.anchor = new Phaser.Point(0.5, 0.5)
	// this.rotation = dirToRot[this.direction]

	this.sprite = webtron.add.sprite(x, y, "gridbike-" + this.color)
	this.sprite.anchor = new Phaser.Point(0.5, 0.5);
	this.sprite.rotation = dirToRot[this.direction]

	this.trail = new gridTrail(this)

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

function gridTrail(bike) {
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

gridTrail.prototype.update = function(bike) {
	this.lines[this.lines.length - 1].
	setTo(this.startX, this.startY, bike.sprite.x, bike.sprite.y)

	this.graphics.lineStyle(2, this.color)
	this.graphics.lineTo(
		this.lines[this.lines.length - 1].end.x,
		this.lines[this.lines.length - 1].end.y)
}

gridTrail.prototype.setTurn = function(bike) {
	this.update(bike)
	this.startX = bike.sprite.x
	this.startY = bike.sprite.y
	this.lines.push(new Phaser.Line(
		this.startX, this.startY, this.startX, this.startY
	))
}

gridTrail.prototype.detectPtCollision = function(point) {
	for (var i = 0; i < this.lines.length; i++) {
		if (this.lines[i].pointOnSegment(point.x, point.y)) {
			return true
		}
	}
	return false
}
