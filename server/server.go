package server

import (
	"github.com/desertbit/glue"
	"github.com/inconshreveable/log15"
	"go.owls.io/webtron/server/msg"
	"go.owls.io/webtron/server/simulation"
	"golang.org/x/net/websocket"
)

//// Server ////////////////////////////////////////////////////////////////////

// Server is the main webtron server class
// it handles negotiation with new player clients, as well as cleaning up
// disconnected clients
// it also contains a reference to the gameworld simulation
type Server struct {
	MaxClients          int
	NumConnectedPlayers int
	ConnectedPlayers    map[int]*Player

	Sim *simulation.Simulation
}

// New creates a new server
func New(debug bool, maxClients int) *Server {
	// New server
	s := &Server{
		MaxClients:          maxClients,
		NumConnectedPlayers: 0,
		ConnectedPlayers:    make(map[int]*Player),

		Sim: simulation.New(560, 560),
	}

	// Debug flag
	if debug {
		log15.Root().SetHandler(log15.CallerFileHandler(log15.StdoutHandler))
	}

	return s
}

// SocketConnect handles a new client connecting via websockets
func (s *Server) SocketConnect(ws *websocket.Conn) {

	var msg string
	var err error
	for {
		err = websocket.Message.Receive(ws, &msg)
		if err != nil {
			log15.Error("Error reading ws", "error", err)
			break
		}
		log15.Info("Message Received", "message", msg)
	}

	ws.Close()
}

// Shutdown is called when the server should prepare for program termination
func (s *Server) Shutdown() {
	log15.Info("Server shutting down!")
	// add shutdown sequence here
}

// ConnectPlayer handles connecting a new player to the game
func (s *Server) ConnectPlayer(socket *glue.Socket) {
	// Logging
	log15.Debug("socket connected", "address", socket.RemoteAddr())
	socket.OnClose(func() {
		log15.Debug("socket closed", "address", socket.RemoteAddr())
	})

	// Attempt to allocate free player Slot
	if slot := s.nextSlot(); slot != -1 {
		log15.Info("Accepting new player connection", "slot", slot)
		s.ConnectedPlayers[slot] = &Player{
			Server: s,
			Slot:   slot,
			Socket: socket,
		}
		s.ConnectedPlayers[slot].Socket.OnClose(func() {
			log15.Debug("socket closed", "address", socket.RemoteAddr())
			s.DisconnectPlayer(slot)
		})
		s.NumConnectedPlayers++
		s.ConnectedPlayers[slot].Socket.Write(msg.SConnected)
		s.ConnectedPlayers[slot].Socket.Write(msg.SDisplayMessage + ":Press [SPACEBAR] To Spawn!")
		go s.ConnectedPlayers[slot].ReadLoop()

	} else {
		// No free slots available
		log15.Info("Rejecting new player connection: Server is full!")
		socket.Write(msg.SGameFull)
	}
}

// DisconnectPlayer handles removing a player from the game
func (s *Server) DisconnectPlayer(slot int) {
	log15.Info("Player disconnected", "address", s.ConnectedPlayers[slot].Socket.RemoteAddr(), "slot", slot)

	s.NumConnectedPlayers--
	delete(s.ConnectedPlayers, slot)
}

//// Functions /////////////////////////////////////////////////////////////////

// nextSlot returns the next available player slot, or -1 if no slots available
func (s *Server) nextSlot() int {
	for i := 0; i < s.MaxClients; i++ {
		if _, exists := s.ConnectedPlayers[i]; !exists {
			return i
		}
	}
	return -1
}
