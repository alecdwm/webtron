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

func SocketHandler(writer http.ResponseWriter, request *http.Request) {
	id := clientHandler.Add(upgrader.Upgrade(writer, request, nil))
	if id == 0 {
		return
	}

	for clientHandler.Active(id) {
		if message, ok := clientHandler.Get(id); ok {
			logrus.WithFields(logrus.Fields{
				"clientID": id,
				"message":  message,
			}).Info("message received")
		}
	}
}
