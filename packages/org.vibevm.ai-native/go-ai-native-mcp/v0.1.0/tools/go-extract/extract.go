// go-extract — the stdlib-only fact extractor for the AI-Native Go
// discipline (conform-frontend-go brief §2; GO-AI-NATIVE-PLAN D4).
//
// Parse .go files with go/parser and emit one NDJSON record per file:
// conform facts (item / import / go_unsafe / file_metrics) plus the
// //spec: directive markers, protocol 1. Stdlib-only BY CONSTRUCTION —
// `go run extract.go` must work with no module context, no go.mod, no
// network, on any machine the floor already requires go on.
//
// B5 (monotone utility): an unparseable file degrades to a
// metrics-only record with degraded=true; it never kills the run.
//
// Delivered embedded in go-ai-native-extract-bridge (include_str!) and
// materialised content-addressed under target/conform/go-extract/.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
)

const protocol = 1

// record is one file's extraction — the NDJSON line shape the bridge's
// replay tests freeze from both ends.
type record struct {
	Protocol int      `json:"protocol"`
	File     string   `json:"file"`
	InTest   bool     `json:"in_test"`
	Degraded bool     `json:"degraded"`
	Facts    []fact   `json:"facts"`
	Markers  []marker `json:"markers"`
}

// fact mirrors the engine's serde-tagged vocabulary ("fact" tag,
// snake_case kinds).
type fact struct {
	Fact string `json:"fact"`
	// item
	Kind          string `json:"kind,omitempty"`
	Symbol        string `json:"symbol,omitempty"`
	IsExported    *bool  `json:"is_exported,omitempty"`
	HasDocExample *bool  `json:"has_doc_example,omitempty"`
	// item kind=type only: the underlying type's rendering when it is
	// a defined type over a primitive (`type AccountID string` →
	// "string") — the Go brand signal the oracle's scope answers use.
	Underlying string `json:"underlying,omitempty"`
	// import
	ToPath string `json:"to_path,omitempty"`
	// go_unsafe reuses Kind; Reason carries deviation testimony
	Reason *string `json:"reason,omitempty"`
	// file_metrics
	Lines uint32 `json:"lines,omitempty"`
	// shared
	Line uint32 `json:"line,omitempty"`
	// go_conformance: the seam interface and implementing type of a
	// `var _ <seam> = (*<Impl>)(nil)` loud-conformance assertion.
	Seam string `json:"seam,omitempty"`
	Impl string `json:"impl,omitempty"`
	// invariant_comment: the canonical invariant marker a comment leads
	// with (`INVARIANT:` / `WARNING:` / `PANICS:` / …), normalised to the
	// config dictionary's spelling. Feeds invariant-comment-position.
	Marker string `json:"marker,omitempty"`
}

// marker is one //spec: directive (GUIDE-AI-NATIVE-GO §8).
type marker struct {
	Tag    string  `json:"tag"`
	URI    string  `json:"uri"`
	R      *uint32 `json:"r"`
	Reason *string `json:"reason"`
	Symbol *string `json:"symbol"`
	Line   uint32  `json:"line"`
}

var skipDirs = map[string]bool{
	"vendor": true, "testdata": true, "node_modules": true,
	".git": true, "vibedeps": true, "target": true,
}

func main() {
	root := flag.String("root", ".", "project root")
	filesGiven := flag.Bool("files", false,
		"positional args are the file list; with zero args, extract nothing (probe)")
	stdinFile := flag.String("stdin-file", "",
		"extract exactly this repo-relative file, its content read from stdin (the overlay form)")
	flag.Parse()
	files := flag.Args()

	out := json.NewEncoder(os.Stdout)
	if *stdinFile != "" {
		src, err := io.ReadAll(os.Stdin)
		if err != nil {
			fmt.Fprintf(os.Stderr, "go-extract: reading stdin: %v\n", err)
			os.Exit(1)
		}
		rec := extractSource(filepath.ToSlash(*stdinFile), src)
		if err := out.Encode(rec); err != nil {
			fmt.Fprintf(os.Stderr, "go-extract: encoding %s: %v\n", *stdinFile, err)
			os.Exit(1)
		}
		return
	}

	if !*filesGiven && len(files) == 0 {
		walked, err := walkTree(*root)
		if err != nil {
			fmt.Fprintf(os.Stderr, "go-extract: walking %s: %v\n", *root, err)
			os.Exit(1)
		}
		files = walked
	}
	sort.Strings(files)

	for _, rel := range files {
		rec := extractFile(*root, filepath.ToSlash(rel))
		if err := out.Encode(rec); err != nil {
			fmt.Fprintf(os.Stderr, "go-extract: encoding %s: %v\n", rel, err)
			os.Exit(1)
		}
	}
}

