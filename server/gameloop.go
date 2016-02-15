package server

import (
	"log"

	"github.com/desertbit/glue"
)

// clientHandler.NewClient(conn)

// id := clientHandler.Add(upgrader.Upgrade(writer, request, nil))
// if id == 0 {
// 	return
// }
//
// for clientHandler.Active(id) {
// 	if message, ok := clientHandler.Get(id); ok {
// 		clientHandler.Clients[id].ReadMessage(message)
//
// 		logrus.WithFields(logrus.Fields{
// 			"clientID": id,
// 			"message":  message,
// 		}).Info("message received")
// 	}
// }

// HandlePlayer is an io loop running per connected player
func (gs *GameServer) HandlePlayer(id int) {
	for {
		// Wait for available data.
		// Optional: pass a timeout duration to read.
		data, err := gs.ConnectedPlayers[id].Socket.Read()
		if err != nil {
			// Just return and release this goroutine if the socket was closed.
			if err == glue.ErrSocketClosed {
				return
			}

			log.Printf("read error: %v", err)
			continue
		}

		// Echo the received data back to the client.
		gs.ConnectedPlayers[id].Socket.Write(data)
	}
}

// switch gs.ConnectedPlayers[id].State {
// case Connecting:
// messageType, message, err := gs.ConnectedPlayers[id].Conn.ReadMessage()
//
// // if err != nil {
//
// if err != nil {
// 	if !checkDisconnect(clientID, err) {
// 		logrus.WithFields(logrus.Fields{
// 			"clientID": clientID,
// 			"error":    err,
// 		}).Error("attempted to receive message from a client whose ID does exist")
// 		logrus.WithField("clientID", clientID).Info("setting client to inactive")
// 		h.Clients[clientID].Active = false
// 		h.Clients[clientID].Connection.Close()
// 		return "", false
// 	}
// }
//
// if messageType != websocket.TextMessage {
// 	logrus.WithFields(logrus.Fields{
// 		"clientID":    clientID,
// 		"messageType": messageType,
// 		"message":     message,
// 	}).Info("received non-text message from client")
// 	return "", false
// }
//
// return string(message), true
// gs.ConnectedPlayers[id].Conn.WriteMessage(websocket.TextMessage, msgdefs.ConnMsg)

// case InGame:

// case Dead:

// case Disconnected:
// gs.ConnectedPlayers[id].Conn.Close()
// gs.NumConnectedPlayers--
// delete(gs.ConnectedPlayers, id)
// }
// }

// GameLoop contains server looping game logic
func GameLoop(gs *GameServer) {

}
