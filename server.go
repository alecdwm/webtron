package main

import (
	"flag"
	"net/http"

	"github.com/Sirupsen/logrus"
)

func main() {
	var listenPort string
	flag.StringVar(&listenPort, "listenPort", "8080", "port to listen on")
	flag.Parse()

	err := http.ListenAndServe(":"+listenPort, http.FileServer(http.Dir("./client")))
	if err != nil {
		logrus.WithError(err).Fatal("fileserver quit with fatal error")
	}
}
