package server

import (
	"math"
	"strings"

	"github.com/desertbit/glue"
	"github.com/inconshreveable/log15"
	"go.owls.io/webtron/server/msg"
	"go.owls.io/webtron/server/simulation"
)

//// Player ////////////////////////////////////////////////////////////////////

// Player stores logic for an individual player
type Player struct {
	Slot   int
	Server *Server
	Socket *glue.Socket

	Bike *simulation.GridBike
}

// ReadLoop receives data from the player client
func (p *Player) ReadLoop() {
	for {
		// Wait for available data
		data, err := p.Socket.Read()
		if err != nil {
			// return and release this goroutine if the socket was closed
			if err == glue.ErrSocketClosed {
				return
			}

			log15.Error("read error", "error", err)
			continue
		}

		components := strings.Split(data, ":")
		switch components[0] {
		case msg.CRequestState:
			p.Socket.Write(msg.SNewState + ":" + p.Server.Sim.LatestState)

		case msg.CSpawn:
			if !p.neededComponents(components, 2) {
				break
			}

			if p.Bike != nil && p.Bike.GetState() != "dead" {
				log15.Debug("Player attempted to spawn with existing bike", "bike", p.Bike)
				break
			}

			p.Bike = p.Server.Sim.SpawnGridBike(components[1], components[2])
			p.Socket.Write(msg.SDisplayMessage + ":")

		case msg.CTurn:
			if !p.neededComponents(components, 1) {
				break
			}
			if p.Bike == nil || p.Bike.GetState() != "move" {
				log15.Debug("Player attemped to turn without existing bike", "bike", p.Bike)
				break
			}
			dir := components[1]
			switch dir {
			case "RIGHT":
				p.Bike.SetTurn(0)
			case "DOWN":
				p.Bike.SetTurn(math.Pi / 2)
			case "LEFT":
				p.Bike.SetTurn(math.Pi)
			case "UP":
				p.Bike.SetTurn(3 * math.Pi / 2)
			default:
				log15.Error("invalid TURN argument", "arg", dir)
			}

		// case msg.CBroadcast:
		// 	// Echo the received data to all other clients
		// 	for i := range gs.ConnectedPlayers {
		// 		if gs.ConnectedPlayers[i].Socket.ID() == p.Socket.ID() {
		// 			continue
		// 		}
		// 		gs.ConnectedPlayers[i].Socket.Write(strings.Join(components[1:], ":"))
		// 	}

		default:
			log15.Info("Returning unknown request to the client", "request", data)
			p.Socket.Write(data)
		}
	}
}

func (p *Player) neededComponents(components []string, num int) bool {
	for i := range components {
		if i >= num {
			return true
		}
	}
	log15.Error("not enough components given to command", "given", components, "needed", num)
	return false
}
