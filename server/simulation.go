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
	state string // new, move, dead
	pos   vec2.T
	rot   float64
	speed float64

	// input
	rotNew float64

	trail *GridTrail
}

// Simulate runs a frame of simulation
func (sm *SimManager) Simulate(deltaTime float64) {
	for i := range sm.GridBikes {
		switch sm.GridBikes[i].state {
		case "move":
			sm.GridBikes[i].Move(deltaTime)

		case "turn":
			sm.GridBikes[i].Turn()
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

// Turn this grid bike
func (gb *GridBike) Turn() {
	gb.trail.end = gb.pos
	gb.trail.verts = append(gb.trail.verts, gb.pos)
	gb.rot = gb.rotNew
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
