// Package plan is a deliberately dirty fixture: every census kind the
// extractor must catch, in one small cell.
//
//spec:scope spec://demo/PROP-001#cells r=1
package plan

import (
	"fmt"
	"os"
	"strings"
	"time"

	_ "net/http/pprof"
)

// PlanError lacks a Spec field on purpose (seam_error_missing_req).
type PlanError struct {
	Code int
	Err  error
}

func (e *PlanError) Error() string { return fmt.Sprintf("plan: %d", e.Code) }

func init() { // init_decl
	fmt.Println("registering")
}

// Solve carries an implements marker.
//
//spec:implements spec://demo/PROP-001#req-solve r=2
func Solve() error {
	now := time.Now()          // ambient_call
	home := os.Getenv("HOME")  // ambient_call
	go func() { _ = home }()   // naked_go
	err := fmt.Errorf("x %v", now)
	if err.Error() == "boom" { // error_string_match
		return err
	}
	if strings.Contains(err.Error(), "boom") { // error_string_match
		return err
	}
	//nolint
	return nil
}

// Sanctioned reads the clock under recorded testimony — the census
// site inside must carry the reason, not a finding.
//
//spec:deviates spec://demo/PROP-001#cells r=1 reason="wall clock IS the domain here"
func Sanctioned() time.Time {
	return time.Now()
}
