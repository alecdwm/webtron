package main

import (
	"bufio"
	"flag"
	"fmt"
	"net/http"
	"os"
	"strings"

	"github.com/Sirupsen/logrus"
	"go.owls.io/webtron/server"
)

func main() {
	// Client Vars
	var listenPort string
	flag.StringVar(&listenPort, "listenPort", "8080", "port to serve client on")

	// Server Vars
	var maxPlayers int
	flag.IntVar(&maxPlayers, "maxPlayers", 4, "max number of players on server simultaneously")

	flag.Parse()

	// Configure GameServer
	var webtronServer = server.GameServer{
		MaxPlayers: maxPlayers,
	}

	// Start GameServer
	go webtronServer.Run()
	defer webtronServer.End()

	// Configure WebSocket bridge
	http.HandleFunc("/ws", func(writer http.ResponseWriter, request *http.Request) {
		server.SocketHandler(writer, request, webtronServer)
	})

	// Configure GameClient distributor
	http.Handle("/", http.FileServer(http.Dir("client")))

	// Listen for gameclient/websocket requests on http
	go func() {
		err := http.ListenAndServe(":"+listenPort, nil)
		if err != nil {
			logrus.WithError(err).Error("serving http requests")
		}
	}()

	// Command line input
	reader := bufio.NewReader(os.Stdin)
input_loop:
	for {
		fmt.Print("console@webtron:~$ ")
		input, _ := reader.ReadString('\n')
		input = strings.Trim(input, "\n")
		switch input {
		case "":

		case "help":
			fmt.Println(
				"Available commands:\n" +
					"help, exit")

		case "exit", "quit":
			break input_loop

		default:
			fmt.Println("Unknown command: " + input)
		}
	}
}
