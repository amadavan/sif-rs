use std::str::FromStr;

use crate::ParseError;

/// Indicates whether the problem data is stored in row-major or column-major
/// order, which determines how the two name fields in each data row are
/// interpreted (row name first vs. column name first).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum Major {
    /// `ROWS` / `GROUPS` sections appear before `COLUMNS` / `VARIABLES`.
    Row,
    /// `COLUMNS` / `VARIABLES` sections appear before `ROWS` / `GROUPS`.
    Column,
}

/// The type of a row (constraint) in the SIF problem.
///
/// The single-letter variants are standard MPS/SIF row types:
///
/// | Variant | Meaning |
/// |---------|---------|
/// | `N`     | Free row (typically the objective) |
/// | `G`     | Greater-than-or-equal constraint (≥) |
/// | `L`     | Less-than-or-equal constraint (≤) |
/// | `E`     | Equality constraint (=) |
///
/// The prefixed variants (`X*`, `Z*`, `D*`) are LANCELOT/SIF extensions used
/// for nonlinear group types.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RowType {
    /// Free row (no constraint); usually the objective function.
    N,
    /// Greater-than-or-equal (≥) constraint.
    G,
    /// Less-than-or-equal (≤) constraint.
    L,
    /// Equality (=) constraint.
    E,
    // XN,
    // XG,
    // XL,
    // XE,
    // ZN,
    // ZG,
    // ZL,
    // ZE,

    // DN,
    // DG,
    // DL,
    // DE,
}

impl FromStr for RowType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "N" => Ok(RowType::N),
            "G" => Ok(RowType::G),
            "L" => Ok(RowType::L),
            "E" => Ok(RowType::E),
            // "XN" => Ok(SifRowType::XN),
            // "XG" => Ok(SifRowType::XG),
            // "XL" => Ok(SifRowType::XL),
            // "XE" => Ok(SifRowType::XE),
            // "ZN" => Ok(SifRowType::ZN),
            // "ZG" => Ok(SifRowType::ZG),
            // "ZL" => Ok(SifRowType::ZL),
            // "ZE" => Ok(SifRowType::ZE),
            // "DN" => Ok(SifRowType::DN),
            // "DG" => Ok(SifRowType::DG),
            // "DL" => Ok(SifRowType::DL),
            // "DE" => Ok(SifRowType::DE),
            _ => Err(ParseError {
                message: format!("Unknown row type: {}", s.trim()),
            }),
        }
    }
}

impl ToString for RowType {
    fn to_string(&self) -> String {
        match self {
            RowType::N => "N".to_string(),
            RowType::G => "G".to_string(),
            RowType::L => "L".to_string(),
            RowType::E => "E".to_string(),
            // SifRowType::XN => "XN".to_string(),
            // SifRowType::XG => "XG".to_string(),
            // SifRowType::XL => "XL".to_string(),
            // SifRowType::XE => "XE".to_string(),
            // SifRowType::ZN => "ZN".to_string(),
            // SifRowType::ZG => "ZG".to_string(),
            // SifRowType::ZL => "ZL".to_string(),
            // SifRowType::ZE => "ZE".to_string(),
            // SifRowType::DN => "DN".to_string(),
            // SifRowType::DG => "DG".to_string(),
            // SifRowType::DL => "DL".to_string(),
            // SifRowType::DE => "DE".to_string(),
        }
    }
}

/// The type marker for a column (variable) in the SIF `COLUMNS` section.
///
/// The marker appears in the first character position of a `COLUMNS` row and
/// controls how the variable is treated by the solver:
///
/// | Variant | Marker | Meaning |
/// |---------|--------|---------|
/// | `__`    | ` `    | Continuous variable (default) |
/// | `X`     | `X`    | Integer / general-integer variable |
/// | `Z`     | `Z`    | Binary (0-1 integer) variable |
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ColumnType {
    /// Continuous variable (blank marker).
    __,
    /// Integer variable.
    X,
    /// Binary variable.
    Z,
}

