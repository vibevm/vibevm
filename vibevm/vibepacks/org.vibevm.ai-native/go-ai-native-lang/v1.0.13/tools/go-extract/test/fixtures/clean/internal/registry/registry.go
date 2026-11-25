// Package registry is the composition root — the one legal importer of
// cell packages (GUIDE-AI-NATIVE-GO §6). The green exhibit for
// go-flag-sites: a cell imported here is exactly where a selection flag
// becomes a cell, so this import stays silent.
//
//spec:scope spec://demo/PROP-001#cells r=1
package registry

import "example.com/demo/internal/cells/greet"

// SelectGreet wires the greet cell through its Greeting seam — the
// registry switch is the system's table of contents.
func SelectGreet() greet.Greeting { return nil }
