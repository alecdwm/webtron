const { whenDev } = require('@craco/craco')
const { DefinePlugin } = require('webpack')
const createWebpackOverridePlugin = (overrideWebpackConfig) => ({ plugin: { overrideWebpackConfig } })

const PrettierLoaderPlugin = createWebpackOverridePlugin(({ webpackConfig }) => {
  webpackConfig.module.rules.push({
    test: /\.(jsx?|css)$/,
    exclude: /node_modules/,
    use: 'prettier-loader',
  })
  return webpackConfig
})

const AbsoluteImportsPlugin = createWebpackOverridePlugin(({ webpackConfig }) => {
  webpackConfig.resolve.modules.push('src')
  return webpackConfig
})

const DevModeGlobalPlugin = createWebpackOverridePlugin(({ webpackConfig }) => {
  webpackConfig.plugins.push(new DefinePlugin({ 'global.devMode': whenDev(() => true, false) }))
  return webpackConfig
})

module.exports = {
  plugins: [PrettierLoaderPlugin, AbsoluteImportsPlugin, DevModeGlobalPlugin],
}
