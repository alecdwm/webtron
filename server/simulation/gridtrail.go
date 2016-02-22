package simulation

import (
	"math"

	"github.com/ungerik/go3d/float64/vec2"
)

//// GridTrail /////////////////////////////////////////////////////////////////

// GridTrailState is an enum type for the possible states a gridtrail can have
type GridTrailState string

// GridTrailStates
const (
	Active   GridTrailState = "active"
	Inactive GridTrailState = "inactive"
)

// GridTrail is an impassible wall of grid... stuff
type GridTrail struct {
	state  GridTrailState
	colour string

	origin vec2.T
	verts  []vec2.T
	end    vec2.T
}

//// Callbacks /////////////////////////////////////////////////////////////////

// OnTrail checks if a point is colliding with this trail
func (gt *GridTrail) OnTrail(pos vec2.T) bool {
	verts := []vec2.T{gt.origin}
	verts = append(verts, gt.verts...)
	verts = append(verts, gt.end)
	for i := range verts {
		if i == 0 {
			continue
		}

		if gt.pointBetweenVerts(pos, verts[i-1], verts[i]) {
			return true
		}
	}
	return false
}

//// Internal Functions ////////////////////////////////////////////////////////

// pointBetweenVerts returns true if point is between two verticies
func (gt *GridTrail) pointBetweenVerts(pt, lst, lnd vec2.T) bool {
	threshold := 1.0

	if lst[0] != lnd[0] {
		if lst[0] < lnd[0] {
			return math.Abs(pt[1]-lst[1]) < threshold && lst[0] < pt[0] && pt[0] < lnd[0]
		}
		return math.Abs(pt[1]-lst[1]) < threshold && lnd[0] < pt[0] && pt[0] < lst[0]
	}
	if lst[1] != lnd[1] {
		if lst[1] < lnd[1] {
			return math.Abs(pt[0]-lst[0]) < threshold && lst[1] < pt[1] && pt[1] < lnd[1]
		}
		return math.Abs(pt[0]-lst[0]) < threshold && lnd[1] < pt[1] && pt[1] < lst[1]
	}
	// bike probably just turned, since lst == lnd
	return false
}
