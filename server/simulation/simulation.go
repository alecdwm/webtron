package simulation

import (
	"fmt"
	"math"
	"math/rand"
	"sync"
	"time"

	"github.com/inconshreveable/log15"
	"github.com/ungerik/go3d/float64/vec2"
)

//// Simulation ////////////////////////////////////////////////////////////////

// Simulation runs the game simulation
type Simulation struct {
	sync.Mutex

	gridSize       vec2.T
	numActiveBikes int
	gridBikes      []*GridBike
	gridTrails     []*GridTrail

	rand        *rand.Rand
	LatestState string
}

// New creates and returns a new simulation instance
func New(sizex, sizey float64) *Simulation {
	s := &Simulation{
		gridSize: vec2.T{0: sizex, 1: sizey},

		rand: rand.New(rand.NewSource(time.Now().UnixNano())),
	}

	return s
}

//// Concurrency-Safe Getters/Setters //////////////////////////////////////////

// IncreaseNumActiveBikes counter by one
func (s *Simulation) IncreaseNumActiveBikes() {
	s.Lock()
	s.numActiveBikes++
	s.Unlock()
}

// DecreaseNumActiveBikes counter by one
func (s *Simulation) DecreaseNumActiveBikes() {
	s.Lock()
	s.numActiveBikes--
	s.Unlock()
}

//// Callbacks /////////////////////////////////////////////////////////////////

// SpawnGridBike spawns a new gridbike
func (s *Simulation) SpawnGridBike(name string, colour string) *GridBike {
	pos, rot := s.calcNewSpawnpoint()
	speed := 120.0

	log15.Debug("Spawning bike", "at", pos, "rot", rot, "speed", speed, "name", name, "colour", colour)

	newGridTrail := &GridTrail{
		state:  Active,
		colour: colour,
		origin: pos,
	}

	newGridBike := &GridBike{
		state:  Move,
		name:   name,
		colour: colour,

		pos:   pos,
		rot:   rot,
		speed: speed,

		rotNew: rot,

		trail:      newGridTrail,
		simulation: s,
	}

	s.gridTrails = append(s.gridTrails, newGridTrail)
	s.gridBikes = append(s.gridBikes, newGridBike)

	if s.numActiveBikes == 0 {
		s.IncreaseNumActiveBikes()
		go func() {
			now := time.Now()
			then := now
			for s.numActiveBikes > 0 {
				now = time.Now()
				s.Simulate(now.Sub(then).Seconds())
				then = now
			}
			time.Sleep(4 * time.Second)
			s.updateStateString()
		}()
	} else {
		s.IncreaseNumActiveBikes()
	}

	return newGridBike
}

// Simulate runs a frame of simulation
func (s *Simulation) Simulate(deltaTime float64) {
	// Simulate one frame of gridbike movement
	for i := range s.gridBikes {
		switch s.gridBikes[i].state {
		case Move:
			s.gridBikes[i].Move(deltaTime)

		case Turn:
			s.gridBikes[i].Turn()
			s.gridBikes[i].SetState(Move)
			s.gridBikes[i].Move(deltaTime)
		}
	}

	// check for collisions
	for i := range s.gridBikes {
		if s.gridBikes[i].state == Dead {
			continue
		}
		checkPos := vec2.Add(&s.gridBikes[i].pos, &vec2.T{
			math.Cos(s.gridBikes[i].rot) * s.gridBikes[i].speed * deltaTime,
			math.Sin(s.gridBikes[i].rot) * s.gridBikes[i].speed * deltaTime,
		})
		if collided, with := s.checkCollisions(checkPos); collided {
			log15.Debug("Bike collision", "with", with, "at", s.gridBikes[i].pos, "collision point", checkPos)
			s.gridBikes[i].Kill()
		}
	}

	// update json string to send to clients
	s.updateStateString()
}

//// Internal Functions ////////////////////////////////////////////////////////

// checkCollisions returns true if position collides with anything
func (s *Simulation) checkCollisions(point vec2.T) (bool, string) {
	if point[0] < 0 || point[0] > s.gridSize[0] ||
		point[1] < 0 || point[1] > s.gridSize[1] {
		return true, "arena wall"
	}
	for i := range s.gridTrails {
		if s.gridTrails[i].state == Inactive {
			continue
		}
		if s.gridTrails[i].OnTrail(point) {
			return true, "grid trail"
		}
	}
	return false, ""
}

// updateStateString updates the json-encoded representation of the game state
func (s *Simulation) updateStateString() {
	newState := `{"BIKES":[`
	for i := range s.gridBikes {
		if i != 0 {
			newState += `,`
		}
		newState += fmt.Sprintf(`{"STATE":"%s","NAME":"%s","COLOUR":"%s","X":%f,"Y":%f,"ROT":%f}`,
			s.gridBikes[i].state,
			s.gridBikes[i].name,
			s.gridBikes[i].colour,
			s.gridBikes[i].pos[0],
			s.gridBikes[i].pos[1],
			s.gridBikes[i].rot,
		)
	}
	newState += `],"TRAILS":[`
	for i := range s.gridTrails {
		if i != 0 {
			newState += `,`
		}
		newState += fmt.Sprintf(`{"STATE":"%s","COLOUR":"%s","STARTX":%f,"STARTY":%f,"VERTS":[`,
			s.gridTrails[i].state,
			s.gridTrails[i].colour,
			s.gridTrails[i].origin[0],
			s.gridTrails[i].origin[1],
		)
		for v := range s.gridTrails[i].verts {
			if v != 0 {
				newState += ","
			}
			newState += fmt.Sprintf(`{"X":%f,"Y":%f}`,
				s.gridTrails[i].verts[v][0],
				s.gridTrails[i].verts[v][1],
			)
		}
		newState += fmt.Sprintf(`],"ENDX":%f,"ENDY":%f}`,
			s.gridTrails[i].end[0],
			s.gridTrails[i].end[1],
		)
	}
	newState += `]}`
	s.LatestState = newState
}

func (s *Simulation) calcNewSpawnpoint() (vec2.T, float64) {
	pos := vec2.T{
		0: s.rand.Float64() * s.gridSize[0],
		1: s.rand.Float64() * s.gridSize[1],
	}
	rot := 0.0

	if pos[0] < s.gridSize[0]/2 { // left half
		if pos[1] < s.gridSize[1]/2 { // top left quad
			if pos[0] < pos[1] { // closer to left than top
				rot = 0.0 // right
			} else { // closer to top than left
				rot = math.Pi / 2.0 // down
			}
		} else { // bottom left quad
			if pos[0] < s.gridSize[1]-pos[1] { // closer to left than bottom
				rot = 0.0 // right
			} else { // closer to bottom than left
				rot = 3 * math.Pi / 2.0 // up
			}
		}
	} else { // right half
		if pos[1] < s.gridSize[1]/2 { // top right quad
			if s.gridSize[0]-pos[0] < pos[1] { // closer to right than top
				rot = math.Pi // left
			} else { // closer to top than right
				rot = math.Pi / 2.0 // down
			}
		} else { // bottom right quad
			if s.gridSize[0]-pos[0] < s.gridSize[1]-pos[1] { // closer to right than bottom
				rot = math.Pi // left
			} else { // closer to bottom than right
				rot = 3 * math.Pi / 2.0 // up
			}
		}
	}

	if collided, _ := s.checkCollisions(pos); collided {
		return s.calcNewSpawnpoint()
	}

	return pos, rot
}
