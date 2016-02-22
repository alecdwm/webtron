package server

import (
	"fmt"
	"math"
	"math/rand"
	"time"

	"github.com/inconshreveable/log15"
	"github.com/ungerik/go3d/float64/vec2"
)

// SimManager runs the game simulation
type SimManager struct {
	GridSize    vec2.T
	ActiveBikes int
	GridBikes   []*GridBike
	GridTrails  []*GridTrail

	Rand        *rand.Rand
	LatestState string
}

// SpawnGridBike spawns a new gridbike
func (sm *SimManager) SpawnGridBike(name string, colour string) *GridBike {
	pos, rot := sm.calcNewSpawnpoint()
	speed := 120.0

	log15.Debug("Spawning bike", "at", pos, "rot", rot, "speed", speed, "name", name, "colour", colour)

	newGridTrail := &GridTrail{
		state:  "active",
		colour: colour,
		origin: pos,
	}

	newGridBike := &GridBike{
		state:  "new",
		name:   name,
		colour: colour,

		pos:   pos,
		rot:   rot,
		speed: speed,

		rotNew: rot,

		trail:     newGridTrail,
		simulator: sm,
	}

	sm.GridTrails = append(sm.GridTrails, newGridTrail)
	sm.GridBikes = append(sm.GridBikes, newGridBike)

	if sm.ActiveBikes == 0 {
		sm.ActiveBikes++
		go func() {
			now := time.Now()
			then := now
			for sm.ActiveBikes > 0 {
				now = time.Now()
				sm.Simulate(now.Sub(then).Seconds())
				then = now
			}
		}()
	} else {
		sm.ActiveBikes++
	}

	return newGridBike
}

// GridBike is a player vehicle
type GridBike struct {
	// state
	state  string // new, move, turn, dead
	name   string
	colour string

	pos   vec2.T
	rot   float64
	speed float64

	// input
	rotNew float64

	simulator *SimManager
	trail     *GridTrail
}

// Simulate runs a frame of simulation
func (sm *SimManager) Simulate(deltaTime float64) {
	// Simulate one frame of gridbike movement
	for i := range sm.GridBikes {
		switch sm.GridBikes[i].state {
		case "new":
			sm.GridBikes[i].state = "move"
			sm.GridBikes[i].Move(deltaTime)

		case "move":
			sm.GridBikes[i].Move(deltaTime)

		case "turn":
			sm.GridBikes[i].Turn()
			sm.GridBikes[i].state = "move"
			sm.GridBikes[i].Move(deltaTime)
		}
	}

	// check for collisions
	for i := range sm.GridBikes {
		if sm.GridBikes[i].state == "dead" {
			continue
		}
		checkPos := vec2.Add(&sm.GridBikes[i].pos, &vec2.T{
			math.Cos(sm.GridBikes[i].rot) * sm.GridBikes[i].speed * deltaTime,
			math.Sin(sm.GridBikes[i].rot) * sm.GridBikes[i].speed * deltaTime,
		})
		if checkPos[0] < 0 || checkPos[0] > sm.GridSize[0] ||
			checkPos[1] < 0 || checkPos[1] > sm.GridSize[1] {
			log15.Debug("Bike collision with arena wall", "at", sm.GridBikes[i].pos)
			sm.GridBikes[i].Kill()
			continue
		}
		for i := range sm.GridTrails {
			if sm.GridTrails[i].state == "inactive" {
				continue
			}
			if sm.GridTrails[i].OnTrail(checkPos) {
				log15.Debug("Bike collision with grid trail", "at", sm.GridBikes[i].pos)
				sm.GridBikes[i].Kill()
			}
		}
	}

	// update json string to send to clients
	newState := `{"BIKES":[`
	for i := range sm.GridBikes {
		if i != 0 {
			newState += `,`
		}
		newState += fmt.Sprintf(`{"NAME":"%s","COLOUR":"%s","X":%f,"Y":%f,"ROT":%f}`,
			sm.GridBikes[i].name,
			sm.GridBikes[i].colour,
			sm.GridBikes[i].pos[0],
			sm.GridBikes[i].pos[1],
			sm.GridBikes[i].rot,
		)
	}
	newState += `],"TRAILS":[`
	for i := range sm.GridTrails {
		if i != 0 {
			newState += `,`
		}
		newState += fmt.Sprintf(`{"COLOUR":"%s","STARTX":%f,"STARTY":%f,"VERTS":[`,
			sm.GridTrails[i].colour,
			sm.GridTrails[i].origin[0],
			sm.GridTrails[i].origin[1],
		)
		for v := range sm.GridTrails[i].verts {
			if v != 0 {
				newState += ","
			}
			newState += fmt.Sprintf(`{"X":%f,"Y":%f}`,
				sm.GridTrails[i].verts[v][0],
				sm.GridTrails[i].verts[v][1],
			)
		}
		newState += fmt.Sprintf(`],"ENDX":%f,"ENDY":%f}`,
			sm.GridTrails[i].end[0],
			sm.GridTrails[i].end[1],
		)
	}
	newState += `]}`
	sm.LatestState = newState
}

