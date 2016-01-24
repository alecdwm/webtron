package server

import (
	"strconv"

	"github.com/Sirupsen/logrus"
	"github.com/gorilla/websocket"
)

type Client struct {
	Active     bool
	ID         int
	Connection *websocket.Conn
}

type ClientHandler struct {
	LatestID   int
	NumClients int
	Clients    map[int]*Client
}

var clientHandler ClientHandler

// Adds a new websocket connection as a game client
// Returns int>0 as ID of new client, or 0 if there is an error
func (h *ClientHandler) Add(conn *websocket.Conn, err error) int {
	if err != nil {
		logrus.WithError(err).Error("establishing websocket connection")
		return 0
	}

	if h.LatestID == 0 {
		h.Clients = make(map[int]*Client)
	}
	h.LatestID++

	h.Clients[h.LatestID] = &Client{Active: true, ID: h.LatestID, Connection: conn}
	h.NumClients++

	logrus.WithField("ID", h.LatestID).Info("sending ID to newly connected client")
	h.Send(h.LatestID, "ID:"+strconv.Itoa(h.LatestID))

	return h.LatestID
}

func (h *ClientHandler) Active(clientID int) bool {
	if _, exists := h.Clients[clientID]; exists {
		return h.Clients[clientID].Active
	} else {
		logrus.WithField("clientID", clientID).
			Error("attempted to get active status of a client whose ID doesn't exist")
		return false
	}
}

func (h *ClientHandler) Send(clientID int, message string) {
	if _, exists := h.Clients[clientID]; exists {
		err := h.Clients[clientID].Connection.WriteMessage(websocket.TextMessage, []byte(message))
		if err != nil {
			logrus.WithFields(logrus.Fields{
				"clientID": clientID,
				"message":  message,
				"error":    err,
			}).Error("attempted to message a client whose ID does exist")
		}
	} else {
		logrus.WithFields(logrus.Fields{
			"clientID": clientID,
			"message":  message,
		}).Error("attempted to message a client whose ID doesn't exist")
	}
}

func (h *ClientHandler) Get(clientID int) (string, bool) {
	if _, exists := h.Clients[clientID]; exists {
		messageType, message, err := h.Clients[clientID].Connection.ReadMessage()
		if err != nil {
			logrus.WithFields(logrus.Fields{
				"clientID": clientID,
				"error":    err,
			}).Error("attempted to receive message from a client whose ID does exist")
			logrus.WithField("clientID", clientID).Info("setting client to inactive")
			h.Clients[clientID].Active = false
			return "", false
		}

		if messageType != websocket.TextMessage {
			logrus.WithFields(logrus.Fields{
				"clientID":    clientID,
				"messageType": messageType,
				"message":     message,
			}).Info("received non-text message from client")
			return "", false
		}

		return string(message), true

	} else {
		logrus.WithField("clientID", clientID).
			Error("attempted to receive message from a client whose ID doesn't exist")
		return "", false
	}
}
