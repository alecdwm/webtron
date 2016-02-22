package msg

var (
	// Server -> Client
	SConnected      = "CONNECTED"       // player has connected
	SGameFull       = "GAME_FULL"       // game is full
	SNewState       = "NEW_STATE"       // new game simulation state data
	SDisplayMessage = "DISPLAY_MESSAGE" // display message on client

	// Client -> Server
	CRequestState = "REQUEST_STATE" // request a state update
	CSpawn        = "SPAWN"         // request to spawn a gridbike
	CTurn         = "TURN"          // request to turn this player's gridbike
)
