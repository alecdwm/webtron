package server

import "go.owls.io/webtron/server/simulation"

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
	// for {

	// }
}

func (g *Game) AddPlayer(player *Player) {
	player.game = g
	g.Players = append(g.Players, player)
}
