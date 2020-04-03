const { createProxyMiddleware } = require('http-proxy-middleware')

module.exports = {
  plugins: ['gatsby-plugin-eslint'],
  developMiddleware: (app) => {
    app.use(
      '/ws',
      createProxyMiddleware({
        target: 'http://localhost:3001',
        changeOrigin: true,
        ws: true,
        xfwd: true,
      }),
    )
  },
}
