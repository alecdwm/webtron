export type ClassName = string | false | null | undefined | ClassName[]

// Used to resolve className arrays into className strings.
// Falsy values inside the className array will be filtered out.
// Some usage examples:
//
//   resolveClassName(['one', 'two', 'three']) -> 'one two three'
//
//   resolveClassName('four') -> 'four'
//
//   const portraitUI = false
//   const errorUI = true
//   resolveClassName([
//     'item-wrapper'
//     portraitUI && 'portrait-ui'
//     errorUI && 'error-ui'
//   ]) -> 'item-wrapper error-ui'
//
export default function resolveClassName(className: ClassName): string | undefined {
  if (!Array.isArray(className)) return className || undefined
  return className.map(resolveClassName).filter(Boolean).join(' ') || undefined
}
