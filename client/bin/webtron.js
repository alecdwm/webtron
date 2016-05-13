var __extends = (this && this.__extends) || function (d, b) {
    for (var p in b) if (b.hasOwnProperty(p)) d[p] = b[p];
    function __() { this.constructor = d; }
    d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
};
define("webtron", ["require", "exports", 'jquery'], function (require, exports, $) {
    "use strict";
    var Webtron;
    (function (Webtron) {
        var playerName = "", playerColor = "", socket, serverMsg = "", uiFont = '"Courier New", Courier, monospace', colors = [
            "orange",
            "blue",
            "green",
            "purple",
            "red",
            "white"
        ], colorsToHexString = {
            'blue': "#00c2cc",
            'green': "#2ee5c7",
            'orange': "#f2d91a",
            'purple': "#8a2ee5",
            'red': "#e5482e",
            'white': "#e5feff",
        }, colorsToHex = {
            'blue': 0x00c2cc,
            'green': 0x2ee5c7,
            'orange': 0xf2d91a,
            'purple': 0x8a2ee5,
            'red': 0xe5482e,
            'white': 0xe5feff,
        };
        var Game = (function (_super) {
            __extends(Game, _super);
            function Game() {
                var width = 560, height = 560, renderer = Phaser.WEBGL, parent = "webtron", state = null, transparent = true, antialias = true;
                _super.call(this, width, height, renderer, parent, state, transparent, antialias);
                this.state.add("mainmenu", MainMenu, false);
                this.state.add("connect", Connect, false);
                this.state.add("gamemenu", GameMenu, false);
                this.state.add("ingame", InGame, false);
                this.state.start("mainmenu");
            }
            return Game;
        }(Phaser.Game));
        Webtron.Game = Game;
        var MainMenu = (function (_super) {
            __extends(MainMenu, _super);
            function MainMenu() {
                _super.apply(this, arguments);
            }
            MainMenu.prototype.preload = function () {
                this.game.load.image("button_name", "img/button_name.png");
                this.game.load.image("button_color", "img/button_color.png");
                this.game.load.image("button_join", "img/button_join.png");
                this.game.load.audio("scifi5", ["sfx/scifi5.mp3"]);
                this.game.load.audio("keyboard_key", ["sfx/keyboard_key.mp3"]);
            };
            MainMenu.prototype.create = function () {
                playerName = (playerName == "") ? "" : playerName;
                playerColor = (playerColor == "") ? colors[0] : playerColor;
                this.nameMaxLength = 10;
                this.game.input.keyboard.callbackContext = this;
                this.game.input.keyboard.onPressCallback = this.keyPress;
                this.game.input.keyboard.onDownCallback = this.keyDown;
                this.nameButton = this.game.add.button(0, 0, "button_name");
                this.colorSelectPrevButton = this.game.add.button(0, 200, "button_color", this.colorSelectPrev, this);
                this.colorSelectNextButton = this.game.add.button(280, 200, "button_color", this.colorSelectNext, this);
                this.enterGameButton = this.game.add.button(0, 460, "button_join", this.enterGame, this);
                this.nameField = this.game.add.text(this.game.width / 2, 100, (playerName == "") ? "_TYPE_NAME_" : playerName, null);
                this.nameField.anchor.set(0.5, 0.5);
                this.colorSelectText = this.game.add.text(this.game.width / 2, 306, "SELECT_COLOUR", null);
                this.colorSelectText.anchor.set(0.5, 0.5);
                this.colorPrevText = this.game.add.text(100, 300, "←", null);
                this.colorPrevText.anchor.set(0.5, 0.5);
                this.colorNextText = this.game.add.text(this.game.width - 100, 300, "→", null);
                this.colorNextText.anchor.set(0.5, 0.5);
                this.enterGameText = this.game.add.text(this.game.width / 2, 510, "ENTER_THE_GRID", null);
                this.enterGameText.anchor.set(0.5, 0.5);
                this.serverMsgText = this.game.add.text(this.game.width / 2, this.game.height, serverMsg, null);
                this.serverMsgText.anchor.set(0.5, 1);
                this.nameTypeSound = this.game.add.audio("keyboard_key");
                this.nameTypeSound.allowMultiple = true;
                this.colorSelectSound = this.game.add.audio("scifi5");
                this.colorSelectSound.allowMultiple = true;
                this.updateMenuTextColors();
            };
            MainMenu.prototype.keyPress = function (char) {
                switch (char) {
                    case " ":
                        playerName += "_";
                        break;
                    case "\n":
                    case "\r":
                        break;
                    default:
                        playerName += char;
                        break;
                }
                if (playerName.length <= this.nameMaxLength) {
                    this.nameTypeSound.play();
                }
                playerName = playerName.substring(0, this.nameMaxLength);
                this.nameField.setText(playerName);
            };
            MainMenu.prototype.keyDown = function (event) {
                switch (event.code) {
                    case "Backspace":
                        event.preventDefault();
                        if (playerName.length > 0) {
                            this.nameTypeSound.play();
                        }
                        playerName = (playerName.length > 0) ? playerName.substring(0, playerName.length - 1) : "";
                        this.nameField.setText(playerName);
                        break;
                    case "ArrowLeft":
                        this.colorSelectPrev();
                        break;
                    case "ArrowRight":
                        this.colorSelectNext();
                        break;
                    case "Enter":
                    case "Return":
                        this.enterGame();
                        break;
                }
            };
            MainMenu.prototype.colorSelectPrev = function () {
                this.colorSelectSound.play();
                playerColor = colors[(colors.indexOf(playerColor) - 1 >= 0) ? colors.indexOf(playerColor) - 1 : colors.length - 1];
                this.updateMenuTextColors();
            };
            MainMenu.prototype.colorSelectNext = function () {
                this.colorSelectSound.play();
                playerColor = colors[(colors.indexOf(playerColor) + 1 < colors.length) ? colors.indexOf(playerColor) + 1 : 0];
                this.updateMenuTextColors();
            };
            MainMenu.prototype.updateMenuTextColors = function () {
                $('#webtron canvas').css('border', '3px solid ' + colorsToHexString[playerColor]);
                this.nameField.setStyle({
                    "font": "30px " + uiFont,
                    "fill": colorsToHexString[playerColor]
                });
                this.colorSelectText.setStyle({
                    "font": "30px " + uiFont,
                    "fill": colorsToHexString[playerColor]
                });
                this.colorPrevText.setStyle({
                    "font": "50px " + uiFont,
                    "fill": colorsToHexString[colors[(colors.indexOf(playerColor) - 1 >= 0) ? colors.indexOf(playerColor) - 1 : colors.length - 1]]
                });
                this.colorNextText.setStyle({
                    "font": "50px " + uiFont,
                    "fill": colorsToHexString[colors[(colors.indexOf(playerColor) + 1 < colors.length) ? colors.indexOf(playerColor) + 1 : 0]]
                });
                this.enterGameText.setStyle({
                    "font": "30px " + uiFont,
                    "fill": colorsToHexString[playerColor]
                });
                this.serverMsgText.setStyle({
                    "font": "20px " + uiFont,
                    "fill": colorsToHexString[playerColor]
                });
            };
            MainMenu.prototype.enterGame = function () {
                playerName = (playerName == "") ? "ANON" : playerName;
                this.game.state.start("connect");
            };
            MainMenu.prototype.shutdown = function () {
                this.game.input.keyboard.callbackContext = null;
                this.game.input.keyboard.onPressCallback = null;
                this.game.input.keyboard.onDownCallback = null;
            };
            return MainMenu;
        }(Phaser.State));
        Webtron.MainMenu = MainMenu;
        var Connect = (function (_super) {
            __extends(Connect, _super);
            function Connect() {
                _super.apply(this, arguments);
            }
            Connect.prototype.create = function () {
                this.connectingText = this.game.add.text(this.game.width / 2, this.game.height / 2, "CONNECTING", null);
                this.connectingText.anchor.set(0.5, 0.5);
                this.connectingText.setStyle({
                    "font": "30px " + uiFont,
                    "fill": colorsToHexString[playerColor]
                });
                var protocol = (window.location.protocol == "https:") ? "wss:" : "ws:", hostname = window.location.hostname, port = window.location.port, path = "/ws", address = protocol + "//" + hostname + ":" + port + path;
                var state = this;
                socket = new WebSocket(address);
                socket.onerror = function (event) {
                    serverMsg = "Connection Error";
                    state.game.state.start("mainmenu");
                };
                socket.onclose = function (event) {
                    serverMsg = "Disconnected";
                    state.game.state.start("mainmenu");
                };
                socket.onopen = function (event) {
                    state.game.state.start("gamemenu");
                };
            };
            return Connect;
        }(Phaser.State));
        Webtron.Connect = Connect;
        var GameMenu = (function (_super) {
            __extends(GameMenu, _super);
            function GameMenu() {
                _super.apply(this, arguments);
            }
            GameMenu.prototype.create = function () {
                socket.onmessage = this.socketmessage;
                socket.send("LIST_GAMES");
            };
            GameMenu.prototype.socketmessage = function (event) {
                console.log(event);
            };
            return GameMenu;
        }(Phaser.State));
        Webtron.GameMenu = GameMenu;
        var InGame = (function (_super) {
            __extends(InGame, _super);
            function InGame() {
                _super.apply(this, arguments);
            }
            InGame.prototype.preload = function () {
                this.game.load.image("background", "img/gridBG.png");
                this.game.load.image("gridbike-blue", "img/gridbike-blue.png");
                this.game.load.image("gridbike-green", "img/gridbike-green.png");
                this.game.load.image("gridbike-orange", "img/gridbike-orange.png");
                this.game.load.image("gridbike-purple", "img/gridbike-purple.png");
                this.game.load.image("gridbike-red", "img/gridbike-red.png");
                this.game.load.image("gridbike-white", "img/gridbike-white.png");
            };
            InGame.prototype.shutdown = function () {
            };
            return InGame;
        }(Phaser.State));
        Webtron.InGame = InGame;
    })(Webtron = exports.Webtron || (exports.Webtron = {}));
});
//# sourceMappingURL=webtron.js.map