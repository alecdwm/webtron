package main

import (
	"flag"
	"net/http"

	"github.com/Sirupsen/logrus"
	"go.owls.io/webtron/server"
)

func main() {
	var listenPort string
	flag.StringVar(&listenPort, "listenPort", "8080", "port to listen on")
	flag.Parse()

	http.HandleFunc("/ws", server.SocketHandler)
	http.Handle("/", http.FileServer(http.Dir("./client")))

	err := http.ListenAndServe(":"+listenPort, nil)
	if err != nil {
		logrus.WithError(err).Error("serving http requests")
	}
}
