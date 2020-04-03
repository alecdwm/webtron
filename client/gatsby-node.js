const PrettierPlugin = require('prettier-webpack-plugin')

exports.onCreateWebpackConfig = ({ actions }) => {
  actions.setWebpackConfig({
    resolve: {
      modules: ['src', 'node_modules'],
    },
    plugins: [new PrettierPlugin()],
  })
}
