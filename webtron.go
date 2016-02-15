package main

import (
	"bufio"
	"flag"
	"fmt"
	"net/http"
	"os"
	"strings"

	"github.com/Sirupsen/logrus"
	"github.com/desertbit/glue"
	"github.com/kardianos/osext"
	"go.owls.io/webtron/server"
)

func main() {
	// Command line variables
	var debug bool
	var listenPort string
	var maxPlayers int

	flag.BoolVar(&debug, "debug", false, "run server in debug mode")
	flag.StringVar(&listenPort, "listenPort", "8080", "port to serve client on")
	flag.IntVar(&maxPlayers, "maxPlayers", 4, "max number of players on server simultaneously")
	flag.Parse()

	// Find executable location
	osextDir, err := osext.ExecutableFolder()
	if err != nil {
		logrus.WithError(err).Error("Finding executable location")
	}

	// Configure GameClient distributor
	http.Handle("/", http.FileServer(http.Dir(osextDir+"/client")))

	// Configure GameServer
	var webtronServer = server.GameServer{
		MaxPlayers: maxPlayers,
	}

	// Start GameServer
	go webtronServer.Run(debug)
	defer webtronServer.End()

	// Configure Glue (websocket wrapper) bridge
	glueServer := glue.NewServer(glue.Options{
		HTTPSocketType:    glue.HTTPSocketTypeNone,
		HTTPListenAddress: ":" + listenPort,
		HTTPHandleURL:     "/",
	})
	defer glueServer.Release()
	glueServer.OnNewSocket(webtronServer.ConnectPlayer)
	http.HandleFunc("/ws", glueServer.ServeHTTP)

	// Listen for gameclient/websocket requests on http(s)
	go func() {
		err := http.ListenAndServe(":"+listenPort, nil)
		if err != nil {
			logrus.WithError(err).Error("serving http(s) requests")
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
