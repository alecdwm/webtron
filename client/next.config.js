const withPlugins = require('next-compose-plugins')
const { DefinePlugin } = require('webpack')
const { resolve } = require('path')

const withPrettier = (nextConfig = {}) => ({
  ...nextConfig,
  webpack(config, options) {
    if (!options.isServer) {
      const { prettierLoaderOptions } = config
      options.defaultLoaders.prettier = {
        loader: 'prettier-loader',
        options: { parser: 'babel', prettierLoaderOptions },
      }

      config.module.rules.push({
        enforce: 'pre',
        test: /\.jsx?$/,
        exclude: /node_modules/,
        use: options.defaultLoaders.prettier,
      })
    }

    if (typeof nextConfig.webpack === 'function') return nextConfig.webpack(config, options)
    return config
  },
})

const withEslint = (nextConfig = {}) => ({
  ...nextConfig,
  webpack(config, options) {
    if (!options.isServer) {
      const { eslintLoaderOptions } = config
      options.defaultLoaders.eslint = {
        loader: 'eslint-loader',
        options: { failOnError: true, ...eslintLoaderOptions },
      }

      config.module.rules.push({
        enforce: 'pre',
        test: /\.jsx?$/,
        exclude: /node_modules/,
        use: options.defaultLoaders.eslint,
      })
    }

    if (typeof nextConfig.webpack === 'function') return nextConfig.webpack(config, options)
    return config
  },
})

const withUrlLoader = (nextConfig = {}) => ({
  ...nextConfig,
  webpack(config, options) {
    options.defaultLoaders.url = {
      loader: 'url-loader',
    }

    config.module.rules.push({
      test: /\.(svg|png)$/,
      exclude: /node_modules/,
      use: options.defaultLoaders.url,
    })

    if (typeof nextConfig.webpack === 'function') return nextConfig.webpack(config, options)
    return config
  },
})

const withDevModeGlobal = (nextConfig = {}) => ({
  ...nextConfig,
  webpack(config, options) {
    config.plugins.push(new DefinePlugin({ 'global.devMode': process.env.NODE_ENV !== 'production' }))

    if (typeof nextConfig.webpack === 'function') return nextConfig.webpack(config, options)
    return config
  },
})

const withAbsoluteWebpackImports = (nextConfig = {}) => ({
  ...nextConfig,
  webpack(config, options) {
    config.resolve.modules.push('src')

    if (typeof nextConfig.webpack === 'function') return nextConfig.webpack(config, options)
    return config
  },
})

const withNoXPoweredByHeader = (config = {}) => ({ ...config, poweredByHeader: false })

const withNoAutomaticStaticOptimizationIndicator = (config = {}) => ({
  ...config,
  devIndicators: {
    autoPrerender: false,
  },
})

module.exports = withPlugins([
  withPrettier,
  withEslint,
  withUrlLoader,
  withDevModeGlobal,
  withAbsoluteWebpackImports,
  withNoXPoweredByHeader,
  withNoAutomaticStaticOptimizationIndicator,
])
