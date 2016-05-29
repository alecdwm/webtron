var gulp       = require('gulp')
var cssmin     = require('gulp-cssmin')
var sass       = require('gulp-sass')
var sourcemaps = require('gulp-sourcemaps')
var ts         = require('gulp-typescript')
var uglify     = require('gulp-uglify')

gulp.task('sass', function() {
	return gulp.src('sass/webtron.s+(a|c)ss')
		.pipe(sourcemaps.init())
		.pipe(sass({errLogToConsole: true}))
		.pipe(cssmin())
		.pipe(sourcemaps.write())
		.pipe(gulp.dest('bin'))
})

gulp.task('typescript', function() {
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

gulp.task('watch', function() {
	gulp.watch('sass/**/*.s+(a|c)ss', ['sass'])
	gulp.watch('src/**/*.ts', ['typescript'])
})

gulp.task('default', ['sass', 'typescript'])
