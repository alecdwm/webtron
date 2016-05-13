package server

import (
	"net/http"

	"github.com/gorilla/websocket"
	"github.com/inconshreveable/log15"
	"go.owls.io/webtron/server/msg"
)

//// Server ////////////////////////////////////////////////////////////////////

// Server is the main webtron server class
// it handles negotiation with new player clients, as well as cleaning up
// disconnected clients
// it also contains a reference to the gameworld simulation
type Server struct {
	MaxClients int
	NumClients int
	Clients    map[int]*Client

	channelBufSize int
	mkClientCh     chan *websocket.Conn
	rmClientCh     chan *Client

	wsUpgrader *websocket.Upgrader
	games      []*Game
}

type Config struct {
	Pattern        string
	Debug          bool
	MaxClients     int
	ChannelBufSize int
}

// New creates a new server
func New(c *Config) *Server {

	// New server
	s := &Server{
		MaxClients: c.MaxClients,
		NumClients: 0,
		Clients:    make(map[int]*Client),

		channelBufSize: c.ChannelBufSize,
		mkClientCh:     make(chan *websocket.Conn),
		rmClientCh:     make(chan *Client),

		wsUpgrader: &websocket.Upgrader{
			ReadBufferSize:  1024,
			WriteBufferSize: 1024,
		},
		games: nil,
	}

	// Listen for client connections
	http.HandleFunc(c.Pattern, s.NewSocket)

	// Debug flag
	if c.Debug {
		log15.Root().SetHandler(log15.CallerFileHandler(log15.StdoutHandler))
	}

	return s
}

func (s *Server) Start() {
	for {
		select {

		// Listen for creating clients
		case c := <-s.mkClientCh:
			if id := s.nextSlot(); id != -1 {
				log15.Info("Accepting new client connection", "id", id)
				s.Clients[id] = s.NewClient(id, c)
				s.NumClients++
				go s.Clients[id].ReadLoop()
				go s.Clients[id].WriteLoop()

			} else {
				log15.Info("Rejecting new client connection: Server is full!")
				err := c.WriteMessage(websocket.TextMessage, []byte(msg.SGameFull))
				if err != nil {
					log15.Error("Writing server full message to websocket", "error", err, "address", c.RemoteAddr())
				}
				err = c.Close()
				if err != nil {
					log15.Error("closing websocket", "error", err, "address", c.RemoteAddr())
				}
			}

		// Listen for removing clients
		case c := <-s.rmClientCh:
			if _, exists := s.Clients[c.id]; exists {
				delete(s.Clients, c.id)
				s.NumClients--
				log15.Info("Removed client", "id", c.id)
			}
		}
	}
}

// NewSocket handles a new client connecting via websockets
func (s *Server) NewSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := s.wsUpgrader.Upgrade(w, r, nil)
	if err != nil {
		log15.Error("unable to upgrade websocket connection", "address", r.RemoteAddr)
	}

	// Logging
	log15.Debug("socket connected", "address", conn.RemoteAddr())

	// Add client to server
	s.mkClientCh <- conn
}

// Shutdown is called when the server should prepare for program termination
func (s *Server) Shutdown() {
	for i := range s.Clients {
		_ = s.Clients[i].conn.WriteMessage(websocket.TextMessage, []byte(msg.SShutdown))
	}
	log15.Info("Server shutting down!")
}

//// Functions /////////////////////////////////////////////////////////////////

// nextSlot returns the next available player slot, or -1 if no slots available
func (s *Server) nextSlot() int {
	for i := 0; i < s.MaxClients; i++ {
		if _, exists := s.Clients[i]; !exists {
			return i
		}
	}
	return -1
}
