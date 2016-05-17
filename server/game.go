package server

import (
	"time"

	"github.com/inconshreveable/log15"
	"go.owls.io/webtron/server/msg"
	"go.owls.io/webtron/server/simulation"
)

type Game struct {
	id     int
	Server *Server

	Players []*Player
	Sim     *simulation.Simulation
}

func (s *Server) NewGame(sizex, sizey float64) *Game {
	return &Game{
		Server:  s,
		Players: nil,
		Sim:     simulation.New(sizex, sizey),
	}
}

func (g *Game) Start() {
	now := time.Now()
	then := now
	dt := now.Sub(then).Seconds()
	accum := dt

	plybotheraccum := dt

	for {
		now = time.Now()
		dt = now.Sub(then).Seconds()
		then = now

		accum += dt
		plybotheraccum += dt

		for i := range g.Players {
			for g.Players[i].client.CanRead() {
				g.ProcessInput(g.Players[i])
			}
			if plybotheraccum > 5 {
				msg := &msg.Msg{Commands: []msg.MsgCommand{{"HELLO", []msg.MsgParameter{{"Reason", []byte("Server is curious")}}}}}
				g.Players[i].client.Write(msg)
			}
		}
		if plybotheraccum > 5 {
			plybotheraccum -= 5
		}

		if accum > (1.0 / 30.0) {
			accum -= (1.0 / 30.0)
			// log15.Debug("Physics update!", "dt", (1.0 / 30.0))
		}
	}
}

func (g *Game) ProcessInput(player *Player) {
	msg := player.client.Read()
	for i := range msg.Commands {
		switch msg.Commands[i].Command {
		case "Sup":
			log15.Info("Printing message", "message", msg.String())
		default:
			player.ProcessCommand(msg.Commands[i])
		}
	}
}

func (g *Game) AddPlayer(player *Player) {
	player.game = g
	g.Players = append(g.Players, player)

	msg := &msg.Msg{Commands: []msg.MsgCommand{{"HELLO", []msg.MsgParameter{{"Reason", []byte("Server is curious")}}}}}
	player.client.Write(msg)
}