func walkTree(root string) ([]string, error) {
	var files []string
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return nil // unreadable entries degrade, never abort (B5)
		}
		name := d.Name()
		if d.IsDir() {
			if path != root && (skipDirs[name] || strings.HasPrefix(name, ".")) {
				return filepath.SkipDir
			}
			return nil
		}
		if !strings.HasSuffix(name, ".go") {
			return nil
		}
		rel, relErr := filepath.Rel(root, path)
		if relErr != nil {
			return nil
		}
		files = append(files, filepath.ToSlash(rel))
		return nil
	})
	return files, err
}

func extractFile(root, rel string) record {
	src, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(rel)))
	if err != nil {
		return record{
			Protocol: protocol,
			File:     rel,
			InTest:   strings.HasSuffix(rel, "_test.go"),
			Degraded: true,
			Facts:    []fact{},
			Markers:  []marker{},
		}
	}
	return extractSource(rel, src)
}

func extractSource(rel string, src []byte) record {
	rec := record{
		Protocol: protocol,
		File:     rel,
		InTest:   strings.HasSuffix(rel, "_test.go"),
		Facts:    []fact{},
		Markers:  []marker{},
	}
	rec.Facts = append(rec.Facts, fact{Fact: "file_metrics", Lines: physicalLines(src)})

	fset := token.NewFileSet()
	parsed, perr := parser.ParseFile(fset, rel, src, parser.ParseComments)
	if parsed == nil {
		rec.Degraded = true
		return rec
	}
	if perr != nil {
		// Partial AST: keep going with what parsed, but say so.
		rec.Degraded = true
	}

	ex := extractor{fset: fset, file: parsed, inTest: rec.InTest}
	ex.run()
	rec.Facts = append(rec.Facts, ex.facts...)
	rec.Markers = append(rec.Markers, ex.markers...)
	return rec
}

func physicalLines(src []byte) uint32 {
	if len(src) == 0 {
		return 0
	}
	n := strings.Count(string(src), "\n")
	if src[len(src)-1] != '\n' {
		n++
	}
	return uint32(n) // #nosec: line counts fit u32
}

// extractor walks one parsed file and accumulates facts + markers.
type extractor struct {
	fset    *token.FileSet
	file    *ast.File
	inTest  bool
	facts   []fact
	markers []marker
	// deviations: line ranges covered by a reasoned //spec:deviates,
	// so census sites inside them carry the testimony.
	deviations []devRange
}

type devRange struct {
	from, to uint32
	reason   string
}

func (ex *extractor) line(pos token.Pos) uint32 {
	return uint32(ex.fset.Position(pos).Line) // #nosec: fits u32
}

func (ex *extractor) run() {
	pkgNames := ex.importedPackages()
	docOwners := ex.docOwners()

	// Markers first (deviation ranges feed the census below).
	ex.collectMarkers(docOwners)

	// Declarations: items, init(), blank imports, seam error types.
	errMethods := ex.errorMethods()
	for _, decl := range ex.file.Decls {
		switch d := decl.(type) {
		case *ast.FuncDecl:
			ex.funcItem(d)
		case *ast.GenDecl:
			ex.genItems(d, errMethods)
		}
	}

	// Expression-level census: ambient calls, naked go, error-string
	// matching, t.Skip.
	ast.Inspect(ex.file, func(n ast.Node) bool {
		switch node := n.(type) {
		case *ast.GoStmt:
			ex.unsafeAt("naked_go", ex.line(node.Pos()))
		case *ast.SelectorExpr:
			ex.ambient(node, pkgNames)
		case *ast.BinaryExpr:
			ex.errorStringCompare(node)
		case *ast.CallExpr:
			ex.stringsOnError(node, pkgNames)
			ex.testSkip(node)
		}
		return true
	})

	// Suppression hygiene: every comment line, whole file.
	ex.suppressions()

	// Invariant-marker comments: the same Comments walk, emitting a
	// fact per comment whose lead carries an invariant marker.
	ex.invariantComments()
}

