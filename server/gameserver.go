package server

import (
	"strconv"

	"github.com/Sirupsen/logrus"
	"github.com/desertbit/glue"
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

			logrus.Printf("read error: %v", err)
			continue
		}

		// // Echo the received data back to the client.
		// p.Socket.Write(data)

		// Echo the received data to all other clients
		for i := range gs.ConnectedPlayers {
			if gs.ConnectedPlayers[i].Socket.ID() == p.Socket.ID() {
				continue
			}
			gs.ConnectedPlayers[i].Socket.Write(data)
		}
	}
}

//// Server ////////////////////////////////////////////////////////////////////

// GameServer stores logic for the server and provides callbacks
type GameServer struct {
	// Const
	MaxPlayers int

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
	}

	// Debug flag
	if debug {
		logrus.SetLevel(logrus.DebugLevel)
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
	logrus.WithField("address", s.RemoteAddr()).Debug("socket connected")
	s.OnClose(func() {
		logrus.WithField("address", s.RemoteAddr()).Debug("socket closed")
	})

	// Attempt to allocate free player Slot
	if slot := gs.nextSlot(); slot != -1 {
		logrus.WithField("slot", slot).Info("Accepting new player connection")
		gs.ConnectedPlayers[slot] = &Player{
			Slot:   slot,
			State:  Connecting,
			Socket: s,
		}
		gs.ConnectedPlayers[slot].Socket.OnClose(func() {
			logrus.WithField("address", s.RemoteAddr()).Debug("socket closed")
			gs.RemovePlayerOnSlot(slot)
		})
		gs.ConnectedPlayers[slot].Socket.Write("SLOT:" + strconv.Itoa(slot) + ";" + msgdefs.ConnMsg)
		go gs.ConnectedPlayers[slot].ReadLoop(gs)
	} else {
		// No free slots available
		logrus.Info("Rejecting new player connection: Server is full!")
		s.Write(msgdefs.FullMsg)
	}

	// go readLoop(s)

	// s.Write("Connected")
}

// RemovePlayerOnSlot handles removing a player from the game
func (gs *GameServer) RemovePlayerOnSlot(slot int) {
	logrus.WithFields(logrus.Fields{
		"address": gs.ConnectedPlayers[slot].Socket.RemoteAddr(),
		"slot":    slot,
	}).Info("Removing player on slot")
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
