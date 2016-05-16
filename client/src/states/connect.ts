/// <reference path="../webtron.ts" />

namespace Webtron {
	export class Connect extends Phaser.State {
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
				"fill": colorsToHexString[playerColor],
			})

			var protocol = (window.location.protocol == "https:") ? "wss:" : "ws:",
				hostname = window.location.hostname,
				port = window.location.port,
				path = "/ws",
				address = protocol + "//" + hostname + ":" + port + path

			var state = this
			socket = new WebSocket(address)
			// TODO: Show connection error / disconnect messages on client
			socket.onerror = function(event) {
				serverMsg = "CONNECTION ERROR"
				state.game.state.start("mainmenu")
			}
			socket.onclose = function(event) {
				serverMsg = "DISCONNECTED"
				state.game.state.start("mainmenu")
			}
			socket.onopen = function(event) {
				state.game.state.start("lobby")
			}
		}
	}
}