// importedPackages maps the local name each import binds to its path.
func (ex *extractor) importedPackages() map[string]string {
	out := map[string]string{}
	for _, imp := range ex.file.Imports {
		path, _ := strconv.Unquote(imp.Path.Value)
		ex.facts = append(ex.facts, fact{
			Fact: "import", ToPath: path, Line: ex.line(imp.Pos()),
		})
		name := ""
		if imp.Name != nil {
			name = imp.Name.Name
		} else if i := strings.LastIndex(path, "/"); i >= 0 {
			name = path[i+1:]
		} else {
			name = path
		}
		if name == "_" {
			ex.unsafeAt("blank_import", ex.line(imp.Pos()))
			continue
		}
		out[name] = path
	}
	return out
}

// docOwners maps a doc CommentGroup to the name it documents, so a
// //spec: directive in a doc comment attaches to its declaration.
func (ex *extractor) docOwners() map[*ast.CommentGroup]string {
	out := map[*ast.CommentGroup]string{}
	for _, decl := range ex.file.Decls {
		switch d := decl.(type) {
		case *ast.FuncDecl:
			if d.Doc != nil {
				out[d.Doc] = d.Name.Name
			}
		case *ast.GenDecl:
			if d.Doc != nil {
				if name := firstSpecName(d); name != "" {
					out[d.Doc] = name
				}
			}
			for _, spec := range d.Specs {
				switch s := spec.(type) {
				case *ast.TypeSpec:
					if s.Doc != nil {
						out[s.Doc] = s.Name.Name
					}
				case *ast.ValueSpec:
					if s.Doc != nil && len(s.Names) > 0 {
						out[s.Doc] = s.Names[0].Name
					}
				}
			}
		}
	}
	return out
}

func firstSpecName(d *ast.GenDecl) string {
	for _, spec := range d.Specs {
		switch s := spec.(type) {
		case *ast.TypeSpec:
			return s.Name.Name
		case *ast.ValueSpec:
			if len(s.Names) > 0 {
				return s.Names[0].Name
			}
		}
	}
	return ""
}

var markerTags = map[string]bool{
	"implements": true, "verifies": true, "documents": true,
	"deviates": true, "informs": true, "scope": true,
}

func (ex *extractor) collectMarkers(docOwners map[*ast.CommentGroup]string) {
	declEnd := ex.declRanges()
	for _, group := range ex.file.Comments {
		owner, hasOwner := docOwners[group]
		for _, c := range group.List {
			text := c.Text
			rest, ok := strings.CutPrefix(text, "//spec:")
			if !ok {
				continue
			}
			tag, args, _ := strings.Cut(rest, " ")
			if !markerTags[tag] {
				continue // //spec:cell and future tags: not markers yet
			}
			m := marker{Tag: tag, Line: ex.line(c.Pos())}
			if hasOwner {
				o := owner
				m.Symbol = &o
			}
			fields := strings.Fields(args)
			if len(fields) > 0 {
				m.URI = fields[0]
			}
			for _, f := range fields[1:] {
				if after, isRev := strings.CutPrefix(f, "r="); isRev {
					if n, err := strconv.ParseUint(after, 10, 32); err == nil {
						r := uint32(n)
						m.R = &r
					}
				}
			}
			if idx := strings.Index(args, `reason="`); idx >= 0 {
				tail := args[idx+len(`reason="`):]
				if end := strings.Index(tail, `"`); end >= 0 {
					reason := tail[:end]
					m.Reason = &reason
				}
			}
			ex.markers = append(ex.markers, m)
			if tag == "deviates" && m.Reason != nil && hasOwner {
				if r, found := declEnd[owner]; found {
					ex.deviations = append(ex.deviations, devRange{
						from: r[0], to: r[1], reason: *m.Reason,
					})
				}
			}
		}
	}
}

// declRanges maps each declared name to its [start, end] line range —
// the span a deviates directive on the declaration covers.
func (ex *extractor) declRanges() map[string][2]uint32 {
	out := map[string][2]uint32{}
	for _, decl := range ex.file.Decls {
		switch d := decl.(type) {
		case *ast.FuncDecl:
			out[d.Name.Name] = [2]uint32{ex.line(d.Pos()), ex.line(d.End())}
		case *ast.GenDecl:
			if name := firstSpecName(d); name != "" {
				out[name] = [2]uint32{ex.line(d.Pos()), ex.line(d.End())}
			}
		}
	}
	return out
}

