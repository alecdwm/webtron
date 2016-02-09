package server

import (
	"log"

	"github.com/Sirupsen/logrus"
	"github.com/desertbit/glue"
)

// var upgrader = websocket.Upgrader{
// 	ReadBufferSize:  1024,
// 	WriteBufferSize: 1024,
// }
//
// // SocketHandler handles websocket connections to the game server
// func SocketHandler(writer http.ResponseWriter, request *http.Request, gameServer GameServer) {
// 	conn, err := upgrader.Upgrade(writer, request, nil)
// 	if err != nil {
// 		logrus.WithError(err).Error("error upgrading connection to websocket")
// 		conn.Close()
// 		return
// 	}
//
// 	gameServer.NewPlayer(conn)
// }

// OnNewSocket handles new websocket connections to the game server
func OnNewSocket(s *glue.Socket) {
	logrus.WithField("address", s.RemoteAddr()).Info("socket connected")

	s.OnClose(func() {
		logrus.WithField("address", s.RemoteAddr()).Info("socket closed")
	})

	go readLoop(s)

	s.Write("Connected")
}

func readLoop(s *glue.Socket) {
	for {
		// Wait for available data.
		// Optional: pass a timeout duration to read.
		data, err := s.Read()
		if err != nil {
			// Just return and release this goroutine if the socket was closed.
			if err == glue.ErrSocketClosed {
				return
			}

			log.Printf("read error: %v", err)
			continue
		}

		// Echo the received data back to the client.
		s.Write(data)
	}
}
