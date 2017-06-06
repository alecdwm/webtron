import webpack from 'webpack'
import path from 'path'
import CopyWebpackPlugin from 'copy-webpack-plugin'
import ExtractTextPlugin from 'extract-text-webpack-plugin'

const config = {
  entry: path.join(__dirname, 'client/index.js'),
  resolve: {
    modules: [
      path.resolve(__dirname, 'client'),
      path.resolve(__dirname, 'node_modules'),
    ],
    extensions: ['.js', 'json'],
  },
  devServer:{
    contentBase: 'client/public',
    hot: true,
    inline: true,
    port: 3000,
  },
  devtool: 'cheap-module-source-map',
  output: {
    path: path.resolve(__dirname, 'client/bin'),
    filename: 'webtron.js',
  },
  plugins: [
    new webpack.DefinePlugin({
      'process.env': {
        NODE_ENV: JSON.stringify(process.env.NODE_ENV || 'development'),
      },
    }),
    new webpack.HotModuleReplacementPlugin(),
    new CopyWebpackPlugin([
      { from: path.resolve(__dirname, 'client/public') },
    ]),
    new ExtractTextPlugin('webtron.css'),
  ],
  module: {
    rules: [
      {
        test: /\.js$/,
        enforce: 'pre',
        loader: 'eslint-loader',
        include: [
          path.resolve(__dirname, 'client'),
        ],
        options: {
          configFile: '.eslintrc',
        },
      },
      {
        test: /\.(css|s(a|c)ss)$/,
        use: ExtractTextPlugin.extract({
          fallback: 'style-loader',
          use: [
            'css-loader?sourceMap&importLoaders=1',
            'sass-loader?sourceMap',
          ],
        }),
      },
      {
        test: /\.js$/,
        loader: 'babel-loader',
        include: [
          path.resolve(__dirname, 'client'),
        ],
        options: {
          presets: ['es2015', 'stage-0'],
        },
      },
    ],
  },
}

export default config