func (ex *extractor) unsafeAt(kind string, line uint32) {
	f := fact{Fact: "go_unsafe", Kind: kind, Line: line}
	for _, d := range ex.deviations {
		if line >= d.from && line <= d.to {
			reason := d.reason
			f.Reason = &reason
			break
		}
	}
	ex.facts = append(ex.facts, f)
}

func (ex *extractor) funcItem(d *ast.FuncDecl) {
	kind := "func"
	if d.Recv != nil {
		kind = "method"
	}
	exported := ast.IsExported(d.Name.Name)
	noExample := false
	ex.facts = append(ex.facts, fact{
		Fact: "item", Kind: kind, Symbol: d.Name.Name,
		Line: ex.line(d.Pos()), IsExported: &exported,
		// Example coverage is package-level (Example funcs live in
		// sibling _test.go files); the collector joins them by name.
		HasDocExample: &noExample,
	})
	if d.Name.Name == "init" && d.Recv == nil {
		ex.unsafeAt("init_decl", ex.line(d.Pos()))
	}
}

func (ex *extractor) genItems(d *ast.GenDecl, errOwners map[string]*ast.FuncDecl) {
	for _, spec := range d.Specs {
		switch s := spec.(type) {
		case *ast.TypeSpec:
			exported := ast.IsExported(s.Name.Name)
			noExample := false
			ex.facts = append(ex.facts, fact{
				Fact: "item", Kind: "type", Symbol: s.Name.Name,
				Line: ex.line(s.Pos()), IsExported: &exported,
				HasDocExample: &noExample,
				Underlying:    primitiveUnderlying(s),
			})
			ex.seamErrorShape(s, errOwners)
			ex.seamErrorMessage(s, errOwners)
		case *ast.ValueSpec:
			ex.conformanceAssertion(s)
			kind := "var"
			if d.Tok == token.CONST {
				kind = "const"
			}
			for _, name := range s.Names {
				if name.Name == "_" {
					continue // conformance assertions et al.
				}
				exported := ast.IsExported(name.Name)
				noExample := false
				ex.facts = append(ex.facts, fact{
					Fact: "item", Kind: kind, Symbol: name.Name,
					Line: ex.line(name.Pos()), IsExported: &exported,
					HasDocExample: &noExample,
				})
			}
		}
	}
}

// primitiveUnderlying renders a defined type's underlying primitive
// (`type AccountID string` → "string"); alias declarations and
// non-primitive underlyings yield "".
func primitiveUnderlying(s *ast.TypeSpec) string {
	if s.Assign.IsValid() {
		return "" // an alias is the same type, not a brand
	}
	ident, ok := s.Type.(*ast.Ident)
	if !ok {
		return ""
	}
	switch ident.Name {
	case "string", "bool",
		"int", "int8", "int16", "int32", "int64",
		"uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
		"float32", "float64", "byte", "rune":
		return ident.Name
	}
	return ""
}

// conformanceAssertion recognises the loud-conformance idiom
// `var _ <seam> = (*<Impl>)(nil)` (GUIDE-AI-NATIVE-GO §2) in a ValueSpec
// and emits a go_conformance record carrying the seam interface name and
// the implementing type. The match requires the exact `= (*Ident)(nil)`
// RHS, so the codemod near-misses `var _ = New` (no typed seam, RHS an
// ident) and `var _ *Type` (no values) do not match.
func (ex *extractor) conformanceAssertion(s *ast.ValueSpec) {
	if !isBlankVar(s) || len(s.Values) != 1 {
		return
	}
	impl := nilCastImpl(s.Values[0])
	if impl == "" {
		return
	}
	seam := seamName(s.Type)
	if seam == "" {
		return
	}
	ex.facts = append(ex.facts, fact{
		Fact: "go_conformance",
		Seam: seam,
		Impl: impl,
		Line: ex.line(s.Pos()),
	})
}

