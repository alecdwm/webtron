package server

import (
	"fmt"
	"math"
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

	LatestState string
}

// SpawnGridBike spawns a new gridbike
func (sm *SimManager) SpawnGridBike(position vec2.T, rotation, speed float64) *GridBike {
	log15.Debug("Spawning bike", "at", position, "rot", rotation, "speed", speed)

	newGridTrail := &GridTrail{
		origin: position,
	}

	newGridBike := &GridBike{
		state: "new",
		pos:   position,
		rot:   rotation,
		speed: speed,

		rotNew: rotation,

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
	state string // new, move, turn, dead
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

		case "move":
			sm.GridBikes[i].Move(deltaTime)

		case "turn":
			sm.GridBikes[i].Turn()
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
	newState := "{\"BIKES\":["
	for i := range sm.GridBikes {
		if i != 0 {
			newState += ","
		}
		newState += "{" +
			"\"X\":" + fmt.Sprint(sm.GridBikes[i].pos[0]) + "," +
			"\"Y\":" + fmt.Sprint(sm.GridBikes[i].pos[1]) + "," +
			"\"ROT\":" + fmt.Sprint(sm.GridBikes[i].rot) + "}"
	}
	newState += "],\"TRAILS\":["
	for i := range sm.GridTrails {
		if i != 0 {
			newState += ","
		}
		newState += "{" +
			"\"STARTX\":" + fmt.Sprint(sm.GridTrails[i].origin[0]) + "," +
			"\"STARTY\":" + fmt.Sprint(sm.GridTrails[i].origin[1]) + "," +
			"\"VERTS\":["
		for v := range sm.GridTrails[i].verts {
			if v != 0 {
				newState += ","
			}
			newState += "{\"X\":" + fmt.Sprint(sm.GridTrails[i].verts[v][0]) + "," +
				"\"Y\":" + fmt.Sprint(sm.GridTrails[i].verts[v][1]) + "}"
		}
		newState += "],\"ENDX\":" + fmt.Sprint(sm.GridTrails[i].end[0]) + "," +
			"\"ENDY\":" + fmt.Sprint(sm.GridTrails[i].end[1]) + "}"
	}
	newState += "]}"
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
	gb.state = "move"
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
