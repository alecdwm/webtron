const { DefinePlugin } = require('webpack')
const process = require('process')
const PrettierPlugin = require('prettier-webpack-plugin')

exports.onCreateWebpackConfig = ({ actions }) => {
  actions.setWebpackConfig({
    resolve: {
      modules: ['src', 'node_modules'],
    },
    plugins: [new PrettierPlugin(), new DefinePlugin({ 'global.devMode': process.env.NODE_ENV !== 'production' })],
  })
}