// isBlankVar reports whether a ValueSpec binds at least one blank name
// (`var _ ...`) — the assertion's left-hand side.
func isBlankVar(s *ast.ValueSpec) bool {
	for _, name := range s.Names {
		if name.Name == "_" {
			return true
		}
	}
	return false
}

// nilCastImpl renders the impl type name of a `(*Impl)(nil)` conversion,
// or "" when the expression is not that exact shape.
func nilCastImpl(e ast.Expr) string {
	call, ok := e.(*ast.CallExpr)
	if !ok || len(call.Args) != 1 {
		return ""
	}
	paren, ok := call.Fun.(*ast.ParenExpr)
	if !ok {
		return ""
	}
	star, ok := paren.X.(*ast.StarExpr)
	if !ok {
		return ""
	}
	ident, ok := star.X.(*ast.Ident)
	if !ok {
		return ""
	}
	// The single argument must be the untyped nil.
	if id, ok := call.Args[0].(*ast.Ident); !ok || id.Name != "nil" {
		return ""
	}
	return ident.Name
}

// seamName renders a seam type annotation: a selector `pkg.Iface`
// renders as "pkg.Iface", a bare `Iface` as "Iface". Anything else (no
// type, a star type, …) is not a seam annotation and yields "".
func seamName(t ast.Expr) string {
	switch tt := t.(type) {
	case *ast.SelectorExpr:
		if x, ok := tt.X.(*ast.Ident); ok {
			return x.Name + "." + tt.Sel.Name
		}
		return tt.Sel.Name
	case *ast.Ident:
		return tt.Name
	}
	return ""
}

// errorMethods maps each type name that owns an `Error() string` method
// in this file to that method's declaration — the seam-error shape's
// other half, structure AND message. The structure half
// (`seamErrorShape`) only needs to know the type owns the method; the
// message half (`seamErrorMessage`) walks the method body, so the map
// carries the `*ast.FuncDecl`, not just a bool.
func (ex *extractor) errorMethods() map[string]*ast.FuncDecl {
	out := map[string]*ast.FuncDecl{}
	for _, decl := range ex.file.Decls {
		d, ok := decl.(*ast.FuncDecl)
		if !ok || d.Recv == nil || d.Name.Name != "Error" || len(d.Recv.List) != 1 {
			continue
		}
		t := d.Recv.List[0].Type
		if star, isStar := t.(*ast.StarExpr); isStar {
			t = star.X
		}
		if ident, isIdent := t.(*ast.Ident); isIdent {
			out[ident.Name] = d
		}
	}
	return out
}

// seamErrorShape flags an XxxError struct that has an Error() method
// but no Spec field — a seam error that cannot cite its REQ (§5).
func (ex *extractor) seamErrorShape(s *ast.TypeSpec, errOwners map[string]*ast.FuncDecl) {
	if !strings.HasSuffix(s.Name.Name, "Error") || errOwners[s.Name.Name] == nil {
		return
	}
	st, ok := s.Type.(*ast.StructType)
	if !ok || st.Fields == nil {
		return
	}
	for _, field := range st.Fields.List {
		for _, name := range field.Names {
			if name.Name == "Spec" {
				return
			}
		}
	}
	ex.unsafeAt("seam_error_missing_req", ex.line(s.Pos()))
}

// seamErrorMessage flags an *Error type whose Error() method renders no
// REQ — the message half of the seam-error contract (§5). It walks the
// Error() body's string literals (the fmt.Sprintf/fmt.Errorf template,
// returned constants, concatenations); if none carries the Class-F `REQ`
// marker or a literal `spec://`, the type's Error() cannot cite its
// violated requirement. It fires at the type-decl line — the same anchor
// the structure half uses — so a type missing both halves reports two
// findings at one site under distinct fingerprints.
//
// The Go idiom is `violates REQ %s` where the URI rides a Spec field
// (not a literal `spec://`), so the gate honours either token: a bare
// `spec://`-only check (the literal Rust analogue) would false-red the
// guide's own clean idiom.
func (ex *extractor) seamErrorMessage(s *ast.TypeSpec, errOwners map[string]*ast.FuncDecl) {
	if !strings.HasSuffix(s.Name.Name, "Error") {
		return
	}
	method := errOwners[s.Name.Name]
	if method == nil || method.Body == nil {
		return
	}
	citesReq := false
	ast.Inspect(method.Body, func(n ast.Node) bool {
		lit, ok := n.(*ast.BasicLit)
		if !ok || lit.Kind != token.STRING {
			return true
		}
		if text, err := strconv.Unquote(lit.Value); err == nil &&
			(strings.Contains(text, "spec://") || strings.Contains(text, "violates REQ")) {
			citesReq = true
			return false
		}
		return true
	})
	if !citesReq {
		// Anchor at the Error() method, not the type decl the structure
		// half uses, so the two halves land on distinct lines: the gate
		// keys a finding by (file, line), so a shared line would swallow
		// one half in the count.
		ex.unsafeAt("seam_error_message_no_req", ex.line(method.Pos()))
	}
}

