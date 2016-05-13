package server

import "go.owls.io/webtron/server/simulation"

type Game struct {
	Players []*Player
	Sim     *simulation.Simulation
}

func NewGame(sizex, sizey float64) *Game {
	return &Game{
		Players: nil,
		Sim:     simulation.New(sizex, sizey),
	}
}

func (g *Game) AddPlayer(player *Player) {
	player.game = g
	g.Players = append(g.Players, player)
}
