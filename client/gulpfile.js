var gulp = require('gulp')
var ts = require('gulp-typescript')
var sourcemaps = require('gulp-sourcemaps')
var uglify = require('gulp-uglify')

gulp.task('default', function () {
	return gulp.src(['src/**/*.ts', '!src/**/*.d.ts'])
		.pipe(sourcemaps.init())
		.pipe(ts({
			typescript: require('typescript'),
			sortOutput: true,
			noImplicitAny: true,
			removeComments: true,
			//module: 'amd',
			target: 'ES5',
			outFile: 'webtron.js',
		}))
		.pipe(uglify())
		.pipe(sourcemaps.write())
		.pipe(gulp.dest('bin'))
})