// Move this grid bike's position
func (gb *GridBike) Move(deltaTime float64) {
	gb.trail.end = gb.pos
	gb.pos.Add(&vec2.T{
		math.Cos(gb.rot) * gb.speed * deltaTime,
		math.Sin(gb.rot) * gb.speed * deltaTime,
	})
}

// SetTurn sets the next move to be a turn
func (gb *GridBike) SetTurn(newRot float64) {
	gb.rotNew = newRot
	gb.state = "turn"
}

// Turn this grid bike
func (gb *GridBike) Turn() {
	log15.Debug("Bike turned", "at", gb.pos)
	gb.trail.end = gb.pos
	gb.trail.verts = append(gb.trail.verts, gb.pos)
	gb.rot = gb.rotNew
}

// Kill this grid bike
func (gb *GridBike) Kill() {
	gb.state = "dead"
	gb.simulator.ActiveBikes--
	go func() {
		time.Sleep(3)
		gb.trail.state = "inactive"
	}()
}

// GridTrail is an impassible wall of grid... stuff
type GridTrail struct {
	state  string // active, inactive
	colour string

	origin vec2.T
	verts  []vec2.T
	end    vec2.T
}

// OnTrail checks if a point is colliding with this trail
func (gt *GridTrail) OnTrail(pos vec2.T) bool {
	verts := []vec2.T{gt.origin}
	verts = append(verts, gt.verts...)
	verts = append(verts, gt.end)
	for i := range verts {
		if i == 0 {
			continue
		}

		if pointOnLine(verts[i-1], verts[i], pos) {
			return true
		}
	}

	return false
}

// Check if point is on line:
// 1. is slope of a to c the same as a to b?
// 2. is c.x between a.x and b.x, and c.y between a.y and b.y?
func pointOnLine(a, b, c vec2.T) bool {
	// if (b[0]-a[0])*(c[1]-a[1]) == (c[0]-a[0])*(b[1]-a[1]) &&
	// 	math.Abs(cmp(a[0], c[0])+cmp(b[0], c[0])) <= 1 &&
	// 	math.Abs(cmp(a[1], c[1])+cmp(b[1], c[1])) <= 1 {
	// 	return true
	// }
	return false
}

// stolen from python
// returns -1 if a < b, 0 if a == b, 1 if a > b
func cmp(a, b float64) float64 {
	if a < b {
		return -1
	}
	if a == b {
		return 0
	}
	return 1
}

func (sm *SimManager) calcNewSpawnpoint() (vec2.T, float64) {
	pos := vec2.T{
		0: sm.Rand.Float64() * sm.GridSize[0],
		1: sm.Rand.Float64() * sm.GridSize[1],
	}
	rot := 0.0

	if pos[0] < sm.GridSize[0]/2 { // left half
		if pos[1] < sm.GridSize[1]/2 { // top left quad
			if pos[0] < pos[1] { // closer to left than top
				rot = 0.0 // right
			} else { // closer to top than left
				rot = math.Pi / 2.0 // down
			}
		} else { // bottom left quad
			if pos[0] < sm.GridSize[1]-pos[1] { // closer to left than bottom
				rot = 0.0 // right
			} else { // closer to bottom than left
				rot = 3 * math.Pi / 2.0 // up
			}
		}
	} else { // right half
		if pos[1] < sm.GridSize[1]/2 { // top right quad
			if sm.GridSize[0]-pos[0] < pos[1] { // closer to right than top
				rot = math.Pi // left
			} else { // closer to top than right
				rot = math.Pi / 2.0 // down
			}
		} else { // bottom right quad
			if sm.GridSize[0]-pos[0] < sm.GridSize[1]-pos[1] { // closer to right than bottom
				rot = math.Pi // left
			} else { // closer to bottom than right
				rot = 3 * math.Pi / 2.0 // up
			}
		}
	}

	return pos, rot
}
