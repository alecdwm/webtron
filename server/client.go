package server

//
// import (
// 	"strings"
//
// 	"github.com/Sirupsen/logrus"
// 	"github.com/gorilla/websocket"
// )
//
// // Client represents a connection to a player's client
// type Client struct {
// 	Active     bool
// 	ID         int
// 	Connection *websocket.Conn
//
// 	Name string
// }
//
// // ReadMessage is called in the websocket loop to read the next message sent
// // from this client
// func (c *Client) ReadMessage(message string) {
// 	instructions := strings.Split(message, ";")
// 	for _, instruction := range instructions {
// 		c.RunCommand(instruction)
// 	}
// }
//
// // RunCommand parses an instruction from this client and acts on it
// func (c *Client) RunCommand(instruction string) {
// 	components := strings.Split(instruction, ":")
// 	if len(components) != 2 {
// 		logrus.WithField("components", len(components)).
// 			Error("Number of components in instruction did not equal 2!")
// 	}
//
// 	switch components[0] {
// 	case "NAME":
// 		c.Name = components[1]
//
// 	case "RELOAD":
// 		for _, client := range clientHandler.Clients {
// 			clientHandler.Send(c.ID, "NAME:"+client.Name)
// 		}
// 	}
// }
