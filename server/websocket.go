package server

import (
	"net/http"

	"github.com/Sirupsen/logrus"
	"github.com/gorilla/websocket"
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
}

// SocketHandler handles websocket connections to the game server
func SocketHandler(writer http.ResponseWriter, request *http.Request, gameServer GameServer) {
	conn, err := upgrader.Upgrade(writer, request, nil)
	if err != nil {
		logrus.WithError(err).Error("error upgrading connection to websocket")
		conn.Close()
		return
	}

	gameServer.NewPlayer(conn)
}