// ambientDefaults: package → the selectors that couple a cell to
// ambient state (GUIDE-AI-NATIVE-GO §2).
var ambientDefaults = map[string]map[string]bool{
	"os": {
		"Getenv": true, "Setenv": true, "LookupEnv": true, "Environ": true,
		"Stdin": true, "Stdout": true, "Stderr": true,
	},
	"time": {"Now": true, "Since": true, "Until": true},
	"http": {
		"DefaultClient": true, "DefaultServeMux": true, "DefaultTransport": true,
		"Get": true, "Post": true, "Head": true, "PostForm": true,
	},
	"flag": {"CommandLine": true, "Parse": true, "Args": true},
	"rand": {
		"Int": true, "Intn": true, "Int63": true, "Int31": true,
		"Float64": true, "Float32": true, "Seed": true, "Shuffle": true, "Perm": true,
	},
	"log":  {"Print": true, "Printf": true, "Println": true, "Fatal": true, "Fatalf": true, "Fatalln": true, "Panic": true, "Panicf": true},
	"slog": {"Default": true, "Info": true, "Warn": true, "Error": true, "Debug": true, "SetDefault": true},
}

// ambientPaths guards against a local identifier shadowing a package
// name: the selector only counts when the file really imports the path
// the short name implies.
var ambientPaths = map[string]string{
	"os": "os", "time": "time", "http": "net/http", "flag": "flag",
	"rand": "math/rand", "log": "log", "slog": "log/slog",
}

func (ex *extractor) ambient(sel *ast.SelectorExpr, pkgs map[string]string) {
	ident, ok := sel.X.(*ast.Ident)
	if !ok {
		return
	}
	wanted, isAmbientPkg := ambientDefaults[ident.Name]
	if !isAmbientPkg || !wanted[sel.Sel.Name] {
		return
	}
	imported, has := pkgs[ident.Name]
	if !has || imported != ambientPaths[ident.Name] {
		if !(ident.Name == "rand" && imported == "math/rand/v2") {
			return
		}
	}
	ex.unsafeAt("ambient_call", ex.line(sel.Pos()))
}

// errorStringCompare: `err.Error() == "…"` / `!=` — matching contract
// by prose (§5).
func (ex *extractor) errorStringCompare(b *ast.BinaryExpr) {
	if b.Op != token.EQL && b.Op != token.NEQ {
		return
	}
	if isErrorCall(b.X) || isErrorCall(b.Y) {
		ex.unsafeAt("error_string_match", ex.line(b.Pos()))
	}
}

// stringsOnError: strings.Contains/HasPrefix/HasSuffix/EqualFold over
// an .Error() result.
func (ex *extractor) stringsOnError(call *ast.CallExpr, pkgs map[string]string) {
	sel, ok := call.Fun.(*ast.SelectorExpr)
	if !ok {
		return
	}
	ident, ok := sel.X.(*ast.Ident)
	if !ok || ident.Name != "strings" || pkgs["strings"] != "strings" {
		return
	}
	switch sel.Sel.Name {
	case "Contains", "HasPrefix", "HasSuffix", "EqualFold":
	default:
		return
	}
	for _, arg := range call.Args {
		if isErrorCall(arg) {
			ex.unsafeAt("error_string_match", ex.line(call.Pos()))
			return
		}
	}
}

func isErrorCall(e ast.Expr) bool {
	call, ok := e.(*ast.CallExpr)
	if !ok || len(call.Args) != 0 {
		return false
	}
	sel, ok := call.Fun.(*ast.SelectorExpr)
	return ok && sel.Sel.Name == "Error"
}

