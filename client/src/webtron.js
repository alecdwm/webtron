/* jshint asi:true */

// Helper Functions
// Prototype for finding the difference of two 'numbers'
Number.prototype.between = function(a, b, inclusive) {
	var min = Math.min.apply(Math, [a, b]),
		max = Math.max.apply(Math, [a, b]);
	return inclusive ? this >= min && this <= max : this > min && this < max;
};

// Variables
var webtron, // global phaser instance
	background,
	thisPlayer,
	colorHexes = {
		blue: "#00c2cc",
		green: "#2ee5c7",
		orange: "#f2d91a",
		purple: "#8a2ee5",
		red: "#e5482e",
		white: "#e5feff"
	};

// Initialize
$(window)
	.load(function() {
		webtron = new Phaser.Game(560, 560, Phaser.AUTO, "webtron", {
			preload: preload,
			create: create,
			update: update,
		}, true);
	});

// Callbacks
function preload() {
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
	background = webtron.add.image(0, 0, "background")

	// Player
	thisPlayer = new playerBike(250, 500)

	// Player Input
	var upKey = webtron.input.keyboard.addKey(Phaser.Keyboard.W)
	var leftKey = webtron.input.keyboard.addKey(Phaser.Keyboard.A)
	var downKey = webtron.input.keyboard.addKey(Phaser.Keyboard.S)
	var rightKey = webtron.input.keyboard.addKey(Phaser.Keyboard.D)
	upKey.onDown.add(thisPlayer.turnUp, thisPlayer)
	leftKey.onDown.add(thisPlayer.turnLeft, thisPlayer)
	downKey.onDown.add(thisPlayer.turnDown, thisPlayer)
	rightKey.onDown.add(thisPlayer.turnRight, thisPlayer)
}

function update() {
	thisPlayer.move(webtron.time.physicsElapsed);
}

// Entities
function playerBike(initX, initY, color, speed, direction) {
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
	this.directionEnum = {
		UP: 0,
		DOWN: Math.PI,
		RIGHT: Math.PI / 2,
		LEFT: 3 * Math.PI / 2,
	}

	this.sprite = webtron.add.sprite(initX, initY, "lightbike-" + this.color)
	this.sprite.anchor = new Phaser.Point(0.5, 0.5);
	this.sprite.rotation = this.directionEnum[this.direction]

	this.trail = new lightTrail(this)

	this.move = function(dt) {
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
		for (var i = 0; i < this.trail.cornerPts.length; i = i + 2) {
			if (this.sprite.x.between(this.trail.cornerPts[i], this.trail.cornerPts[i + 2], true) && this.sprite.y.between(this.trail.cornerPts[i + 1], this.trail.cornerPts[i + 3], true)) {
				this.die();
			}
		}
		this.trail.cornerPts.pop();
		this.trail.cornerPts.pop();
		this.trail.cornerPts.push(this.sprite.x);
		this.trail.cornerPts.push(this.sprite.y);
		this.trail.cornerPtsPoly.setTo(this.trail.cornerPts)
	}
	this.turnUp = function() {
		if (this.direction != "DOWN" && this.direction != "UP") {
			this.trail.cornerPts.push(this.sprite.x);
			this.trail.cornerPts.push(this.sprite.y);
			this.direction = "UP";
			this.sprite.rotation = this.directionEnum[this.direction]
		}
	}
	this.turnLeft = function() {
		if (this.direction != "RIGHT" && this.direction != "LEFT") {
			this.trail.cornerPts.push(this.sprite.x);
			this.trail.cornerPts.push(this.sprite.y);
			this.direction = "LEFT";
			this.sprite.rotation = this.directionEnum[this.direction]
		}
	}
	this.turnDown = function() {
		if (this.direction != "UP" && this.direction != "DOWN") {
			this.trail.cornerPts.push(this.sprite.x);
			this.trail.cornerPts.push(this.sprite.y);
			this.direction = "DOWN";
			this.sprite.rotation = this.directionEnum[this.direction]
		}
	}
	this.turnRight = function() {
		if (this.direction != "LEFT" && this.direction != "RIGHT") {
			this.trail.cornerPts.push(this.sprite.x);
			this.trail.cornerPts.push(this.sprite.y);
			this.direction = "RIGHT";
			this.sprite.rotation = this.directionEnum[this.direction]
		}
	}
	this.die = function() {
		this.trail.cornerPts = [];
		this.speed = 0;
		this.sprite.x = -20;
		this.sprite.y = 20;
	}
}

function lightTrail(bike) {
	switch (bike.color) {
		case "blue":
		case "green":
		case "orange":
		case "purple":
		case "red":
		case "white":
			this.color = colorHexes[bike.color]
			break;

		case null:
			this.color = colorHexes.orange
			break;

		default:
			console.log("Error: " + bike.color + " is not a trail color!")
			break;
	}
	this.cornerPts = []
	this.cornerPtsPoly = webtron.add.graphics(bike.sprite.x, bike.sprite.y)
	this.cornerPtsPoly.closed = false

	// Start trail at bike pos
	this.cornerPts.push(bike.sprite.x);
	this.cornerPts.push(bike.sprite.y);
	this.cornerPts.push(bike.sprite.x);
	this.cornerPts.push(bike.sprite.y);

	this.draw = function() {
		canvas.fillStyle = this.color;
		canvas.beginPath();
		canvas.lineWidth = 4;
		canvas.strokeStyle = this.color;
		canvas.moveTo(this.cornerPts[0], this.cornerPts[1]);
		for (var i = 2; i < this.cornerPts.length; i = i + 2) {
			canvas.lineTo(this.cornerPts[i], this.cornerPts[i + 1]);
		}
		canvas.lineJoin = "miter";
		canvas.stroke();
	}
}
