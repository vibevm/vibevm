# Calculator

A calculator for the command line: `calc "7 / 2"` prints `3.5`.

## Input

One argument, quoted: two decimal numbers around one operator, separated by
spaces. The operators are `+`, `-`, `*` and `/`, and no others.

## Errors

Dividing by zero prints `error: division by zero` on standard error and
exits with code 1. Any other input that does not parse names the problem
and exits with code 2.

What is still open is in [[decisions]].
