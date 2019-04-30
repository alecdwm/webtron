window.addEventListener('load', () => {
	PIXI.utils.skipHello()
	console.log(`Pixi.js ${PIXI.VERSION} - http://www.pixijs.com/`)

	const protocol = window.location.protocol === 'https' ? 'wss' : 'ws'
	const socket_url = `${protocol}://${window.location.host}/ws`
	const socket = new WebSocket(socket_url)
	socket.addEventListener('open', () => {
		console.log('socket open')
		socket.send(JSON.stringify("ListGames"))
		// socket.send(new ArrayBuffer(12))
	})
	socket.addEventListener('error', error => {
		console.log('socket error', error)
	})
	socket.addEventListener('message', event => {
		const message = event.data
		console.log(`socket message: ${message}`)
	})
	socket.addEventListener('close', event => {
		console.log(`socket closed: ${event.code} ${event.reason}`)
	})

	new Webtron(document.getElementById('webtron'))
})

class Webtron {
	constructor(domElement) {
		if (!domElement) {
			domElement = document.body
		}

		// set up asset loader
		this.loader = new PIXI.loaders.Loader()

		// set up input events
		document.addEventListener('keydown', this.onKeyDown.bind(this))
		document.addEventListener('keyup', this.onKeyUp.bind(this))
		document.addEventListener('keypress', this.onKeyPress.bind(this))
		// document.addEventListener('mousedown', this.onMouseDown.bind(this))
		// document.addEventListener('mouseup', this.onMouseUp.bind(this))
		// document.addEventListener('mousemove', this.onMouseMove.bind(this))
		// document.addEventListener('touchstart', this.onTouchStart.bind(this))
		// document.addEventListener('touchend', this.onTouchEnd.bind(this))
		// document.addEventListener('touchmove', this.onTouchMove.bind(this))

		// set up renderer
		this.renderer = new PIXI.autoDetectRenderer({
			width: 560,
			height: 560,
			transparent: true,
		})
		domElement.appendChild(this.renderer.view)

		// set up scene
		this.scene = new PIXI.Container()

		// start main menu
		this.changeState('MainMenu')

		// start game loop
		this.updateThen = Date.now()
		this.update()
	}

	changeState(newState) {
		if (typeof(newState) === 'string') {
			if (window.states[newState]) {
				newState = window.states[newState]
			} else {
				throw new Error(`No state by name '${newState}' exists!`)
			}
		}

		if (this.state) {
			this.state.onExit && this.state.onExit()
			this.loader.reset()
		}

		this.state = new newState()
		this.state.game = this

		if (this.state.preload && Array.isArray(this.state.preload)) {
			this.loader
				.add(this.state.preload)
				.load((loader, resources) => {
					for (let resourceKey in resources) {
						const resource = resources[resourceKey]
						if (resource.error) {
							console.error(`${resource.error} (assetUrl: ${resource.url})`)
						}
					}
					this.state.onEnter && this.state.onEnter.call(this.state)
				})
		} else {
			this.state.onEnter && this.state.onEnter.call(this.state)
		}
	}

	update() {
		this.updateNow = Date.now()
		const dt = (this.updateNow - this.updateThen) / 1000.0
		this.updateThen = this.updateNow

		if (this.state && this.state.onUpdate && this.state.ready) {
			this.state.onUpdate.call(this.state, dt)
		}
		this.renderer.render(this.scene)

		window.requestAnimationFrame(this.update.bind(this))
	}

	onKeyDown(e) {
		if (this.state && this.state.onKeyDown) {
			this.state.onKeyDown.call(this.state, e.key)
		}
	}

	onKeyUp(e) {
		if (this.state && this.state.onKeyUp) {
			this.state.onKeyUp.call(this.state, e.key)
		}
	}

	onKeyPress(e) {
		if (this.state && this.state.onKeyPress) {
			this.state.onKeyPress.call(this.state, e.key)
		}
	}
}
