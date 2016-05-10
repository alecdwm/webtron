package main

import (
	"bufio"
	"flag"
	"fmt"
	"net/http"
	"os"
	"strings"

	"github.com/inconshreveable/log15"
	"github.com/kardianos/osext"
	"go.owls.io/webtron/server"
	"golang.org/x/net/websocket"
)

func main() {
	// Command line variables
	var debug bool
	var daemon bool
	var bindAddress string
	var listenPort string
	var maxClients int

	flag.BoolVar(&debug, "debug", false, "run server in debug mode")
	flag.BoolVar(&daemon, "daemon", false, "run server in daemon mode (no console prompt)")
	flag.StringVar(&bindAddress, "bindAddress", "0.0.0.0", "address to bind to")
	flag.StringVar(&listenPort, "listenPort", "8080", "port to serve client on")
	flag.IntVar(&maxClients, "maxClients", 8, "maximum simultaneous client connections")
	flag.Parse()

	// Find executable location
	osextDir, err := osext.ExecutableFolder()
	if err != nil {
		log15.Error("Finding executable location", "error", err)
	}

	// Setup game server
	gameServer := server.New(debug, maxClients)
	defer gameServer.Shutdown()
	http.Handle("/ws", websocket.Handler(gameServer.SocketConnect))

	// Setup client webserver
	http.Handle("/", http.FileServer(http.Dir(osextDir+"/client")))

	if daemon {
		listenAndServe(bindAddress, listenPort)

	} else {
		// Listen for gameclient/websocket requests on http(s)
		go listenAndServe(bindAddress, listenPort)

		// Server command line
		cli()
	}
}

// Listen for gameclient/websocket requests on http(s)
func listenAndServe(bindAddress, listenPort string) {
	err := http.ListenAndServe(bindAddress+":"+listenPort, nil)
	if err != nil {
		log15.Error("serving http(s) requests", "error", err)
	}
}

// Server command line interface
func cli() {
	reader := bufio.NewReader(os.Stdin)
cli_input_loop:
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
			break cli_input_loop

		default:
			fmt.Println("Unknown command: " + input)
		}
	}
}