// testSkip: t.Skip / t.Skipf / t.SkipNow in _test.go files (§10).
func (ex *extractor) testSkip(call *ast.CallExpr) {
	if !ex.inTest {
		return
	}
	sel, ok := call.Fun.(*ast.SelectorExpr)
	if !ok {
		return
	}
	switch sel.Sel.Name {
	case "Skip", "Skipf", "SkipNow":
		ex.unsafeAt("t_skip", ex.line(call.Pos()))
	}
}

// suppressions: reasonless //nolint / //lint:ignore /
// //exhaustive:ignore directives (§1).
func (ex *extractor) suppressions() {
	for _, group := range ex.file.Comments {
		for _, c := range group.List {
			text := strings.TrimSpace(c.Text)
			line := ex.line(c.Pos())
			switch {
			case strings.HasPrefix(text, "//lint:ignore"):
				rest := strings.TrimPrefix(text, "//lint:ignore")
				// blessed form: //lint:ignore <Check> <reason…>
				if len(strings.Fields(rest)) < 2 {
					ex.unsafeAt("reasonless_suppression", line)
				}
			case strings.HasPrefix(text, "//exhaustive:ignore"):
				rest := strings.TrimPrefix(text, "//exhaustive:ignore")
				if strings.TrimSpace(rest) == "" {
					ex.unsafeAt("reasonless_suppression", line)
				}
			case strings.HasPrefix(text, "//nolint"):
				// no golangci-lint here (GPL): a //nolint is dead
				// weight unless it names a linter AND a reason.
				rest := strings.TrimPrefix(text, "//nolint")
				named := strings.HasPrefix(rest, ":")
				reasoned := strings.Contains(rest, "//")
				if !named || !reasoned {
					ex.unsafeAt("reasonless_suppression", line)
				}
			}
		}
	}
}

// invariantMarkers is the fixed vocabulary the extractor emits — the
// canonical spelling the config dictionary uses. The rule re-checks the
// active config vocabulary, so the extractor emits generously; all five
// are colon-bearing labeled tags (a marker is a labeled tag, not a prose
// word), so each is self-anchoring.
var invariantMarkers = []string{
	"INVARIANT:", "WARNING:", "PANICS:", "MUST:", "NEVER:",
}

// invariantMarkerOf returns the canonical invariant marker a comment
// leads with, or "" when it leads with none. Detection is anchored at the
// comment's first content token (after the // / /* / * introducer and
// whitespace): a marker not at the very start is not detected. This
// matches the all-caps section-header convention and — deliberately —
// does not flag prose: every marker is a colon-bearing labeled tag, so a
// bare must / never / panics mid-sentence (or even leading one) is not an
// invariant declaration and does not fire. The word-boundary guard
// remains as a forward-compatible check for any future bare marker.
//
// Recorded limit: a marker embedded mid-comment is not seen; the match is
// case-sensitive to the config's canonical spelling, so `// invariant:`
// (lowercase) is not detected.
func invariantMarkerOf(text string) string {
	lead := strings.TrimLeft(text, "/*!\t ")
	for _, m := range invariantMarkers {
		if !strings.HasPrefix(lead, m) {
			continue
		}
		if strings.HasSuffix(m, ":") {
			return m // colon-bearing markers are self-anchoring
		}
		rest := lead[len(m):]
		if rest == "" || !isWordByte(rest[0]) {
			return m
		}
	}
	return ""
}

func isWordByte(b byte) bool {
	return b == '_' ||
		('a' <= b && b <= 'z') ||
		('A' <= b && b <= 'Z') ||
		('0' <= b && b <= '9')
}

// invariantComments emits one invariant_comment fact per comment whose
// lead carries an invariant marker (GUIDE-AI-NATIVE-GO §2, the
// invariant-comment-position signal R3-003). The same whole-file Comments
// walk suppressions() uses; the marker is normalised to the config
// spelling, and in_test is file-grain (the record's flag), stamped by the
// bridge the way every census fact is.
func (ex *extractor) invariantComments() {
	for _, group := range ex.file.Comments {
		for _, c := range group.List {
			marker := invariantMarkerOf(c.Text)
			if marker == "" {
				continue
			}
			ex.facts = append(ex.facts, fact{
				Fact:   "invariant_comment",
				Marker: marker,
				Line:   ex.line(c.Pos()),
			})
		}
	}
}
