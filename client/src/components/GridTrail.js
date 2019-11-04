
// function svg() {
// 	const [down, setDown] = useState(false)
// 	function mouseDown() {
// 		setDown(true)
// 	}
// 	function mouseUp() {
// 		setDown(false)
// 	}

// 	function generatePoints() {
// 		const points = []
// 		for (let i = 0; i<1000; i++) {
// 			points.push((Math.random() < 0.5 ? 'H' : 'V') + parseInt(Math.random()*520))
// 		}
// 		return points.join('')
// 	}

// 	const [points, setPoints] = useState(generatePoints)
// 	const [size, setSize] = useState(40)
// 	function expand() {
// 		setSize(size+10)
// 		setPoints(generatePoints())
// 	}

// 	return e('svg', { viewBox: '0 0 560 560' }, [
// 		e('path', {
// 			d:
// 				'M2.625 2.7v47.25H6.9L24.15 30.3v-5.625h-11.7V2.7H2.625zm10.8-2.625l1.65 1.8V22.05h10.05l1.65 1.8v7.425l-19.05 21.3H1.65L0 50.775v-48.9l1.65-1.8h11.775zM41.85 2.7v21.975h-11.7V30.3L47.4 49.95h4.275V2.7H41.85zM52.65.075l1.65 1.8v48.9l-1.65 1.8h-6.075l-19.05-21.3V23.85l1.65-1.8h10.05V1.875l1.65-1.8H52.65zM99.836 18.825H57.31l-1.5 1.125v31.5l1.5 1.125h45.9l1.575-1.125V39.825l-1.5-1.8h-32.25v-4.2h12.9q3.75-.225 6.45-1.425 2.775-1.275 4.5-2.85 1.8-1.65 3.375-4.8t1.575-5.925zm-41.4 2.625h37.95q-.525 2.325-2.25 4.575-1.725 2.175-4.125 3.6-2.4 1.35-6.375 1.575H68.41v9.45h33.75v9.3H58.436v-28.5zM57.31 14.7l-1.5-1.125V1.2l1.5-1.125h45.975l1.5 1.125v12.375l-1.5 1.125H57.31zm44.85-2.625V2.7H58.436v9.375h43.725zM161.023 18.825q0 3.225-1.65 6.075t-3.525 4.5q-1.8 1.575-4.725 3t-6.675 1.425v2.625q6.225 0 9.15 1.8 3 1.725 5.025 5.1 2.025 3.375 2.025 6.225v3h-52.8l-1.5-1.125V18.825h15.225V36.45h12.75v-2.625h-8.475v-15h35.175zm-32.55 2.625v9.75h8.25v7.875h-17.775V21.45h-9.975v28.5h49.125q0-1.875-1.05-4.125-1.05-2.325-3.75-4.725-2.625-2.4-11.775-2.175V31.2q5.475 0 8.7-1.5 3.3-1.5 5.025-3.375 2.025-2.175 2.775-4.875h-29.55zM106.348 1.2l1.5-1.2h37.2q3.375 0 6.3 1.35 3 1.35 5.175 3.45 2.25 2.1 3.45 4.875 1.275 2.7 1.275 5.025h-54.9V1.2zm39.075 1.5h-36.45v9.375h48.9q-.825-3-2.85-4.95-2.025-2.025-4.35-3.225-2.25-1.2-5.25-1.2z',
// 			fill: down ? '#ff0000' : '#f2d91a',
// 			onMouseDown: mouseDown,
// 			onMouseUp: mouseUp,
// 		}),
// 		e('path', {
// 			d: `M0 0h${size}v${size}z${points}`,
// 			fill: 'white',
// 			onClick: expand,
// 		}),
// 	])
// }

// function Webtron() {
// 	const [clicked, setClicked] = React.useState(0)

// 	function bumpClicked() {
// 		setClicked(clicked + 1)
// 	}

// 	const Webtron = useClassName('webtron')

// 	return e(Webtron, { onClick: bumpClicked }, [
// 		e('div', null, 'Hello React!'), //
// 		e('div', null, `Clicked: ${clicked}`), //
// 		e(svg), //
// 	])
// }

// ReactDOM.render(e(Webtron), document.getElementById('webtron'))
