package server

import (
	"github.com/gorilla/websocket"
	"go.owls.io/webtron/server/msgdefs"
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
	switch gs.ConnectedPlayers[id].State {
	case Connecting:
		gs.ConnectedPlayers[id].Conn.WriteMessage(websocket.TextMessage, msgdefs.ConnMsg)

	case InGame:

	case Dead:

	case Disconnected:
		gs.ConnectedPlayers[id].Conn.Close()
		gs.NumConnectedPlayers--
		delete(gs.ConnectedPlayers, id)
	}
}

// GameLoop contains server looping game logic
func GameLoop(gs *GameServer) {

}
