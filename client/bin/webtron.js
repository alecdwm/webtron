var __extends = (this && this.__extends) || function (d, b) {
    for (var p in b) if (b.hasOwnProperty(p)) d[p] = b[p];
    function __() { this.constructor = d; }
    d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
};
define("webtron", ["require", "exports", 'jquery'], function (require, exports, $) {
    "use strict";
    var Webtron = (function (_super) {
        __extends(Webtron, _super);
        function Webtron() {
            var width = 560, height = 560, renderer = Phaser.WEBGL, parent = "webtron", state = null, transparent = true, antialias = true;
            _super.call(this, width, height, renderer, parent, state, transparent, antialias);
            this.state.add("menu", Menu, false);
            this.state.add("game", Game, false);
            this.state.start("menu");
        }
        return Webtron;
    }(Phaser.Game));
    exports.Webtron = Webtron;
    var Menu = (function (_super) {
        __extends(Menu, _super);
        function Menu() {
            _super.apply(this, arguments);
        }
        Menu.prototype.preload = function () {
            this.colors = [
                "orange",
                "blue",
                "green",
                "purple",
                "red",
                "white"
            ];
            this.colorsToHex = {
                'blue': "#00c2cc",
                'green': "#2ee5c7",
                'orange': "#f2d91a",
                'purple': "#8a2ee5",
                'red': "#e5482e",
                'white': "#e5feff",
            };
            this.game.load.image("button_name", "img/button_name.png");
            this.game.load.image("button_color", "img/button_color.png");
            this.game.load.image("button_join", "img/button_join.png");
            this.game.load.audiosprite("scifi5", ["sfx/scifi5.mp3"]);
            this.game.load.audiosprite("keyboard_key", ["sfx/keyboard_key.mp3"]);
        };
        Menu.prototype.create = function () {
            this.name = "";
            this.nameMaxLength = 10;
            this.color = this.colors[0];
            this.game.input.keyboard.callbackContext = this;
            this.game.input.keyboard.onPressCallback = this.keyPress;
            this.game.input.keyboard.onDownCallback = this.keyDown;
            this.nameButton = this.game.add.button(0, 0, "button_name");
            this.colorSelectPrevButton = this.game.add.button(0, 200, "button_color", this.colorSelectPrev, this);
            this.colorSelectNextButton = this.game.add.button(280, 200, "button_color", this.colorSelectNext, this);
            this.joinGameButton = this.game.add.button(0, 460, "button_join", this.joinGame, this);
            this.nameField = this.game.add.text(this.game.width / 2, 100, "_TYPE_NAME_", null);
            this.nameField.anchor.set(0.5, 0.5);
            this.colorPrevText = this.game.add.text(140, 330, "←", null);
            this.colorPrevText.anchor.set(0.5, 0.5);
            this.colorNextText = this.game.add.text(this.game.width - 140, 330, "→", null);
            this.colorNextText.anchor.set(0.5, 0.5);
            this.joinGameText = this.game.add.text(this.game.width / 2, 510, "ENTER THE GRID", null);
            this.joinGameText.anchor.set(0.5, 0.5);
            this.nameTypeSound = this.game.add.audio("keyboard_key");
            this.nameTypeSound.allowMultiple = true;
            this.colorSelectSound = this.game.add.audio("scifi5");
            this.colorSelectSound.allowMultiple = true;
            this.updateMenuTextColors();
        };
        Menu.prototype.keyPress = function (char) {
            switch (char) {
                case " ":
                    this.name += "_";
                    break;
                default:
                    this.name += char;
                    break;
            }
            if (this.name.length <= this.nameMaxLength) {
                this.nameTypeSound.play();
            }
            this.name = this.name.substring(0, this.nameMaxLength);
            this.nameField.setText(this.name);
        };
        Menu.prototype.keyDown = function (event) {
            switch (event.code) {
                case "Backspace":
                    event.preventDefault();
                    if (this.name.length > 0) {
                        this.nameTypeSound.play();
                    }
                    this.name = (this.name.length > 0) ? this.name.substring(0, this.name.length - 1) : "";
                    this.nameField.setText(this.name);
                    break;
                case "ArrowLeft":
                    this.colorSelectPrev();
                    break;
                case "ArrowRight":
                    this.colorSelectNext();
                    break;
                case "Enter":
                case "Return":
                    this.joinGame();
                    break;
            }
        };
        Menu.prototype.colorSelectPrev = function () {
            this.colorSelectSound.play();
            this.color = this.colors[(this.colors.indexOf(this.color) - 1 >= 0) ? this.colors.indexOf(this.color) - 1 : this.colors.length - 1];
            this.updateMenuTextColors();
        };
        Menu.prototype.colorSelectNext = function () {
            this.colorSelectSound.play();
            this.color = this.colors[(this.colors.indexOf(this.color) + 1 < this.colors.length) ? this.colors.indexOf(this.color) + 1 : 0];
            this.updateMenuTextColors();
        };
        Menu.prototype.updateMenuTextColors = function () {
            $('#webtron canvas').css('border', '3px solid ' + this.colorsToHex[this.color]);
            this.nameField.setStyle({
                "font": "30px \"Courier New\", Courier, monospace",
                "fill": this.colorsToHex[this.color]
            });
            this.colorPrevText.setStyle({
                "font": "50px \"Courier New\", Courier, monospace",
                "fill": this.colorsToHex[this.colors[(this.colors.indexOf(this.color) - 1 >= 0) ? this.colors.indexOf(this.color) - 1 : this.colors.length - 1]]
            });
            this.colorNextText.setStyle({
                "font": "50px \"Courier New\", Courier, monospace",
                "fill": this.colorsToHex[this.colors[(this.colors.indexOf(this.color) + 1 < this.colors.length) ? this.colors.indexOf(this.color) + 1 : 0]]
            });
            this.joinGameText.setStyle({
                "font": "30px \"Courier New\", Courier, monospace",
                "fill": this.colorsToHex[this.color]
            });
        };
        Menu.prototype.joinGame = function () {
            if (this.name == "") {
                this.name = "CLU";
            }
            this.game.state.clearCurrentState();
            this.game.state.start("game");
        };
        Menu.prototype.shutdown = function () {
            this.game.input.keyboard.callbackContext = null;
            this.game.input.keyboard.onPressCallback = null;
            this.game.input.keyboard.onDownCallback = null;
        };
        return Menu;
    }(Phaser.State));
    exports.Menu = Menu;
    var Game = (function (_super) {
        __extends(Game, _super);
        function Game() {
            _super.apply(this, arguments);
        }
        Game.prototype.preload = function () {
            this.game.load.image("background", "img/gridBG.png");
            this.game.load.image("gridbike-blue", "img/gridbike-blue.png");
            this.game.load.image("gridbike-green", "img/gridbike-green.png");
            this.game.load.image("gridbike-orange", "img/gridbike-orange.png");
            this.game.load.image("gridbike-purple", "img/gridbike-purple.png");
            this.game.load.image("gridbike-red", "img/gridbike-red.png");
            this.game.load.image("gridbike-white", "img/gridbike-white.png");
        };
        Game.prototype.create = function () {
            this.game.add.image(0, 0, "background");
        };
        Game.prototype.shutdown = function () {
        };
        return Game;
    }(Phaser.State));
    exports.Game = Game;
});
//# sourceMappingURL=webtron.js.map