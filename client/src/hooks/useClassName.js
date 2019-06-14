import React, { useEffect, useCallback, useRef, forwardRef } from 'react'
import useForceUpdate from 'hooks/useForceUpdate'

// Usage examples:
//
//   const BoldText = useClassName('bold', 'span')
//   return <BoldText>This text can be styled boldly!</BoldText>
//
//   const BoldText = useClassName('bold', 'span')
//   return <BoldText className="paragraph">This text can _still_ be styled boldly!</BoldText>
//
//   const ArticleContainer = useClassName('article-container')
//   const Title = useClassName('article-title')
//   const Body = useClassName('article-body')
//   return (
//     <ArticleContainer>
//       <Title>Lorem Ipsum</Title>
//       <Body>
//         lorem ipsum dolar sit amet...
//       </Body>
//     </Container>
//   )
//
export default function useClassName(staticClassName, WrappedComponent = 'div') {
  // Resolve staticClassName to a string.
  // We will be doing an equality check against this string later on.
  // If we don't resolve it, and it therefore possibly remains as an array,
  // that equality check will always fail and we will be wasting render cycles.
  staticClassName = resolveClassName(staticClassName)

  // Store staticClassName in a ref so we don't need to remount the
  // WithClassName wrapper component when the staticClassName changes.
  const staticClassNameRef = useRef(staticClassName)

  // Store forceUpdate in a ref so we can use it outside of the
  // WithClassName wrapper component.
  const forceUpdateRef = useRef()

  // useCallback memoizes the WithClassName component using the `WrappedComponent` variable.
  // (if WrappedComponent doesn't change, the callback function is only called once).
  //
  // This stops React from replacing the entire DOM structure below and including
  // the WithClassName wrapper component when the WrappedComponent hasn't changed.
  const WithClassName = useCallback(
    // eslint-disable-next-line react/display-name
    forwardRef(
      // eslint-disable-next-line react/prop-types
      function WithClassName({ className: propsClassName, ...props }, ref) {
        const forceUpdate = useForceUpdate()

        // Provide the forceUpdate function to the useClassName effect hook...
        forceUpdateRef.current = forceUpdate

        // ...but remove it when we unmount this component, so that the useEffect
        // hook cannot call setState on an unmounted component.
        useEffect(
          () => () => {
            // Don't do anything if the forceUpdateRef.current value has changed
            // since we set it.
            if (forceUpdateRef.current !== forceUpdate) return

            forceUpdateRef.current = undefined
          },
          [forceUpdate],
        )

        // Resolve propsClassName to a string.
        propsClassName = resolveClassName(propsClassName)

        // Join staticClassName with the value passed to WithClassName's className prop.
        // Allows for use-cases such as:
        //
        //   const Item = useClassName('item')
        //   return <Item className="blue">A blue item!</Item>
        //
        const className = resolveClassName([staticClassNameRef.current, propsClassName])

        return React.createElement(WrappedComponent, { ...props, className, ref })
      },
    ),
    [WrappedComponent],
  )

  // When staticClassName changes, update the value stored inside the staticClassNameRef
  // variable, then trigger a forceUpdate in the wrapper component (if available).
  useEffect(() => {
    staticClassNameRef.current = staticClassName
    forceUpdateRef.current && forceUpdateRef.current()
  }, [staticClassName])

  return WithClassName
}

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
function resolveClassName(className) {
  if (!Array.isArray(className)) return className
  return className.filter(Boolean).join(' ')
}