impl FromStr for ColumnType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            " " => Ok(ColumnType::__),
            "X" => Ok(ColumnType::X),
            "Z" => Ok(ColumnType::Z),
            _ => Err(ParseError {
                message: format!("Unknown column type: {}", s.trim()),
            }),
        }
    }
}

impl ToString for ColumnType {
    fn to_string(&self) -> String {
        match self {
            ColumnType::__ => " ".to_string(),
            ColumnType::X => "X".to_string(),
            ColumnType::Z => "Z".to_string(),
        }
    }
}

/// The type of a variable bound in the SIF `BOUNDS` section.
///
/// Each bound row carries a two-letter type code in its first field:
///
/// | Variant | Code | Meaning |
/// | ------- | ---- | ------- |
/// | `Lo`    | `LO` | Explicit lower bound |
/// | `Up`    | `UP` | Explicit upper bound |
/// | `Fx`    | `FX` | Fixed value (lower = upper) |
/// | `Fr`    | `FR` | Free variable (−∞ to +∞) |
/// | `Mi`    | `MI` | Lower bound of −∞ (upper stays at default) |
/// | `Pl`    | `PL` | Upper bound of +∞ (default upper) |
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BoundType {
    /// Explicit lower bound.
    Lo,
    /// Explicit upper bound.
    Up,
    /// Fixed value: lower bound equals upper bound.
    Fx,
    /// Free variable: no lower or upper bound (−∞ to +∞).
    Fr,
    /// Lower bound of −∞; upper bound unchanged.
    Mi,
    /// Upper bound of +∞ (the default); lower bound unchanged.
    Pl,
}

impl FromStr for BoundType {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "LO" => Ok(BoundType::Lo),
            "UP" => Ok(BoundType::Up),
            "FX" => Ok(BoundType::Fx),
            "FR" => Ok(BoundType::Fr),
            "MI" => Ok(BoundType::Mi),
            "PL" => Ok(BoundType::Pl),
            _ => Err(ParseError {
                message: format!("Unknown bound type: {}", s.trim()),
            }),
        }
    }
}

impl ToString for BoundType {
    fn to_string(&self) -> String {
        match self {
            BoundType::Lo => "LO".to_string(),
            BoundType::Up => "UP".to_string(),
            BoundType::Fx => "FX".to_string(),
            BoundType::Fr => "FR".to_string(),
            BoundType::Mi => "MI".to_string(),
            BoundType::Pl => "PL".to_string(),
        }
    }
}

/// A SIF section indicator (the all-caps keyword that begins each section).
///
/// Indicators appear at column 0 on a line by themselves and delimit the
/// sections of a SIF file. The parser uses them to determine how the
/// following data rows should be interpreted.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Indicator {
    /// `NAME` — problem name header.
    Name,

    /// `GROUPS` — nonlinear group definitions (LANCELOT extension).
    Groups,
    /// `ROWS` — linear row (constraint) definitions.
    Rows,
    /// `CONSTRAINTS` — alias for `ROWS`.
    Constraints,
    /// `VARIABLES` — alias for `COLUMNS`.
    Variables,
    /// `COLUMNS` — variable-to-row coefficient entries.
    Columns,

    /// `CONSTANTS` — alias for `RHS`.
    Constants,
    /// `RHS` — right-hand side values.
    Rhs,
    /// `RHS'` — transposed right-hand side (alternative notation).
    RhsPrime,
    /// `RANGES` — range values that widen equality/inequality constraints.
    Ranges,
    /// `BOUNDS` — variable bound definitions.
    Bounds,
    /// `START POINT` — initial variable values for warm-starting.
    StartPoint,
    /// `QUADRATIC` — quadratic objective coefficients.
    Quadratic,
    /// `HESSIAN` — Hessian matrix entries (alias for `QUADRATIC`).
    Hessian,
    /// `QUADS` — alias for `QUADRATIC`.
    Quads,
    /// `QUADOBJ` — quadratic objective section (QPS format).
    QuadObjective,
    /// `QSECTION` — quadratic section header (alternative QPS notation).
    QSection,
    /// `ELEMENT TYPE` — nonlinear element-type definitions (LANCELOT).
    ElementType,
    /// `ELEMENT USES` — nonlinear element instantiations (LANCELOT).
    ElementUses,
    /// `GROUP TYPE` — nonlinear group-type definitions (LANCELOT).
    GroupType,
    /// `GROUP USES` — nonlinear group instantiations (LANCELOT).
    GroupUses,
    /// `OBJECT BOUNDS` — known bounds on the objective value.
    ObjectBounds,

    /// `ENDATA` — end-of-file marker; must be the last indicator.
    Endata,
}

