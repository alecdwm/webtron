/// <reference path="../webtron.ts" />

namespace Webtron {
	export class Lobby extends Phaser.State {
		player: Phaser.Sprite
		games: number[]
		gameList: Phaser.Text[]

		preload() {
			this.game.load.image("gridply-blue", "img/gridply-blue.png")
			this.game.load.image("gridply-green", "img/gridply-green.png")
			this.game.load.image("gridply-orange", "img/gridply-orange.png")
			this.game.load.image("gridply-purple", "img/gridply-purple.png")
			this.game.load.image("gridply-red", "img/gridply-red.png")
			this.game.load.image("gridply-white", "img/gridply-white.png")
		}

		create() {
			// connect to the server
			socket.onmessage = this.socketmessage
			socket.send("LIST_GAMES")

			this.player = this.game.add.sprite(280, 280, "gridply-" + playerColor)
			this.player.anchor.set(0.5, 0.5)

			// this.game.state.start("ingame")
		}

		update() {
			if (this.input.keyboard.isDown(Phaser.KeyCode.W)) {
				this.player.y = this.player.y - 20 * this.game.time.physicsElapsed
				this.player.rotation = 3 * Math.PI / 2
			}
			if (this.input.keyboard.isDown(Phaser.KeyCode.A)) {
				this.player.x = this.player.x - 20 * this.game.time.physicsElapsed
				this.player.rotation = Math.PI
			}
			if (this.input.keyboard.isDown(Phaser.KeyCode.S)) {
				this.player.y = this.player.y + 20 * this.game.time.physicsElapsed
				this.player.rotation = Math.PI / 2
			}
			if (this.input.keyboard.isDown(Phaser.KeyCode.D)) {
				this.player.x = this.player.x + 20 * this.game.time.physicsElapsed
				this.player.rotation = 0
			}
		}

		socketmessage(event: any) {
			console.log(event)
		}
	}
}
