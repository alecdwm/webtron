package simulation

import (
	"math"
	"sync"
	"time"

	"github.com/inconshreveable/log15"
	"github.com/ungerik/go3d/float64/vec2"
)

//// GridBike //////////////////////////////////////////////////////////////////

// GridBikeState is an enum type for the possible states a gridbike can have
type GridBikeState string

// GridBikeStates
const (
	Move GridBikeState = "move"
	Turn GridBikeState = "turn"
	Dead GridBikeState = "dead"
)

// GridBike is a player vehicle
type GridBike struct {
	sync.Mutex

	// state
	state  GridBikeState
	name   string
	colour string

	pos   vec2.T
	rot   float64
	speed float64

	// input
	rotNew float64

	simulation *Simulation
	trail      *GridTrail
}

//// Concurrency-Safe Getters/Setters //////////////////////////////////////////

// GetState simply returns this gridbike's state for other packages to access
func (gb *GridBike) GetState() GridBikeState {
	return gb.state
}

// SetState changes the gridbike's state, unless it is dead
func (gb *GridBike) SetState(newState GridBikeState) {
	gb.Lock()
	if gb.state == "dead" {
		log15.Warn("attempt was made to change the state of a dead gridbike")
		return
	}
	gb.state = newState
	gb.Unlock()
}

//// Callbacks /////////////////////////////////////////////////////////////////

// Move this gridbike's position
func (gb *GridBike) Move(deltaTime float64) {
	gb.trail.end = gb.pos
	gb.pos.Add(&vec2.T{
		math.Cos(gb.rot) * gb.speed * deltaTime,
		math.Sin(gb.rot) * gb.speed * deltaTime,
	})
}

// SetTurn tells this gridbike to turn on the next update
func (gb *GridBike) SetTurn(newRot float64) {
	if gb.state != Dead {
		gb.rotNew = newRot
		gb.SetState(Turn)
	} else {
		log15.Debug("not turning dead gridbike!")
	}
}

// Turn this gridbike
func (gb *GridBike) Turn() {
	if gb.rot+math.Pi == gb.rotNew || gb.rot-math.Pi == gb.rotNew {
		log15.Debug("Bike can't turn 180°", "at", gb.pos)
		gb.rotNew = gb.rot
		return
	}
	log15.Debug("Bike turned", "at", gb.pos)
	gb.trail.end = gb.pos
	gb.trail.verts = append(gb.trail.verts, gb.pos)
	gb.rot = gb.rotNew
}

// Kill this gridbike
func (gb *GridBike) Kill() {
	gb.SetState(Dead)
	gb.simulation.DecreaseNumActiveBikes()
	go func() {
		time.Sleep(3 * time.Second)
		gb.trail.state = Inactive
	}()
}
