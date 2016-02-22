package server

import (
	"math"
	"math/rand"
	"strings"
	"time"

	"github.com/desertbit/glue"
	"github.com/inconshreveable/log15"
	"github.com/ungerik/go3d/float64/vec2"
	"go.owls.io/webtron/server/msgdefs"
)

//// Player ////////////////////////////////////////////////////////////////////

// PlayerState i.e. Connecting, InGame, Dead, Disconnected
type PlayerState int

// Player States
const (
	Connecting PlayerState = iota
	Connected
	Disconnected
)

// Player stores logic for an individual player
type Player struct {
	Slot   int
	State  PlayerState
	Socket *glue.Socket

	Bike *GridBike
}

// ReadLoop receives data from the player client
func (p *Player) ReadLoop(gs *GameServer) {
	for {
		// Wait for available data.
		// Optional: pass a timeout duration to read.ca
		data, err := p.Socket.Read()
		if err != nil {
			// Just return and release this goroutine if the socket was closed.
			if err == glue.ErrSocketClosed {
				return
			}

			log15.Error("read error", "error", err)
			continue
		}

		components := strings.Split(data, ":")
		switch components[0] {
		case msgdefs.CReqStateMsg:
			p.Socket.Write(msgdefs.SNewStateMsg + ":" + gs.Sim.LatestState)

		case msgdefs.CSpawnMsg:
			if !neededComponents(components, 2) {
				break
			}

			if p.Bike != nil && p.Bike.state != "dead" {
				log15.Debug("Player attempted to spawn with existing bike", "bike", p.Bike)
				break
			}

			p.Bike = gs.Sim.SpawnGridBike(components[1], components[2])
			p.Socket.Write(msgdefs.SDispMsgMsg + ":")

		case msgdefs.CTurnMsg:
			if !neededComponents(components, 1) {
				break
			}
			if p.Bike == nil || p.Bike.state != "move" {
				log15.Debug("Player attemped to turn without existing bike", "bike", p.Bike)
				break
			}
			dir := components[1]
			switch dir {
			case "RIGHT":
				p.Bike.SetTurn(0)
			case "DOWN":
				p.Bike.SetTurn(math.Pi / 2)
			case "LEFT":
				p.Bike.SetTurn(math.Pi)
			case "UP":
				p.Bike.SetTurn(3 * math.Pi / 2)
			default:
				log15.Error("invalid TURN argument", "arg", dir)
			}

		// case "BROADCAST":
		// 	// Echo the received data to all other clients
		// 	for i := range gs.ConnectedPlayers {
		// 		if gs.ConnectedPlayers[i].Socket.ID() == p.Socket.ID() {
		// 			continue
		// 		}
		// 		gs.ConnectedPlayers[i].Socket.Write(strings.Join(components[1:], ":"))
		// 	}

		default:
			log15.Info("Returning unknown request to the client", "request", data)
			p.Socket.Write(data)
		}
	}
}

func neededComponents(components []string, num int) bool {
	for i := range components {
		if i >= num {
			return true
		}
	}
	log15.Error("not enough components given to command", "given", components, "needed", num)
	return false
}

//// Server ////////////////////////////////////////////////////////////////////

// GameServer stores logic for the server and provides callbacks
type GameServer struct {
	// Const
	MaxPlayers int
	Sim        SimManager

	// Var
	Running             bool
	NumConnectedPlayers int
	ConnectedPlayers    map[int]*Player
}

//// Callbacks /////////////////////////////////////////////////////////////////

// New creates a new game server
func New(debug bool, maxPlayers int) *GameServer {
	// New game server
	gs := &GameServer{
		MaxPlayers: maxPlayers,
		Sim: SimManager{
			GridSize: vec2.T{0: 560, 1: 560},
			Rand:     rand.New(rand.NewSource(time.Now().UnixNano())),
		},
	}

	// Debug flag
	if debug {
		log15.Root().SetHandler(log15.CallerFileHandler(log15.StdoutHandler))
	}

	// Initialize vars
	gs.ConnectedPlayers = make(map[int]*Player)

	return gs
}

// Shutdown is called when the gameserver should prepare for program termination
func (gs *GameServer) Shutdown() {
	// add shutdown sequence here
}

// ConnectPlayer handles connecting a new player to the Game
func (gs *GameServer) ConnectPlayer(s *glue.Socket) {
	// Logging
	log15.Debug("socket connected", "address", s.RemoteAddr())
	s.OnClose(func() {
		log15.Debug("socket closed", "address", s.RemoteAddr())
	})

	// Attempt to allocate free player Slot
	if slot := gs.nextSlot(); slot != -1 {
		log15.Info("Accepting new player connection", "slot", slot)
		gs.ConnectedPlayers[slot] = &Player{
			Slot:   slot,
			State:  Connecting,
			Socket: s,
		}
		gs.ConnectedPlayers[slot].Socket.OnClose(func() {
			log15.Debug("socket closed", "address", s.RemoteAddr())
			gs.RemovePlayerOnSlot(slot)
		})
		gs.NumConnectedPlayers++
		gs.ConnectedPlayers[slot].Socket.Write(msgdefs.SConnMsg)
		gs.ConnectedPlayers[slot].Socket.Write(msgdefs.SDispMsgMsg + ":Press [SPACEBAR] To Spawn!")
		go gs.ConnectedPlayers[slot].ReadLoop(gs)

	} else {
		// No free slots available
		log15.Info("Rejecting new player connection: Server is full!")
		s.Write(msgdefs.SFullMsg)
	}

	// go readLoop(s)

	// s.Write("Connected")
}

// RemovePlayerOnSlot handles removing a player from the game
func (gs *GameServer) RemovePlayerOnSlot(slot int) {
	log15.Info("Removing player on slot", "address", gs.ConnectedPlayers[slot].Socket.RemoteAddr(), "slot", slot)
	gs.ConnectedPlayers[slot].State = Disconnected

	gs.NumConnectedPlayers--
	delete(gs.ConnectedPlayers, slot)
}

//// Functions /////////////////////////////////////////////////////////////////

// nextSlot returns the next available player slot, or -1 if no slots available
func (gs *GameServer) nextSlot() int {
	for i := 0; i < gs.MaxPlayers; i++ {
		if _, exists := gs.ConnectedPlayers[i]; !exists {
			return i
		}
	}
	return -1
}
