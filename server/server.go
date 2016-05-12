package server

import (
	"net/http"

	"github.com/inconshreveable/log15"
	"go.owls.io/webtron/server/msg"
	"go.owls.io/webtron/server/simulation"
	"golang.org/x/net/websocket"
)

//// Msg ///////////////////////////////////////////////////////////////////////

type Msg struct {
	content []byte
}

//// Client ////////////////////////////////////////////////////////////////////

type Client struct {
	id     int
	conn   *websocket.Conn
	server *Server

	msgInCh  chan *Msg
	msgOutCh chan *Msg
	doneCh   chan bool
}

func (s *Server) NewClient(id int, conn *websocket.Conn) *Client {
	return &Client{
		id:     id,
		conn:   conn,
		server: s,

		msgInCh:  make(chan *Msg, s.channelBufSize),
		msgOutCh: make(chan *Msg, s.channelBufSize),
		doneCh:   make(chan bool),
	}
}

func (c *Client) WriteLoop() {
	log15.Debug("Open write loop for client", "id", c.id, "address", c.conn.RemoteAddr())
	for {
		select {
		case <-c.doneCh:
			c.server.rmClientCh <- c
			return

		case msg := <-c.msgOutCh:
			err := websocket.Message.Send(c.conn, msg.content)

			if err != nil {
				log15.Error("writing to client socket", "error", err, "id", c.id, "address", c.conn.RemoteAddr())
				c.doneCh <- true
				return
			} else {
				log15.Debug("sent message", "id", c.id, "address", c.conn.RemoteAddr())
			}
		}
	}
}

func (c *Client) ReadLoop() {
	log15.Debug("Open read loop for client", "id", c.id, "address", c.conn.RemoteAddr())
	for {
		select {
		case <-c.doneCh:
			c.server.rmClientCh <- c
			return

		default:
			msg := &Msg{}
			err := websocket.Message.Receive(c.conn, &msg.content)
			if err != nil {
				log15.Error("reading from client socket", "error", err, "id", c.id, "address", c.conn.RemoteAddr())
				c.doneCh <- true
				return
			} else {
				log15.Debug("received message", "id", c.id, "address", c.conn.RemoteAddr())
			}
		}
	}
}

// func (c *Client) Send(msg []byte) {
// 	_, err := c.ws.Write(msg)
// 	if err != nil {
// 		log15.Error("sending message", "error", err)
// 	}
// }

// func (c *Client) Listen() {
// 	var msg []byte
// 	var err error
// 	for {
// 		err = websocket.Message.Receive(c.ws, msg)
// 		if err != nil {
// 			log15.Error("receiving message", "error", err)
// 		}
// 	}
// }

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

	Sim *simulation.Simulation
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

		Sim: simulation.New(560, 560),
	}

	// Listen for client connections
	http.Handle(c.Pattern, websocket.Handler(s.NewSocket))

	// Debug flag
	if c.Debug {
		log15.Root().SetHandler(log15.CallerFileHandler(log15.StdoutHandler))
	}

	return s
}

func (s *Server) Start() {
	for {
		select {
		case c := <-s.mkClientCh:
			if id := s.nextSlot(); id != -1 {
				log15.Info("Accepting new client connection", "id", id)
				s.Clients[id] = s.NewClient(id, c)
				s.NumClients++
				go s.Clients[id].ReadLoop()
				go s.Clients[id].WriteLoop()

			} else {
				log15.Info("Rejecting new client connection: Server is full!")
				err := websocket.Message.Send(c, msg.SGameFull)
				if err != nil {
					log15.Error("Writing server full message to websocket", "address", c.RemoteAddr(), "error", err)
				}
				err = c.Close()
				if err != nil {
					log15.Error("closing websocket", "address", c.RemoteAddr(), "error", err)
				}
			}

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
func (s *Server) NewSocket(ws *websocket.Conn) {
	// Logging
	log15.Debug("socket connected", "address", ws.RemoteAddr())

	// Add client to server
	s.mkClientCh <- ws
}

// Shutdown is called when the server should prepare for program termination
func (s *Server) Shutdown() {
	log15.Info("Server shutting down!")
	// add shutdown sequence here
}

// // ConnectPlayer handles connecting a new player to the game
// func (s *Server) ConnectPlayer(socket *glue.Socket) {
// 	// Logging
// 	socket.OnClose(func() {
// 		log15.Debug("socket closed", "address", socket.RemoteAddr())
// 	})
// 		s.Clients[slot].Socket.OnClose(func() {
// 			log15.Debug("socket closed", "address", socket.RemoteAddr())
// 			s.DisconnectPlayer(slot)
// 		})
// 		// s.Clients[slot].Socket.Write(msg.SConnected)
// 		// s.Clients[slot].Socket.Write(msg.SDisplayMessage + ":Press [SPACEBAR] To Spawn!")
// 		// go s.Clients[slot].ReadLoop()
// }

// // DisconnectPlayer handles removing a player from the game
// func (s *Server) DisconnectPlayer(slot int) {
// 	log15.Info("Player disconnected", "address", s.Clients[slot].Socket.RemoteAddr(), "slot", slot)

// 	s.NumClients--
// 	delete(s.Clients, slot)
// }

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
