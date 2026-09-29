// Package greet is the clean fixture: a well-formed cell under the
// discipline — injected capabilities, a closed error set with its REQ,
// no census sites.
//
//spec:scope spec://demo/PROP-001#cells r=1
package greet

import "fmt"

// clock is the injected capability (consumer-side narrow interface).
type clock interface {
	Unix() int64
}

// GreetError is the seam's closed failure set.
type GreetError struct {
	Code int
	Spec string
	Err  error
}

func (e *GreetError) Error() string {
	return fmt.Sprintf("greet: %d: violates REQ %s", e.Code, e.Spec)
}

func (e *GreetError) Unwrap() error { return e.Err }

// Greeting is the seam this cell satisfies.
type Greeting interface {
	Greet(name string) string
}

// Greeter greets deterministically off its injected clock.
type Greeter struct {
	clk clock
}

// Conformance is made loud: Greeter satisfies the Greeting seam.
var _ Greeting = (*Greeter)(nil)

// FormalGreeting is the `formal` Greeting cell — its name is the
// computed one (Pascal("formal") + seam "Greeting" = "FormalGreeting"),
// the clean exhibit for cell-name-is-computed (B-038).
//
//spec:cell seam=Greeting variant=formal
type FormalGreeting struct{}

// New is the blessed construction path.
//
//spec:implements spec://demo/PROP-001#req-greet r=1
func New(clk clock) *Greeter { return &Greeter{clk: clk} }

// Greet renders a stamped greeting.
func (g *Greeter) Greet(name string) string {
	return fmt.Sprintf("hello %s @%d", name, g.clk.Unix())
}
