export default function createSimpleAction(type, ...argNames) {
  return function (...args) {
    const action = { type }
    argNames.forEach((argName, index) => {
      action[argName] = args[index]
    })
    return action
  }
}
