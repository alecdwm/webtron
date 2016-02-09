package server

import (
	"github.com/Sirupsen/logrus"
	"github.com/gorilla/websocket"
	"go.owls.io/webtron/server/msgdefs"
)

//// Enums

// PlayerState i.e. Connecting, InGame, Dead, Disconnected
type PlayerState int

// Player States
const (
	Connecting PlayerState = iota
	InGame
	Dead
	Disconnected
)

//// Structures

// Player stores logic for an individual player
type Player struct {
	ID    int
	State PlayerState
	Conn  *websocket.Conn
}

// GameServer stores logic for the server and provides callbacks
type GameServer struct {
	// Const
	MaxPlayers int

	// Var
	Running             bool
	NumConnectedPlayers int
	ConnectedPlayers    map[int]*Player
}

//// Callbacks

// Run launches the server gameloop
func (gs *GameServer) Run() {
	// Initialize vars
	gs.Running = true
	gs.ConnectedPlayers = make(map[int]*Player)

	// Run gameloop
	for gs.Running {
		GameLoop(gs)
	}
}

// End ends the server gameloop
func (gs *GameServer) End() {
	gs.Running = false
}

// NewPlayer handles connecting a new player to the Game
// ret -1: not accepting new players!
func (gs *GameServer) NewPlayer(conn *websocket.Conn) {
	if id := gs.NextID(); id != -1 {
		logrus.WithField("id", id).Info("New player connecting")
		gs.ConnectedPlayers[id] = &Player{
			ID:    id,
			State: Connecting,
			Conn:  conn,
		}
		go gs.HandlePlayer(id)
	} else {
		logrus.Info("Player attempted to join but server is full")
		conn.WriteMessage(websocket.TextMessage, msgdefs.FullMsg)
		conn.Close()
	}
}

//// Functions

// NextID returns the next available playerID, or -1 if no ID available
func (gs *GameServer) NextID() int {
	for i := 0; i < gs.MaxPlayers; i++ {
		if _, exists := gs.ConnectedPlayers[i]; !exists {
			return i
		}
	}
	return -1
}
