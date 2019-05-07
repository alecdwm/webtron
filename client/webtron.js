window.addEventListener('load', () => new Webtron(document.getElementById('webtron')))

class Webtron {
	constructor(domElement = document.body) {
		// connect to server
		this.createWebsocketConnection()

		// Skip PIXI.js hello message,
		// but still print an unobtrusive version when not in development.
		PIXI.utils.skipHello()
		if (window.location.hostname !== 'localhost') console.log(`PixiJS ${PIXI.VERSION} - http://www.pixijs.com/`)

		// setup asset loader
		this.loader = new PIXI.loaders.Loader()

		// setup input events
		document.addEventListener('keydown', this.onKeyDown.bind(this))
		document.addEventListener('keyup', this.onKeyUp.bind(this))
		document.addEventListener('keypress', this.onKeyPress.bind(this))

		// setup renderer
		this.renderer = new PIXI.autoDetectRenderer({ width: 560, height: 560, transparent: true })
		domElement.appendChild(this.renderer.view)
		this.renderer.view.addEventListener('contextmenu', e => e.preventDefault())

		// setup global state
		this.globalstate = {
			playerName: '',
			playerColor: undefined,
			socket: undefined,
			statusText: '',
		}

		// setup scene
		this.scene = new PIXI.Container()

		// start main menu
		this.changeState('MainMenu')

		// start game loop
		this.updateThen = Date.now()
		this.update()
	}

	//
	// update loop
	//

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

	//
	// event callbacks
	//

	onKeyDown(e) {
		if (!this.state || !this.state.onKeyDown) return
		this.state.onKeyDown.call(this.state, e.key)
	}

	onKeyUp(e) {
		if (!this.state || !this.state.onKeyUp) return
		this.state.onKeyUp.call(this.state, e.key)
	}

	onKeyPress(e) {
		if (!this.state || !this.state.onKeyPress) return
		this.state.onKeyPress.call(this.state, e.key)
	}

	//
	// helper methods
	//

	setGlobalState(newState) {
		this.globalstate = Object.assign(this.globalstate, newState)
	}

	createWebsocketConnection() {
		const protocol = window.location.protocol === 'https' ? 'wss' : 'ws'
		const socket_url = `${protocol}://${window.location.host}/ws`
		const socket = new WebSocket(socket_url)
		socket.addEventListener('open', () => {
			console.log('socket open')
			socket.send(JSON.stringify('ListGames'))
		})
		socket.addEventListener('error', error => {
			console.error('socket error', error)
		})
		socket.addEventListener('message', event => {
			if (typeof event.data !== 'string') {
				console.warn('Received non-text websocket message', event)
				return
			}

			const message = JSON.parse(event.data)
			console.log(`socket message`, message)
		})
		socket.addEventListener('close', event => {
			console.log(`socket closed: ${event.code} ${event.reason}`)
		})
	}

	changeState(nextState) {
		// get reference to next state
		if (typeof nextState !== 'string') throw new Error(`nextState must be a string, given ${typeof nextState}`)
		if (!window.states[nextState]) throw new Error(`No state by name '${nextState}' exists!`)
		nextState = window.states[nextState]

		// handle current state exit (if exists)
		if (this.state) {
			this.state.onExit && this.state.onExit()
			this.loader.reset()
		}

		// reset scene
		// this.scene = new PIXI.Container()

		// switch states
		this.state = new nextState()
		this.state.game = this

		// handle new state entry (if no asset preloading required)
		if (!this.state.preload || !Array.isArray(this.state.preload)) {
			this.state.onEnter && this.state.onEnter.call(this.state)
			return
		}

		// handle asset preloading
		this.loader.add(this.state.preload).load((loader, resources) => {
			// print asset preloading errors
			Object.values(resources).forEach(resource => {
				if (resource.error) console.error(`${resource.error} (assetUrl: ${resource.url})`)
			})

			// handle new state entry
			this.state.onEnter && this.state.onEnter.call(this.state)
		})
	}
}

Webtron.colors = ['blue', 'green', 'orange', 'purple', 'red', 'white']
Webtron.randomColor = () => Webtron.colors[parseInt(Math.random() * Webtron.colors.length)]
Webtron.maxPlayerNameLength = 12
Webtron.uiFontFamily = '"Courier New", Courier, monospace'
Webtron.uiFontSize = 32
Webtron.colorToHex = {
	blue: 0x00c2cc,
	green: 0x2ee53d,
	orange: 0xf2d91a,
	purple: 0x8a2ee5,
	red: 0xe5482e,
	white: 0xe5feff,
}
Webtron.colorToHexString = {
	blue: '#00c2cc',
	green: '#2ee53d',
	orange: '#f2d91a',
	purple: '#8a2ee5',
	red: '#e5482e',
	white: '#e5feff',
}
