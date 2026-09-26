import { Component, type ReactNode } from 'react'

type ErrorBoundaryProps = {
  children: ReactNode
  // Called once the children have thrown while rendering. Nothing is shown in
  // their place until the boundary is remounted.
  onError: () => void
}

export default class ErrorBoundary extends Component<ErrorBoundaryProps, { failed: boolean }> {
  state = { failed: false }

  static getDerivedStateFromError() {
    return { failed: true }
  }

  componentDidCatch() {
    this.props.onError()
  }

  render() {
    return this.state.failed ? null : this.props.children
  }
}
