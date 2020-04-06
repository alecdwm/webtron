import gridBG from 'img/gridBG.png'
import React from 'react'
import gridbikeImages from 'utils/gridbikeImages'

export default function Arena() {
  return (
    <>
      <img src={gridBG} style={{ position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)' }} />
      <img
        src={gridbikeImages['orange']}
        style={{ position: 'absolute', top: 160, left: 400, transform: 'translate(-50%, -50%) rotate(180deg)' }}
      />
      <img
        src={gridbikeImages['green']}
        style={{ position: 'absolute', top: 320, left: 160, transform: 'translate(-50%, -50%) rotate(270deg)' }}
      />
    </>
  )
}

// namespace Webtron {
// 	export class InGame extends Phaser.State {
// 		// socket
// 		// bikes
// 		// names
// 		// trails
// 		// accumulator
// 		// keybinds
// 		// altKeybinds
// 		// playerData
// 		// textStyle
// 		// colorToHex
// 		// connected: boolean

// 		preload() {
// 			// Load Assets
// 			this.game.load.image("background", "img/gridBG.png")
// 			this.game.load.image("gridbike-blue", "img/gridbike-blue.png")
// 			this.game.load.image("gridbike-green", "img/gridbike-green.png")
// 			this.game.load.image("gridbike-orange", "img/gridbike-orange.png")
// 			this.game.load.image("gridbike-purple", "img/gridbike-purple.png")
// 			this.game.load.image("gridbike-red", "img/gridbike-red.png")
// 			this.game.load.image("gridbike-white", "img/gridbike-white.png")
// 		}

// 		create() {

// 			// 	this.colorToHex = {
// 			// 		'blue': 0x00c2cc,
// 			// 		'green': 0x2ee5c7,
// 			// 		'orange': 0xf2d91a,
// 			// 		'purple': 0x8a2ee5,
// 			// 		'red': 0xe5482e,
// 			// 		'white': 0xe5feff,
// 			// 	};

// 			this.game.add.image(0, 0, "background")

// 			// 	// Input
// 			// 	this.keybinds = this.game.input.keyboard.addKeys({
// 			// 		'spawn': Phaser.Keyboard.SPACEBAR,
// 			// 		'up': Phaser.Keyboard.W,
// 			// 		'left': Phaser.Keyboard.A,
// 			// 		'down': Phaser.Keyboard.S,
// 			// 		'right': Phaser.Keyboard.D,
// 			// 	})
// 			// 	this.altKeybinds = this.game.input.keyboard.addKeys({
// 			// 		'up': Phaser.Keyboard.UP,
// 			// 		'left': Phaser.Keyboard.LEFT,
// 			// 		'down': Phaser.Keyboard.DOWN,
// 			// 		'right': Phaser.Keyboard.RIGHT,
// 			// 	})

// 			// 	this.game.input.keyboard.addKeyCapture([Phaser.Keyboard.SPACEBAR]);

// 			// 	var state = this

// 			// 	// this.keybinds.spawn.onDown.add(function() { this.socket.send("SPAWN:" + playerData.name + ":" + playerData.colour) })
// 			// 	this.keybinds.spawn.onDown.add(function() { state.socket.send("SPAWN:" + "FRED" + ":" + "orange") })
// 			// 	this.keybinds.up.onDown.add(function() { state.socket.send("TURN:UP") })
// 			// 	this.altKeybinds.up.onDown.add(function() { state.socket.send("TURN:UP") })
// 			// 	this.keybinds.left.onDown.add(function() { state.socket.send("TURN:LEFT") })
// 			// 	this.altKeybinds.left.onDown.add(function() { state.socket.send("TURN:LEFT") })
// 			// 	this.keybinds.down.onDown.add(function() { state.socket.send("TURN:DOWN") })
// 			// 	this.altKeybinds.down.onDown.add(function() { state.socket.send("TURN:DOWN") })
// 			// 	this.keybinds.right.onDown.add(function() { state.socket.send("TURN:RIGHT") })
// 			// 	this.altKeybinds.right.onDown.add(function() { state.socket.send("TURN:RIGHT") })

// 			// 	this.bikes = {}
// 			// 	this.names = {}
// 			// 	this.trails = {}
// 			// 	this.accumulator = 0

// 			// 	// Socket
// 			// 	var state = this
// 			// 	document.getElementById("socketmessages").textContent = "Connecting\n"
// 			// 	this.socket = glue(null, {
// 			// 		baseURL: "/",
// 			// 		forceSocketType: "WebSocket",
// 			// 		reconnect: false,
// 			// 	});

// 			// 	this.socket.onMessage(function(data) {
// 			// 		state.processNetwork(state, data)
// 			// 	})

// 			// 	this.socket.on("disconnected", function() {
// 			// 		document.getElementById("socketmessages").textContent = "Disconnected\n"
// 			// 		console.log("Disconnected")
// 			// 		state.socket.close()
// 			// 		state.game.state.clearCurrentState()
// 			// 		state.game.state.start("mainmenu")
// 			// 	})
// 		}

// 		// update() {
// 		// 	if (this.connected) {
// 		// 		this.accumulator += this.game.time.physicsElapsed
// 		// 		if (this.accumulator > 1 / 10) {
// 		// 			this.socket.send("REQUEST_STATE")
// 		// 			this.accumulator -= 1 / 10
// 		// 		}
// 		// 	}
// 		// }

// 		// goToMenu() {
// 		// 	this.game.state.clearCurrentState()
// 		// 	this.game.state.start("mainmenu")
// 		// }

// 		// processNetwork(state, data) {
// 		// 	var components = data.split(":")

// 		// 	switch (components[0]) {
// 		// 		case "CONNECTED":
// 		// 			document.getElementById("socketmessages").textContent = "Connected\n"
// 		// 			state.connected = true
// 		// 			break;

// 		// 		case "GAME_FULL":
// 		// 			document.getElementById("socketmessages").textContent = "Game Full\n"
// 		// 			state.goToMenu()
// 		// 			break;

// 		// 		case "NEW_STATE":
// 		// 			var json = data.replace("NEW_STATE:", "")
// 		// 			if (json == "") {
// 		// 				break;
// 		// 			}
// 		// 			var newState = JSON.parse(json)

// 		// 			for (var i = 0; i < newState.BIKES.length; i++) {
// 		// 				if (state.bikes[i] == null || state.bikes[i] == undefined) {
// 		// 					state.bikes[i] = state.game.add.sprite(newState.state.bikes[i].X, newState.state.bikes[i].Y, "gridbike-" + newState.state.bikes[i].COLOUR)
// 		// 					state.bikes[i].anchor = new Phaser.Point(0.5, 0.5)
// 		// 					state.bikes[i].rotation = newState.state.bikes[i].ROT
// 		// 					state.names[i] = state.game.add.text(newState.state.bikes[i].X, newState.state.bikes[i].Y - 20, newState.state.bikes[i].NAME, state.textStyle)
// 		// 					state.names[i].anchor = new Phaser.Point(0.5, 0.5)
// 		// 				} else {
// 		// 					if (newState.state.bikes[i].STATE == "dead") {
// 		// 						state.bikes[i].alpha = 0.2
// 		// 						state.names[i].alpha = 0.2
// 		// 					}
// 		// 					state.bikes[i].x = newState.state.bikes[i].X
// 		// 					state.bikes[i].y = newState.state.bikes[i].Y
// 		// 					state.bikes[i].rotation = newState.state.bikes[i].ROT
// 		// 					state.names[i].x = state.bikes[i].x
// 		// 					state.names[i].y = state.bikes[i].y - 20
// 		// 				}
// 		// 			}
// 		// 			for (var i = 0; i < newState.TRAILS.length; i++) {
// 		// 				if (state.trails[i] == null || state.trails[i] == undefined) {
// 		// 					state.trails[i] = state.game.add.graphics(0, 0)
// 		// 				}
// 		// 				state.trails[i].clear()
// 		// 				if (newState.TRAILS[i].STATE == "inactive") {
// 		// 					state.trails[i].lineStyle(1, state.colorToHex[newState.TRAILS[i].COLOUR], 0.2)
// 		// 				} else {
// 		// 					state.trails[i].lineStyle(2, state.colorToHex[newState.TRAILS[i].COLOUR])
// 		// 				}
// 		// 				state.trails[i].moveTo(newState.TRAILS[i].STARTX, newState.TRAILS[i].STARTY)
// 		// 				for (var v = 0; v < newState.TRAILS[i].VERTS.length; v++) {
// 		// 					state.trails[i].lineTo(
// 		// 						newState.TRAILS[i].VERTS[v].X,
// 		// 						newState.TRAILS[i].VERTS[v].Y)
// 		// 				}
// 		// 				state.trails[i].lineTo(
// 		// 					newState.TRAILS[i].ENDX,
// 		// 					newState.TRAILS[i].ENDY)
// 		// 			}
// 		// 			break;

// 		// 		case "DISPLAY_MESSAGE":
// 		// 			document.getElementById("socketmessages").textContent = data.replace("DISPLAY_MESSAGE:", "") + "\n"
// 		// 			break;

// 		// 		default:
// 		// 			console.log("unknown command:" + components[0])
// 		// 			break;
// 		// 	}
// 		// }

// 		shutdown() {

// 		}
// 	}
// }

// // old code, left for reference when implementing client-side prediction
// // // Entities
// // function playerBike(x, y, color, speed, direction) {
// // 	switch (color) {
// // 		case "blue":
// // 		case "green":
// // 		case "orange":
// // 		case "purple":
// // 		case "red":
// // 		case "white":
// // 			this.color = color
// // 			break;

// // 		case undefined:
// // 		case null:
// // 			this.color = "orange"
// // 			break;

// // 		default:
// // 			console.log("Error: " + color + " is not a player color!")
// // 			break;
// // 	}
// // 	this.speed = (speed !== undefined && speed !== null) ? speed : 120;
// // 	switch (direction) {
// // 		case "UP":
// // 		case "LEFT":
// // 		case "DOWN":
// // 		case "RIGHT":
// // 			this.direction = direction
// // 			break;

// // 		case undefined:
// // 		case null:
// // 			this.direction = "UP"
// // 			break;

// // 		default:
// // 			console.log("Error: " + direction + " is not a player direction!")
// // 			break;
// // 	}
// // 	//
// // 	// Phaser.Sprite.call(this, x, y, "gridbike-" + this.color)
// // 	// this.anchor = new Phaser.Point(0.5, 0.5)
// // 	// this.rotation = dirToRot[this.direction]

// // 	this.sprite = webtron.add.sprite(x, y, "gridbike-" + this.color)
// // 	this.sprite.anchor = new Phaser.Point(0.5, 0.5);
// // 	this.sprite.rotation = dirToRot[this.direction]

// // 	this.trail = new gridTrail(this)

// // 	this.justTurned = true
// // 	this.alive = true
// // }
// // playerBike.prototype.update = function(dt) {
// // 	if (!this.alive) {
// // 		return
// // 	}
// // 	if (this.direction === "UP") {
// // 		this.sprite.y -= this.speed * dt;
// // 	} else if (this.direction === "RIGHT") {
// // 		this.sprite.x += this.speed * dt;
// // 	} else if (this.direction === "DOWN") {
// // 		this.sprite.y += this.speed * dt;
// // 	} else if (this.direction === "LEFT") {
// // 		this.sprite.x -= this.speed * dt;
// // 	}
// // 	if (this.sprite.x < 0 || this.sprite.x > webtron.width ||
// // 		this.sprite.y < 0 || this.sprite.y > webtron.height) {
// // 		this.die();
// // 	}
// // 	// needs to check all trails, not just this bike's
// // 	if (!this.justTurned && this.trail.detectPtCollision(this.frontCollidePt())) {
// // 		this.die();
// // 	}
// // 	this.trail.update(this)
// // 	this.justTurned = false
// // }
// // playerBike.prototype.turnUp = function() {
// // 	if (this.alive && this.direction != "DOWN" && this.direction != "UP") {
// // 		socket.send("TURNBIKE:" + slot + "-UP")
// // 		this.trail.setTurn(this)
// // 		this.direction = "UP";
// // 		this.sprite.rotation = dirToRot[this.direction]
// // 		this.justTurned = true
// // 	}
// // }
// // playerBike.prototype.turnLeft = function() {
// // 	if (this.alive && this.direction != "RIGHT" && this.direction != "LEFT") {
// // 		socket.send("TURNBIKE:" + slot + "-LEFT")
// // 		this.trail.setTurn(this)
// // 		this.direction = "LEFT";
// // 		this.sprite.rotation = dirToRot[this.direction]
// // 		this.justTurned = true
// // 	}
// // }
// // playerBike.prototype.turnDown = function() {
// // 	if (this.alive && this.direction != "UP" && this.direction != "DOWN") {
// // 		socket.send("TURNBIKE:" + slot + "-DOWN")
// // 		this.trail.setTurn(this)
// // 		this.direction = "DOWN";
// // 		this.sprite.rotation = dirToRot[this.direction]
// // 		this.justTurned = true
// // 	}
// // }
// // playerBike.prototype.turnRight = function() {
// // 	if (this.alive && this.direction != "LEFT" && this.direction != "RIGHT") {
// // 		socket.send("TURNBIKE:" + slot + "-RIGHT")
// // 		this.trail.setTurn(this)
// // 		this.direction = "RIGHT";
// // 		this.sprite.rotation = dirToRot[this.direction]
// // 		this.justTurned = true
// // 	}
// // }
// // playerBike.prototype.frontCollidePt = function() {
// // 	switch (this.direction) {
// // 		case "UP":
// // 			return new Phaser.Point(this.sprite.x, this.sprite.y - 4)
// // 			break;
// // 		case "LEFT":
// // 			return new Phaser.Point(this.sprite.x - 4, this.sprite.y)
// // 			break;
// // 		case "RIGHT":
// // 			return new Phaser.Point(this.sprite.x + 4, this.sprite.y)
// // 			break;
// // 		case "DOWN":
// // 			return new Phaser.Point(this.sprite.x, this.sprite.y + 4)
// // 			break;
// // 	}
// // 	console.log("Player direction '" + this.direction + " is not valid")
// // 	return new Phaser.Point(this.sprite.x, this.sprite.y)
// // }
// // playerBike.prototype.die = function() {
// // 	this.alive = false
// // 	this.speed = 0;
// // }

// // function gridTrail(bike) {
// // 	switch (bike.color) {
// // 		case "blue":
// // 		case "green":
// // 		case "orange":
// // 		case "purple":
// // 		case "red":
// // 		case "white":
// // 			this.color = colorToHex[bike.color]
// // 			break;

// // 		case null:
// // 			this.color = colorToHex.orange
// // 			break;

// // 		default:
// // 			console.log("Error: " + bike.color + " is not a trail color!")
// // 			break;
// // 	}
// // 	this.startX = bike.sprite.x
// // 	this.startY = bike.sprite.y
// // 	this.lines = [new Phaser.Line(
// // 		this.startX, this.startY, this.startX, this.startY
// // 	)]
// // 	this.graphics = webtron.add.graphics()
// // 	this.graphics.lineStyle(2, this.color)
// // 	this.graphics.moveTo(this.startX, this.startY)
// // }

// // gridTrail.prototype.update = function(bike) {
// // 	this.lines[this.lines.length - 1].
// // 		setTo(this.startX, this.startY, bike.sprite.x, bike.sprite.y)

// // 	this.graphics.lineStyle(2, this.color)
// // 	this.graphics.lineTo(
// // 		this.lines[this.lines.length - 1].end.x,
// // 		this.lines[this.lines.length - 1].end.y)
// // }

// // gridTrail.prototype.setTurn = function(bike) {
// // 	this.update(bike)
// // 	this.startX = bike.sprite.x
// // 	this.startY = bike.sprite.y
// // 	this.lines.push(new Phaser.Line(
// // 		this.startX, this.startY, this.startX, this.startY
// // 	))
// // }

// // gridTrail.prototype.detectPtCollision = function(point) {
// // 	for (var i = 0; i < this.lines.length; i++) {
// // 		if (this.lines[i].pointOnSegment(point.x, point.y)) {
// // 			return true
// // 		}
// // 	}
// // 	return false
// // }
