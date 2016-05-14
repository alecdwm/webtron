package server

import (
	"github.com/gorilla/websocket"
	"github.com/inconshreveable/log15"
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
	player *Player

	msgInCh  chan *Msg
	msgOutCh chan *Msg
	doneCh   chan bool
}

func (s *Server) NewClient(id int, conn *websocket.Conn) *Client {
	return &Client{
		id:     id,
		conn:   conn,
		server: s,
		player: NewPlayer(id, s.lobby),

		msgInCh:  make(chan *Msg, s.channelBufSize),
		msgOutCh: make(chan *Msg, s.channelBufSize),
		doneCh:   make(chan bool),
	}
}

func (c *Client) CanRead() bool {
	if len(c.msgInCh) > 0 {
		return true
	}
	return false
}

func (c *Client) Read() []byte {
	msg := <-c.msgInCh
	return msg.content
}

func (c *Client) Write(data []byte) {
	c.msgOutCh <- &Msg{content: data}
}

func (c *Client) WriteLoop() {
	log15.Debug("Open write loop for client", "id", c.id, "address", c.conn.RemoteAddr())
	for {
		select {
		case <-c.doneCh:
			c.server.rmClientCh <- c
			c.doneCh <- true
			return

		case msg := <-c.msgOutCh:
			err := c.conn.WriteMessage(websocket.TextMessage, msg.content)

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
				c.msgInCh <- &Msg{content: message}
			}
		}
	}
}
