package server

import (
	"math"
	"time"

	"github.com/ungerik/go3d/float64/vec2"
)

// SimManager runs the game simulation
type SimManager struct {
	GridSize   vec2.T
	GridBikes  []*GridBike
	GridTrails []*GridTrail
}

// SpawnGridBike spawns a new gridbike
func (sm *SimManager) SpawnGridBike(position vec2.T, rotation, speed float64) *GridBike {
	newGridTrail := &GridTrail{
		origin: position,
	}

	newGridBike := &GridBike{
		state: "new",
		pos:   position,
		rot:   rotation,
		speed: speed,

		rotNew: rotation,

		trail: newGridTrail,
	}

	sm.GridTrails = append(sm.GridTrails, newGridTrail)
	sm.GridBikes = append(sm.GridBikes, newGridBike)

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

	trail *GridTrail
}

// Simulate runs a frame of simulation
func (sm *SimManager) Simulate(deltaTime float64) {
	// Simulate one frame of gridbike movement
	for i := range sm.GridBikes {
		switch sm.GridBikes[i].state {
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
		for i := range sm.GridTrails {
			if sm.GridTrails[i].state == "inactive" {
				continue
			}
			// checkPos := vec2.Add(&sm.GridBikes[i].pos, &vec2.T{
			// 	sm.GridBikes[i].pos[0] + math.Cos(sm.GridBikes[i].rot)*sm.GridBikes[i].speed*deltaTime,
			// 	sm.GridBikes[i].pos[1] + math.Sin(sm.GridBikes[i].rot)*sm.GridBikes[i].speed*deltaTime,
			// })
			// if sm.GridTrails[i].OnTrail(checkPos) {
			// 	sm.GridBikes[i].state = "dead"
			// }
		}
	}
}

// Move this grid bike's position
func (gb *GridBike) Move(deltaTime float64) {
	gb.trail.end = gb.pos
	gb.pos.Add(&vec2.T{
		gb.pos[0] + math.Cos(gb.rot)*gb.speed*deltaTime,
		gb.pos[1] + math.Sin(gb.rot)*gb.speed*deltaTime,
	})
}

// SetTurn sets the next move to be a turn
func (gb *GridBike) SetTurn(newRot float64) {
	gb.rotNew = newRot
	gb.state = "turn"
}

// Turn this grid bike
func (gb *GridBike) Turn() {
	gb.trail.end = gb.pos
	gb.trail.verts = append(gb.trail.verts, gb.pos)
	gb.rot = gb.rotNew
	gb.state = "move"
}

// Kill this grid bike
func (gb *GridBike) Kill() {
	gb.state = "dead"
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
	if (b[0]-a[0])*(c[1]-a[1]) == (c[0]-a[0])*(b[1]-a[1]) &&
		math.Abs(cmp(a[0], c[0])+cmp(b[0], c[0])) <= 1 &&
		math.Abs(cmp(a[1], c[1])+cmp(b[1], c[1])) <= 1 {
		return true
	}
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
