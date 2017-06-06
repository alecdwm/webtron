package main

import (
	"bufio"
	"flag"
	"fmt"
	"net/http"
	"net/http/httputil"
	"net/url"
	"os"
	"os/exec"
	"strings"
	"syscall"

	"github.com/inconshreveable/log15"
	"github.com/kardianos/osext"
	"go.owls.io/webtron/server"
)

func main() {
	// Command line variables
	var development bool
	var debug bool
	var daemon bool
	var bindAddress string
	var listenPort string
	var maxClients int

	flag.BoolVar(&development, "development", false, "run server in development mode (proxy connections to webpack-dev-server)")
	flag.BoolVar(&debug, "debug", false, "run server in debug mode")
	flag.BoolVar(&daemon, "daemon", false, "run server in daemon mode (no console prompt)")
	flag.StringVar(&bindAddress, "bindAddress", "0.0.0.0", "address to bind to")
	flag.StringVar(&listenPort, "listenPort", "8000", "port to serve client on")
	flag.IntVar(&maxClients, "maxClients", 8, "maximum simultaneous client connections")
	flag.Parse()

	// Find executable location
	osextDir, err := osext.ExecutableFolder()
	if err != nil {
		log15.Error("Finding executable location", "error", err)
	}

	// Setup game server
	gameServer := server.New(&server.Config{
		Pattern:        "/ws",
		Debug:          debug,
		MaxClients:     maxClients,
		ChannelBufSize: 100,
	})

	// Setup client webserver
	if development {
		webpackDevelopment := exec.Command("node", "./node_modules/.bin/webpack-dev-server", "--inline", "--progress", "--colors")

		// kill child process when main process exits
		webpackDevelopment.SysProcAttr = &syscall.SysProcAttr{Pdeathsig: syscall.SIGTERM}

		webpackDevelopment.Dir = osextDir
		webpackDevelopment.Stdout = os.Stdout
		webpackDevelopment.Stderr = os.Stderr
		err := webpackDevelopment.Start()
		if err != nil {
			log15.Error("running webpack-dev-server", "error", err)
		}

		url, err := url.Parse(fmt.Sprintf("http://%s:3000/", bindAddress))
		if err != nil {
			log15.Error("parsing url", "error", err)
		}
		reverseProxy := httputil.NewSingleHostReverseProxy(url)
		http.Handle("/", reverseProxy)

	} else {
		searchPaths := []string{
			osextDir + "/webtron-client",
			osextDir + "/client/bin",
		}
		searchFound := false
		for _, path := range searchPaths {
			if _, err := os.Stat(path); err == nil {
				searchFound = true
				http.Handle("/", http.FileServer(http.Dir(path)))
				break
			}
		}
		if !searchFound {
			log15.Error("unable to locate client files", "searchPaths", searchPaths)
		}
	}

	// Start game server
	go gameServer.Start()
	defer gameServer.Shutdown()

	if daemon {
		listenAndServe(bindAddress, listenPort)

	} else {
		// Listen for gameclient/websocket requests on http(s)
		go listenAndServe(bindAddress, listenPort)

		// Server command line
		cli(osextDir)
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
func cli(osextDir string) {
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
