# sif-rs

A Rust parser for the **Standard Input Format (SIF)** used in mathematical
optimization.

SIF is a fixed-width text format for describing optimization problems such as
linear programs (LP) and quadratic programs (QP). It is closely related to the
MPS format and is natively used by the
[CUTEst](https://github.com/ralna/CUTEst) benchmark library.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sif-rs = { path = "path/to/sif-rs" }   # local checkout
```

## File structure

A SIF file consists of named sections delimited by *indicator cards* — a
keyword in column 1 on its own line. The body of each section contains data
rows. Depending on the section, the first token of each data row may be a
short type marker (e.g. `N`/`G`/`L`/`E` in `ROWS`, `LO`/`UP`/`FX` in
`BOUNDS`) followed by up to two name–value pairs:

```text
<marker>  <name1>   <name2>   <value1>  <name3>   <value2>
```

For example, a `COLUMNS` entry (no marker):

```text
    c1        r1                 2.0   r2                -1.0
```

And a `ROWS` entry (single-letter marker):

```text
 G  r1
```

Lines beginning with `*` are comments and are ignored.

## Usage

```rust
let input = std::fs::read_to_string("examples/qptest.sif").unwrap();
let sif = sif_rs::parse_sif(&input).unwrap();
```

`parse_sif` returns a `SIF` value containing the parsed problem data.
The `SIF` type is currently **opaque** — its fields are not yet part of the
public API. This will change in a future release.

## Supported sections

| Section | Aliases | Description |
|---------|---------|-------------|
| `NAME` | | Problem name |
| `ROWS` | `GROUPS`, `CONSTRAINTS` | Constraint/row definitions |
| `COLUMNS` | `VARIABLES` | Variable-to-row coefficient entries |
| `RHS` | `CONSTANTS`, `RHS'` | Right-hand side values |
| `RANGES` | | Range values for constraints *(stub)* |
| `BOUNDS` | | Variable bounds |
| `START POINT` | | Warm-start variable values *(stub)* |
| `QUADRATIC` | `HESSIAN`, `QUADS`, `QUADOBJ`, `QSECTION` | Quadratic objective terms |
| `ENDATA` | | End-of-file marker |

Sections marked *stub* are recognised but their data is not yet returned.

## Row types

| Type | Meaning |
| ---- | ------- |
| `N` | Free row — typically the objective function |
| `G` | Greater-than-or-equal (≥) constraint |
| `L` | Less-than-or-equal (≤) constraint |
| `E` | Equality (=) constraint |

## Bound types

| Type | Meaning |
| ---- | ------- |
| `LO` | Lower bound |
| `UP` | Upper bound |
| `FX` | Fixed value (lower = upper) |
| `FR` | Free variable (no bounds) |
| `MI` | Lower bound of −∞ |
| `PL` | Upper bound of +∞ (default) |

## License

See [LICENSE](LICENSE).

---

> **AI disclaimer:** The documentation in this repository (README and inline
> doc comments) was generated with the assistance of Claude (Anthropic). Code
> logic and structure were written by the project author.
