package server

// import (
// 	"github.com/inconshreveable/log15"
// 	"golang.org/x/net/websocket"
// )

// var nextID int = 0

// type Client struct {
// 	id int
// 	ws *websocket.Conn
// }

// func NewClient(ws *websocket.Conn) *Client {
// 	client := &Client{
// 		id: nextID,
// 		ws: ws,
// 	}
// 	nextID++
// 	return client
// }

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
