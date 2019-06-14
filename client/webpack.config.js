/* eslint-env node */

const webpack = require('webpack')
const { resolve } = require('path')
const TerserJSPlugin = require('terser-webpack-plugin')
const OptimizeCSSAssetsPlugin = require('optimize-css-assets-webpack-plugin')
const DirectoryNamedPlugin = require('directory-named-webpack-plugin')
const CopyPlugin = require('copy-webpack-plugin')
const MiniCssExtractPlugin = require('mini-css-extract-plugin')
const HtmlPlugin = require('html-webpack-plugin')

const absolutePath = path => resolve(__dirname, path)
const config = devMode => ({
  mode: devMode ? 'development' : 'production',
  resolve: {
    modules: [absolutePath('src'), absolutePath('node_modules')],
    plugins: [new DirectoryNamedPlugin(true)],
    alias: devMode ? { 'react-dom': '@hot-loader/react-dom' } : undefined,
  },
  output: {
    filename: devMode ? '[name].js' : '[name].[contenthash].js',
  },
  devServer: {
    contentBase: false,
    hot: true,
    inline: true,
    disableHostCheck: true,
    host: '0.0.0.0',
    port: 3000,
    stats: { all: false, errors: true, warnings: true },
    proxy: { '/ws': { target: 'http://localhost:3001', ws: true, changeOrigin: true } },
  },
  stats: { all: false, errors: true, warnings: true },
  performance: { hints: devMode ? false : 'warning' },
  devtool: devMode ? 'cheap-module-eval-source-map' : undefined,
  plugins: [
    new webpack.DefinePlugin({ 'global.devMode': devMode }),
    devMode ? new webpack.HotModuleReplacementPlugin() : false,
    new CopyPlugin([{ from: absolutePath('public') }]),
    devMode ? false : new MiniCssExtractPlugin({ filename: devMode ? '[name].css' : '[name].[contenthash].css' }),
    new HtmlPlugin({ title: 'Webtron', template: 'src/index.template.html' }),
  ].filter(Boolean),
  optimization: {
    splitChunks: { chunks: 'all' },
    minimizer: [new TerserJSPlugin(), new OptimizeCSSAssetsPlugin()],
  },
  module: {
    rules: [
      {
        test: /\.js$/,
        enforce: 'pre',
        include: absolutePath('.'),
        use: [
          {
            loader: 'prettier-loader',
            options: { parser: 'babel' },
          },
          {
            loader: 'eslint-loader',
            options: { configFile: '.eslintrc' },
          },
        ],
      },
      {
        test: /\.js$/,
        include: absolutePath('.'),
        loader: 'babel-loader',
        options: {
          presets: ['@babel/preset-react'],
          plugins: ['react-hot-loader/babel'],
          compact: !devMode,
        },
      },
      {
        test: /\.css$/,
        enforce: 'pre',
        include: absolutePath('.'),
        loader: 'prettier-loader',
        options: { parser: 'css' },
      },
      {
        test: /(?:module)\.css$/,
        include: absolutePath('.'),
        use: [
          devMode ? 'style-loader' : MiniCssExtractPlugin.loader,
          {
            loader: 'css-loader',
            options: {
              sourceMap: true,
              modules: true,
              localIdentName: '[name]-[local]-[contenthash:7]',
            },
          },
        ],
      },
      {
        test: /(?<!module)\.css$/,
        include: absolutePath('.'),
        use: [devMode ? 'style-loader' : MiniCssExtractPlugin.loader, 'css-loader?sourceMap'],
      },
    ],
  },
})

module.exports = () => config(process.env.NODE_ENV !== 'production')
