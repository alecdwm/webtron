package server

import (
	"github.com/gorilla/websocket"
	"github.com/inconshreveable/log15"
	"go.owls.io/webtron/server/msg"
)

//// Client ////////////////////////////////////////////////////////////////////

type Client struct {
	id     int
	conn   *websocket.Conn
	server *Server
	player *Player

	msgInCh  chan *msg.Msg
	msgOutCh chan *msg.Msg
	doneCh   chan bool
}

func (s *Server) NewClient(id int, conn *websocket.Conn) *Client {
	c := &Client{
		id:     id,
		conn:   conn,
		server: s,
		player: nil,

		msgInCh:  make(chan *msg.Msg, s.channelBufSize),
		msgOutCh: make(chan *msg.Msg, s.channelBufSize),
		doneCh:   make(chan bool),
	}

	c.player = c.NewPlayer(id, s.Lobby)

	return c
}

func (c *Client) CanRead() bool {
	if len(c.msgInCh) > 0 {
		return true
	}
	return false
}

func (c *Client) Read() *msg.Msg {
	msg := <-c.msgInCh
	return msg
}

func (c *Client) Write(msg *msg.Msg) {
	c.msgOutCh <- msg
}

func (c *Client) WriteLoop() {
	log15.Debug("Open write loop for client", "id", c.id, "address", c.conn.RemoteAddr())
	for {
		select {
		case <-c.doneCh:
			c.server.rmClientCh <- c
			c.doneCh <- true
			return

		// TODO:
		// consider adding an alternative msgOutCh for simple one-word 'text'
		// messages with no msgpack overhead.
		// Maybe useful for triggering/pinging/simple shit.
		// Would send with websocket message type websocket.TextMessage
		// case msg := <-c.txtMsgOutCh:

		case msg := <-c.msgOutCh:
			packed, err := msg.Pack()
			if err != nil {
				log15.Error("packing outgoing message", "error", err, "id", c.id, "address", c.conn.RemoteAddr(), "message", msg.String())
			}

			err = c.conn.WriteMessage(websocket.BinaryMessage, packed)

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
			c.doneCh <- true
			return

		default:
			messageType, message, err := c.conn.ReadMessage()
			if err != nil {
				log15.Error("reading from client socket", "error", err, "id", c.id, "address", c.conn.RemoteAddr())
				c.doneCh <- true
				return
			} else {
				log15.Debug("received message", "type", messageType, "message", string(message), "id", c.id, "address", c.conn.RemoteAddr())
				if messageType != websocket.BinaryMessage {
					log15.Error("dropping incoming message with unsupported message type", "type", messageType, "supported", "websockets.BinaryMessage")
				} else {
					msg, err := msg.Unpack(message)
					if err != nil {
						log15.Error("dropping incoming message which couldn't be unpacked", "error", err, "message", message, "message (string)", string(message), "id", c.id, "address", c.conn.RemoteAddr())
					} else {
						c.msgInCh <- msg
					}
				}
			}
		}
	}
}
