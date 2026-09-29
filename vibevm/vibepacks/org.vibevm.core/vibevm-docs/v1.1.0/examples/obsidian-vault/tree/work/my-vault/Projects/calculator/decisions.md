# Decisions

## The expression arrives as one quoted argument

Unquoted, a shell reads `*` as a file pattern. Revisit when an interactive
mode appears.

## Two numbers only, for this version

Longer expressions need a real parser and a precedence table. Not yet.

## Still open

- What happens to a result too large to print?
- How many digits after the point does a result keep?

The contract itself is in [[spec]].
