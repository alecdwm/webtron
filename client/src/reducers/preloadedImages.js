import { PRELOAD_IMAGES } from 'actions'
import createReducer from 'utils/createReducer'

const initialState = []

export default createReducer(initialState, {
  [PRELOAD_IMAGES]: (store, action) =>
    action.urls.map(url => {
      const image = new Image()
      image.src = url
      return image
    }),
})
