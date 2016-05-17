/// <reference path="../lib/msgpack.d.ts" />

namespace Msg {
	export function Unpack(data: ArrayBuffer): Msg {
		var unpacked = msgpack.unpack(new Uint8Array(data))

		if (unpacked == undefined) {
			console.error("Error unpacking data packet from server: " + new Uint8Array(data))
			return undefined
		}

		if (unpacked.Commands == undefined) {
			console.error("Error received data packet from server with no commands: " + new Uint8Array(data))
			return undefined
		}

		var out = new Msg()
		out.Commands = unpacked.Commands

		return out
	}

	export class Msg {
		Commands: MsgCommand[]

		constructor() {
			this.Commands = new Array<MsgCommand>()
		}

		Pack(): Uint8Array {
			var out = new Uint8Array(msgpack.pack(this, false))
			return out
		}
	}

	export class MsgCommand {
		Command: string
		Parameters: MsgParameter[]

		constructor() {
			this.Parameters = new Array<MsgParameter>()
		}
	}

	export class MsgParameter {
		Key: string
		Val: Uint8Array
	}
}
