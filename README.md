# Encoding

This crate provides a set of functions to encode and decode an interaction net and its related structures.

## Net

A net consists of a root tree, a list of trees pointing to each other(redexes), and a wiring.

So we basically encode:
- The root tree using the `Tree` encoding
- How many redexes there are, using `VarLenNumber` encoding
- The redexes(which are `Vec<(Tree, Tree)>`) using the `Tree` encoding, and reading 2 times the number of redexes
- The wiring using the `Wiring` encoding

## Scalars

- `HVMRef`, uses fixed 60-bits.
- `VarLenNumber`: uses [elias-gamma encoding](https://en.wikipedia.org/wiki/Elias_gamma_coding).
- `Tag` uses 3-bits for the variant and:
  - for `NUM`, has a `VarLenNumber`
  - for `REF`, has a `HVMRef`
  - for `TUP`/`DUP`, has a `VarLenNumber` for the label
  - for `OP2`, has a fixed 4-bits for the operator
  - for the rest, has no data

## Tree

> Note: The tree encoding by itself does NOT handle the encoding of variables, they are handled by the `Net` encoding.

We encode the tree by following a pre-order traversal, and writing the tag of each node

So for example, the tree `((@foo *) [* #123])`:
- By doing a pre-order traversal, we get the structure:
  1. `((@foo *) [* #123])`, has children, visit left, write `CON` (3-bits)
  2. `(@foo *)`, has children, visit left, write `CON` (3-bits)
  3. `@foo`, has no children, write `REF(@foo)`, backtrack to sibling (3-bits + 28-bits)
  4. `*`, has no children, write `ERA`, backtrack to sibling (3-bits)
  5. `[* #123]`, has children, visit left, write `DUP(0)` (3-bits + 1-bit)
  6. `*`, has no children, write `ERA`, backtrack to sibling (3-bits)
  7. `#123`, has no children, write `NUM(123)` (3-bits + 15-bits)
  8. Done, with a total of 65 bits

## Wiring

For encoding the whole net, we need to encode the wiring between all of the `VAR` nodes(free ports).

> Remember that any `VAR` node can connect with any other `VAR` node in the net.

Suppose we have 6 ports, and we want to encode the following wiring:

```text
1 2 3 4 5 6
-----------
a b b c a c
```

- the 1st port connects with the 5th
- the 2nd port connects with the 3rd
- the 4th port connects with the 6th

We can encode this in the following way:

```text
starting with an empty wiring:
---wiring--   | written bits
_ _ _ _ _ _   | []

we begin with the first empty port, which is "a":
a _ _ _ _ _   | []

we know it is connected with the 5th port which looking at the
available empty ports is the 4th port out of 5 possible available ports
so we know that we only have to encode 5 numbers, which can fit in 3 bits,
so we write 4 in 3 bits:
a _ _ _ a _   | [0b100]

we continue with the first empty port, which is "b":
a b _ _ a _   | [0b100]

again, we know that it connects to the "global" 3rd port,
but looking at the available empty ports, we see that it is the 1st available port
out of 3 possible available ports, so we write 0 in 2 bits:
a b b _ a _   | [0b100, 0b00]

we are now in the last connection, because it is the last one,
we know that it can only connect to the remaining one, so we don't
need to write any more bits to recover the original wiring:
a b b c a c   | [0b100, 0b00]
```
