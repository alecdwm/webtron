package main

import (
	"bufio"
	"flag"
	"fmt"
	"net/http"
	"os"
	"strings"

	"github.com/desertbit/glue"
	"github.com/inconshreveable/log15"
	"github.com/kardianos/osext"
	"go.owls.io/webtron/server"
)

func main() {
	// Command line variables
	var debug bool
	var daemon bool
	var listenPort string
	var maxPlayers int

	flag.BoolVar(&debug, "debug", false, "run server in debug mode")
	flag.BoolVar(&daemon, "daemon", false, "run server in daemon mode (no console prompt)")
	flag.StringVar(&listenPort, "listenPort", "8080", "port to serve client on")
	flag.IntVar(&maxPlayers, "maxPlayers", 4, "max number of players on server simultaneously")
	flag.Parse()

	// Find executable location
	osextDir, err := osext.ExecutableFolder()
	if err != nil {
		log15.Error("Finding executable location", "error", err)
	}

	// Configure GameClient distributor
	http.Handle("/", http.FileServer(http.Dir(osextDir+"/client")))

	// Configure webtron GameServer
	webtronServer := server.New(debug, maxPlayers)
	defer webtronServer.Shutdown()

	// Configure Glue (websocket wrapper) bridge
	glueServer := glue.NewServer(glue.Options{
		HTTPSocketType:    glue.HTTPSocketTypeNone,
		HTTPListenAddress: ":" + listenPort,
		HTTPHandleURL:     "/",
	})
	defer glueServer.Release()
	glueServer.OnNewSocket(webtronServer.ConnectPlayer)
	http.HandleFunc("/ws", glueServer.ServeHTTP)

	if daemon {
		listenAndServe(listenPort)

	} else {
		// Listen for gameclient/websocket requests on http(s)
		go listenAndServe(listenPort)

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
}

// Listen for gameclient/websocket requests on http(s)
func listenAndServe(listenPort string) {
	err := http.ListenAndServe(":"+listenPort, nil)
	if err != nil {
		log15.Error("serving http(s) requests", "error", err)
	}
}
