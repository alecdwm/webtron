package main

import "fmt"

type Client struct {
	ID         int
	Connection int
}

type Game struct {
	Players map[int]*Client
}

func (*Game) AddPlayer(Client) {
	this.Players = append(this.Players, Client)
}

type WebtronServer struct {
	Clients map[int]*Client
	Games   map[int]*Game
}

func main() {
	fmt.Println("rawr")
}
