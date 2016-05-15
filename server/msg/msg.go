package msg

import "gopkg.in/vmihailenco/msgpack.v2"

//// Msg ///////////////////////////////////////////////////////////////////////

// A msg object can store multiple 'commands'
type Msg struct {
	Commands []MsgCommand
}

// A command consists of a string and any number of key->val parameters
type MsgCommand struct {
	Command    string
	Parameters []MsgParameter
}

// A parameter consists of a string key associated with a byte-array value
type MsgParameter struct {
	Key string
	Val []byte
}

// Unpacked a msgpack-encoded packet into a msg object
func Unpack(data []byte) (unpacked *Msg, err error) {
	err = msgpack.Unmarshal(data, &unpacked)
	if err != nil {
		return nil, err
	}
	return unpacked, nil
}

// Pack this msg object into a msgpack-encoded packet
func (m *Msg) Pack() ([]byte, error) {
	data, err := msgpack.Marshal(m)
	if err != nil {
		return nil, err
	}
	return data, nil
}

// Return a string representation of this msg object for debugging
func (m *Msg) String() (out string) {
	for i := range m.Commands {
		out += m.Commands[i].Command + "\n"
		for v := range m.Commands[i].Parameters {
			out += "\t" + m.Commands[i].Parameters[v].Key + "\t=>\t" + string(m.Commands[i].Parameters[v].Val) + "\n"
		}
		out += "\n"
	}
	return out
}

// Add a command to this msg object
func (m *Msg) AddCommand(command string, parameters []MsgParameter) {
	m.Commands = append(m.Commands, MsgCommand{command, parameters})
}

// Remove a command from this msg object (reference command by its name)
func (m *Msg) RemoveCommand(command string) {
	for i := range m.Commands {
		if m.Commands[i].Command == command {
			m.Commands = append(m.Commands[:i], m.Commands[i+1:]...)
		}
	}
}

// Add a parameter to a command on this msg object
func (m *Msg) AddParameter(command string, parameter MsgParameter) {
	for i := range m.Commands {
		if m.Commands[i].Command == command {
			m.Commands[i].Parameters = append(m.Commands[i].Parameters, parameter)
		}
	}
}

// Remove a parameter from a command on this msg object (reference param by its key)
func (m *Msg) RemoveParameter(command string, key string) {
	for i := range m.Commands {
		if m.Commands[i].Command == command {
			for v := range m.Commands[i].Parameters {
				if m.Commands[i].Parameters[v].Key == key {
					m.Commands[i].Parameters = append(m.Commands[i].Parameters[:v], m.Commands[i].Parameters[v+1:]...)
				}
			}
		}
	}
}

//// Preconstructed Messages ///////////////////////////////////////////////////

var (
	Disconnect_ServerFull               = &Msg{[]MsgCommand{{"DISCONNECT", []MsgParameter{{"Reason", []byte("Server is full")}}}}}
	Disconnect_ServerFull_Packed, _     = Disconnect_ServerFull.Pack()
	Disconnect_ServerShutdown           = &Msg{[]MsgCommand{{"DISCONNECT", []MsgParameter{{"Reason", []byte("Server is shutting down")}}}}}
	Disconnect_ServerShutdown_Packed, _ = Disconnect_ServerShutdown.Pack()

	// // Server -> Client
	// SConnected      = &Msg{content: []byte("CONNECTED")}       // player has connected
	// SGameFull       = &Msg{content: []byte("GAME_FULL")}       // game is full
	// SNewState       = &Msg{content: []byte("NEW_STATE")}       // new game simulation state data
	// SDisplayMessage = &Msg{content: []byte("DISPLAY_MESSAGE")} // display message on client
	// SShutdown       = &Msg{content: []byte("SERVER_SHUTDOWN")} // server shutting down

	// // Client -> Server
	// CRequestState = &Msg{content: []byte("REQUEST_STATE")} // request a state update
	// CSpawn        = &Msg{content: []byte("SPAWN")}         // request to spawn a gridbike
	// CTurn         = &Msg{content: []byte("TURN")}          // request to turn this player's gridbike
)