impl ToString for Indicator {
    fn to_string(&self) -> String {
        match self {
            Indicator::Name => "NAME".to_string(),
            Indicator::Groups => "GROUPS".to_string(),
            Indicator::Rows => "ROWS".to_string(),
            Indicator::Constraints => "CONSTRAINTS".to_string(),
            Indicator::Variables => "VARIABLES".to_string(),
            Indicator::Columns => "COLUMNS".to_string(),
            Indicator::Constants => "CONSTANTS".to_string(),
            Indicator::Rhs => "RHS".to_string(),
            Indicator::RhsPrime => "RHS'".to_string(),
            Indicator::Ranges => "RANGES".to_string(),
            Indicator::Bounds => "BOUNDS".to_string(),
            Indicator::StartPoint => "START POINT".to_string(),
            Indicator::Quadratic => "QUADRATIC".to_string(),
            Indicator::Hessian => "HESSIAN".to_string(),
            Indicator::Quads => "QUADS".to_string(),
            Indicator::QuadObjective => "QUADOBJ".to_string(),
            Indicator::QSection => "QSECTION".to_string(),
            Indicator::ElementType => "ELEMENT TYPE".to_string(),
            Indicator::ElementUses => "ELEMENT USES".to_string(),
            Indicator::GroupType => "GROUP TYPE".to_string(),
            Indicator::GroupUses => "GROUP USES".to_string(),
            Indicator::ObjectBounds => "OBJECT BOUNDS".to_string(),
            Indicator::Endata => "ENDATA".to_string(),
        }
    }
}

impl FromStr for Indicator {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "NAME" => Ok(Indicator::Name),
            "GROUPS" => Ok(Indicator::Groups),
            "ROWS" => Ok(Indicator::Rows),
            "CONSTRAINTS" => Ok(Indicator::Constraints),
            "VARIABLES" => Ok(Indicator::Variables),
            "COLUMNS" => Ok(Indicator::Columns),
            "CONSTANTS" => Ok(Indicator::Constants),
            "RHS" => Ok(Indicator::Rhs),
            "RHS'" => Ok(Indicator::RhsPrime),
            "RANGES" => Ok(Indicator::Ranges),
            "BOUNDS" => Ok(Indicator::Bounds),
            "START POINT" => Ok(Indicator::StartPoint),
            "QUADRATIC" => Ok(Indicator::Quadratic),
            "HESSIAN" => Ok(Indicator::Hessian),
            "QUADS" => Ok(Indicator::Quads),
            "QUADOBJ" => Ok(Indicator::QuadObjective),
            "QSECTION" => Ok(Indicator::QSection),
            "ELEMENT TYPE" => Ok(Indicator::ElementType),
            "ELEMENT USES" => Ok(Indicator::ElementUses),
            "GROUP TYPE" => Ok(Indicator::GroupType),
            "GROUP USES" => Ok(Indicator::GroupUses),
            "OBJECT BOUNDS" => Ok(Indicator::ObjectBounds),
            "ENDATA" => Ok(Indicator::Endata),
            _ => Err(ParseError {
                message: format!("Unknown indicator: {}", s.trim()),
            }),
        }
    }
}
