package msgdefs

var (
	// Server -> Client
	SConnMsg     = "CONNECTED" // player has connected
	SFullMsg     = "GAME_FULL" // game is full
	SNewStateMsg = "NEWSTATE"  // new game simulation state available

	// Client -> Server
	CReqStateMsg = "REQUESTSTATE" // request a state update
	CSpawnMsg    = "SPAWN"        // request to spawn a gridbike
	CTurnMsg     = "TURN"         // request to turn this player's gridbike
)
