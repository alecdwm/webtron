require.config({
	paths: {
		webtron: "/bin/webtron",
		jquery: "/src/lib/jquery.min",
		// phaser: "/src/lib/phaser.min", // has issues with requirejs
	}
})

require(['jquery', 'webtron'], function(jquery, webtron) {
	var $ = require('jquery')
	var webtron = require('webtron')

	$(document).ready(function() {
		new webtron.Webtron()
	})
})
