//! Parser for the **Standard Input Format (SIF)** used in mathematical
//! optimization.
//!
//! SIF is a fixed-width text format that describes optimization problems such
//! as linear programs (LP) and quadratic programs (QP). It is closely related
//! to the MPS format and is natively used by the
//! [CUTEst](https://github.com/ralna/CUTEst) benchmark library.
//!
//! # File structure
//!
//! A SIF file consists of named *sections* (called indicator cards) followed
//! by data rows. Each data row occupies a fixed-width line where fields appear
//! at specific byte offsets:
//!
//! ```text
//! [ 0..10 ][ 10..20 ][ 20..32 ][ 32..42 ][ 42..52 ]
//!  field1    field2    value1    field4    value2
//! ```
//!
//! Common sections include `ROWS`, `COLUMNS`, `RHS`, `BOUNDS`, and `ENDATA`.
//!
//! # Example
//!
//! ```no_run
//! let input = std::fs::read_to_string("examples/qptest.sif").unwrap();
//! let sif = sif_rs::parse_sif(&input).unwrap();
//! ```
pub mod types;

use derive_more::Display;
use regex::Regex;
use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    str::FromStr,
    sync::LazyLock,
};

use types::{ColumnType, Indicator, Major, RowType};

use crate::types::BoundType;

/// Error returned when a SIF input cannot be parsed.
#[derive(Debug, Display)]
pub struct ParseError {
    message: String,
}

impl Error for ParseError {}

static RE_CARDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)(^[A-Z]+)\n((^[ \t]+.*\n)+)").unwrap());

/// Parses a single SIF data row into five typed fields.
///
/// The SIF specification uses fixed byte offsets, but this parser tokenises
/// each row by whitespace for robustness with real-world files. Up to five
/// tokens are extracted:
///
/// ```text
/// token 1  → field 1  (name / type indicator)
/// token 2  → field 2  (secondary name)
/// token 3  → field 3  (first numeric value)
/// token 4  → field 4  (optional secondary name)
/// token 5  → field 5  (optional second numeric value)
/// ```
///
/// Fields that are absent (i.e. the row has fewer tokens than expected)
/// are populated with `Default::default()` rather than returning an error.
fn parse_sif_row<
    F1: Default + FromStr,
    F2: Default + FromStr,
    F3: Default + FromStr,
    F4: Default + FromStr,
    F5: Default + FromStr,
>(
    input: &str,
) -> Result<(F1, F2, F3, F4, F5), ParseError> {
    let input = {
        if input.chars().next() == Some(' ') {
            "a".to_owned() + &input[1..]
        } else {
            input.to_string()
        }
    };

    let split_input = input.split_whitespace();
    let fields: Vec<&str> = split_input.collect();

    let f1 = fields
        .get(0)
        .unwrap_or(&"")
        .trim()
        .to_string()
        .parse::<F1>()
        .map_err(|_| ParseError {
            message: "Failed to parse field 1".to_string(),
        })?;

    let f2 = fields
        .get(1)
        .unwrap_or(&"")
        .trim()
        .to_string()
        .parse::<F2>()
        .map_err(|_| ParseError {
            message: "Failed to parse field 2".to_string(),
        })?;

    let f3 = if fields.len() > 2 {
        fields
            .get(2)
            .unwrap_or(&"")
            .trim()
            .to_string()
            .parse::<F3>()
            .map_err(|_| ParseError {
                message: "Failed to parse field 3".to_string(),
            })?
    } else {
        F3::default()
    };

    let f4 = if fields.len() > 3 {
        fields
            .get(3)
            .unwrap_or(&"")
            .trim()
            .to_string()
            .parse::<F4>()
            .map_err(|_| ParseError {
                message: "Failed to parse field 4".to_string(),
            })?
    } else {
        F4::default()
    };

    let f5 = if fields.len() > 4 {
        fields
            .get(4)
            .unwrap_or(&"")
            .trim()
            .to_string()
            .parse::<F5>()
            .map_err(|_| ParseError {
                message: "Failed to parse field 5".to_string(),
            })?
    } else {
        F5::default()
    };

    Ok((f1, f2, f3, f4, f5))
}

#[allow(dead_code)]
struct SifParser {
    name: String,

    major: Option<Major>,
    sep: Option<i8>,

    rows: Vec<(String, RowType)>,
    cols: Vec<(String, ColumnType)>,
    entries: Vec<(String, String, f64)>,

    rhs: Vec<(String, String, f64)>,
    ranges: Vec<(String, f64)>,
    bounds: Vec<(String, BoundType, String, f64)>,
    quadratic: Vec<(String, String, f64)>,
}

impl SifParser {
    fn parse_name(&self, input: &str) -> Result<String, ParseError> {
        let name_line = Regex::new(r"(?m)^NAME\s+.*")
            .unwrap()
            .find(input)
            .ok_or_else(|| ParseError {
                message: "Failed to find NAME line in input".to_string(),
            })?
            .as_str();

        (&name_line[..4] == "NAME")
            .then(|| name_line[4..].trim().to_string())
            .ok_or_else(|| ParseError {
                message: "Invalid Sif format: NAME section missing".to_string(),
            })
    }

    fn parse_rows(&mut self, input: &str) -> Result<&Vec<(String, RowType)>, ParseError> {
        let trimmed = input.lines().clone().next().ok_or_else(|| ParseError {
            message: "ROWS section is empty".to_string(),
        })?;

        let re = Regex::new(r"^(\s+[XZD]?[NGLE]\s+)[a-zA-Z-_0-9]*")
            .unwrap()
            .captures(trimmed)
            .ok_or_else(|| ParseError {
                message: "Failed to get separator from ROWS section".to_string(),
            })?;

        self.sep = Some(
            re.get(1)
                .ok_or_else(|| ParseError {
                    message: "Failed to extract row type from ROWS section".to_string(),
                })?
                .as_str()
                .len() as i8,
        );

        let mut rows = Vec::new();

        for row in input.lines() {
            let sep = self.sep.ok_or_else(|| ParseError {
                message: "Separator not set before parsing entries".to_string(),
            })?;
            let type_str = row[..sep as usize].trim();
            let row = row[sep as usize..].trim_start();
            let (name, _, _, _, _) = parse_sif_row::<String, String, f64, String, f64>(row)?;
            let row_type = RowType::from_str(&type_str)?;
            rows.push((name, row_type));
        }

        self.rows = rows;
        Ok(&self.rows)
    }

    fn parse_columns(&self, _input: &str) -> Result<(), ParseError> {
        Err(ParseError {
            message: "Column definitions are not supported in this version".to_string(),
        })
    }

    fn parse_entries(
        &mut self,
        input: &str,
        major: Major,
    ) -> Result<&Vec<(String, String, f64)>, ParseError> {
        let mut entries = Vec::new();

        // let row_added = HashSet::new();
        let mut col_added = HashSet::new();

        for row in input.lines() {
            let sep = self.sep.ok_or_else(|| ParseError {
                message: "Separator not set before parsing entries".to_string(),
            })?;
            let row = row[sep as usize..].trim_start();
            let (f1, f2, val1, f4, val2) = parse_sif_row::<String, String, f64, String, f64>(row)?;

            match major {
                Major::Row => {
                    // Add columns if necessary
                    if !col_added.contains(&f1) {
                        self.cols.push((f1.clone(), ColumnType::__));
                        col_added.insert(f1.clone());
                    }

                    entries.push((f2, f1.clone(), val1));

                    if val2 != 0.0 {
                        entries.push((f4, f1.clone(), val2));
                    }
                }
                Major::Column => {
                    entries.push((f2, f1.clone(), val1));

                    if val2 != 0.0 {
                        entries.push((f4, f1.clone(), val2));
                    }
                }
            }
        }

        self.entries = entries;
        Ok(&self.entries)
    }

    fn parse_rhs(&mut self, input: &str) -> Result<&Vec<(String, String, f64)>, ParseError> {
        let mut rhs = Vec::new();

        for row in input.lines() {
            let sep = self.sep.ok_or_else(|| ParseError {
                message: "Separator not set before parsing entries".to_string(),
            })?;
            let row = row[sep as usize..].to_string();
            let (f1, f2, val1, f4, val2) =
                parse_sif_row::<String, String, f64, String, f64>(row.as_str())?;

            rhs.push((f1.clone(), f2, val1));

            if val2 != 0.0 {
                rhs.push((f1.clone(), f4, val2));
            }
        }

        self.rhs = rhs;
        Ok(&self.rhs)
    }

    fn parse_ranges(&self, _input: &str) -> Result<Vec<(String, f64)>, ParseError> {
        Err(ParseError {
            message: "Range entries are not supported in this version".to_string(),
        })
    }

    fn parse_bounds(
        &mut self,
        input: &str,
    ) -> Result<&Vec<(String, BoundType, String, f64)>, ParseError> {
        let mut bounds = Vec::new();

        for row in input.lines() {
            let sep = self.sep.ok_or_else(|| ParseError {
                message: "Separator not set before parsing entries".to_string(),
            })?;
            let type_str = row[..sep as usize].trim();
            let row = row[sep as usize..].to_string();
            let (f1, f2, val1, _, _) = parse_sif_row::<String, String, f64, String, f64>(&row)?;
            bounds.push((f1.clone(), BoundType::from_str(type_str)?, f2, val1));
        }

        self.bounds = bounds;
        Ok(&self.bounds)
    }

    fn parse_quadratic(&mut self, input: &str) -> Result<&Vec<(String, String, f64)>, ParseError> {
        let mut qterms = Vec::new();

        for row in input.lines() {
            let sep = self.sep.ok_or_else(|| ParseError {
                message: "Separator not set before parsing entries".to_string(),
            })?;
            let row = row[sep as usize..].trim_start();
            let (f1, f2, val1, _f4, _val2) =
                parse_sif_row::<String, String, f64, String, f64>(row)?;
            qterms.push((f1.clone(), f2.clone(), val1));
        }

        self.quadratic = qterms;
        Ok(&self.quadratic)
    }

    fn parse_start_point(&self, _input: &str) -> Result<Vec<(String, f64)>, ParseError> {
        Err(ParseError {
            message: "Start point entries are not supported in this version".to_string(),
        })
    }

    fn parse_element_type(&self, _input: &str) -> Result<(), ParseError> {
        Err(ParseError {
            message: "Element type entries are not supported in this version".to_string(),
        })
    }

    fn parse_element_uses(&self, _input: &str) -> Result<(), ParseError> {
        Err(ParseError {
            message: "Element uses entries are not supported in this version".to_string(),
        })
    }

    fn parse_group_type(&self, _input: &str) -> Result<(), ParseError> {
        Err(ParseError {
            message: "Group type entries are not supported in this version".to_string(),
        })
    }

    fn parse_group_uses(&self, _input: &str) -> Result<(), ParseError> {
        Err(ParseError {
            message: "Group uses entries are not supported in this version".to_string(),
        })
    }

    fn parse_object_bounds(&self, _input: &str) -> Result<(), ParseError> {
        Err(ParseError {
            message: "Object bounds entries are not supported in this version".to_string(),
        })
    }

    fn validate(&self) -> Result<bool, ParseError> {
        let vars = self
            .cols
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<HashSet<String>>();

        if vars.len() != self.cols.len() {
            return Err(ParseError {
                message: "Duplicate column names found".to_string(),
            });
        }

        let constraints = self
            .rows
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<HashSet<String>>();

        if constraints.len() != self.rows.len() {
            return Err(ParseError {
                message: "Duplicate row names found".to_string(),
            });
        }

        // Validate entries reference defined rows and columns
        for (row_name, col_name, _) in &self.entries {
            if !constraints.contains(row_name) {
                return Err(ParseError {
                    message: format!("Entry references undefined row: {}", row_name),
                });
            }
            if !vars.contains(col_name) {
                return Err(ParseError {
                    message: format!("Entry references undefined column: {}", col_name),
                });
            }
        }

        // Validate RHS entries reference defined rows
        for (_, row_name, _) in &self.rhs {
            if !constraints.contains(row_name) {
                return Err(ParseError {
                    message: format!("RHS entry references undefined row: {}", row_name),
                });
            }
        }

        // Validate bounds reference defined columns
        for (_, _, col_name, _) in &self.bounds {
            if !vars.contains(col_name) {
                return Err(ParseError {
                    message: format!("Bound entry references undefined column: {}", col_name),
                });
            }
        }

        // Validate quadratic terms reference defined columns
        for (col_name_i, col_name_j, _) in &self.quadratic {
            if !vars.contains(col_name_i) {
                return Err(ParseError {
                    message: format!("Quadratic term references undefined column: {}", col_name_i),
                });
            }
            if !vars.contains(col_name_j) {
                return Err(ParseError {
                    message: format!("Quadratic term references undefined column: {}", col_name_j),
                });
            }
        }

        Ok(true)
    }

    fn parse(input: &str) -> Result<SIF, ParseError> {
        let mut sif = SifParser {
            name: String::new(),
            major: None,
            sep: None,
            rows: Vec::new(),
            cols: Vec::new(),
            entries: Vec::new(),
            rhs: Vec::new(),
            ranges: Vec::new(),
            bounds: Vec::new(),
            quadratic: Vec::new(),
        };

        sif.name = sif.parse_name(input)?;

        let cards = RE_CARDS.captures_iter(input);
        let mut major = None;

        for card in cards {
            let indicator = Indicator::from_str(&card[1]).unwrap();
            let content = &card[2];
            match indicator {
                Indicator::Groups | Indicator::Rows | Indicator::Constraints => {
                    if major.is_none() {
                        major = Some(Major::Row);
                        sif.parse_rows(content)?;
                    } else {
                        sif.parse_entries(content, major.unwrap()).unwrap();
                    }

                    // sif.rows = parse_rows(content, Major::Row);
                }
                Indicator::Columns | Indicator::Variables => {
                    if major.is_none() {
                        major = Some(Major::Column);
                        sif.parse_columns(content)?;
                    } else {
                        sif.parse_entries(content, major.unwrap()).unwrap();
                    }
                }
                Indicator::Constants | Indicator::Rhs | Indicator::RhsPrime => {
                    sif.parse_rhs(content).unwrap();
                }
                Indicator::Ranges => {
                    sif.parse_ranges(content).unwrap();
                }
                Indicator::Bounds => {
                    sif.parse_bounds(content).unwrap();
                }
                Indicator::StartPoint => {
                    sif.parse_start_point(content).unwrap();
                }
                Indicator::Quadratic
                | Indicator::Hessian
                | Indicator::Quads
                | Indicator::QuadObjective
                | Indicator::QSection => {
                    sif.parse_quadratic(content).unwrap();
                }
                Indicator::ElementType => {
                    sif.parse_element_type(content).unwrap();
                }
                Indicator::ElementUses => {
                    sif.parse_element_uses(content).unwrap();
                }
                Indicator::GroupType => {
                    sif.parse_group_type(content).unwrap();
                }
                Indicator::GroupUses => {
                    sif.parse_group_uses(content).unwrap();
                }
                Indicator::ObjectBounds => {
                    sif.parse_object_bounds(content).unwrap();
                }
                _ => { /* Ignore other indicators for now */ }
            };
        }

        let _ = sif.validate()?;

        Ok(SIF::from(&sif))
    }
}

impl From<&SifParser> for SIF {
    fn from(parser: &SifParser) -> Self {
        let rows: BTreeMap<String, RowType> = parser
            .rows
            .iter()
            .map(|(name, row_type)| (name.clone(), *row_type))
            .collect();

        let cols: BTreeMap<String, ColumnType> = parser
            .cols
            .iter()
            .map(|(name, col_type)| (name.clone(), *col_type))
            .collect();

        let entries: BTreeMap<(String, String), f64> = parser
            .entries
            .iter()
            .map(|(row_name, col_name, coeff)| ((row_name.clone(), col_name.clone()), *coeff))
            .collect();

        let rhs: BTreeMap<String, f64> = parser
            .rhs
            .iter()
            .map(|(_rhs_name, row_name, value)| (row_name.clone(), *value))
            .collect();

        let bounds: BTreeMap<String, (BoundType, f64)> = parser
            .bounds
            .iter()
            .map(|(_, bound_type, col_name, value)| ((col_name.clone()), (*bound_type, *value)))
            .collect();

        let quadratic: BTreeMap<(String, String), f64> = parser
            .quadratic
            .iter()
            .map(|(col_name_i, col_name_j, coeff)| {
                ((col_name_i.clone(), col_name_j.clone()), *coeff)
            })
            .collect();

        let rows = if rows.len() > 0 {
            rows
        } else {
            entries
                .iter()
                .map(|((row_name, _), _)| (row_name.clone(), RowType::N))
                .into_iter()
                .collect()
        };

        let cols = if cols.len() > 0 {
            cols
        } else {
            entries
                .iter()
                .map(|((_, col_name), _)| (col_name.clone(), ColumnType::__))
                .into_iter()
                .collect()
        };

        SIF {
            name: parser.name.clone(),
            rows,
            cols,
            entries,
            rhs,
            // ranges: parser.ranges.clone(),
            bounds,
            // start_point: parser.start_point.clone(),
            quadratic,
        }
    }
}

/// A parsed SIF optimization problem.
///
/// Contains all data extracted from a SIF file. Sections that are absent in
/// the input are represented as empty maps. The fields are currently private;
/// public accessors will be added in a future release.
#[allow(unused)]
pub struct SIF {
    /// Problem name (from the `NAME` line).
    name: String,

    /// Row (constraint) definitions mapped by name.
    rows: BTreeMap<String, RowType>,
    /// Column (variable) definitions mapped by name.
    cols: BTreeMap<String, ColumnType>,
    /// Non-zero matrix entries keyed by `(row_name, col_name)`.
    entries: BTreeMap<(String, String), f64>,

    /// Right-hand side values keyed by row name.
    rhs: BTreeMap<String, f64>,
    /// Range values for constraints: `(row_name, value)`.
    // ranges: BTreeMap<String, f64>,
    /// Variable bounds keyed by column name.
    bounds: BTreeMap<String, (BoundType, f64)>,
    /// Warm-start values: `(col_name, value)`.
    // start_point: BTreeMap<String, f64>,
    /// Quadratic objective terms keyed by `(col_name_i, col_name_j)`.
    quadratic: BTreeMap<(String, String), f64>,
    // element_type: String,
    // element_uses: Vec<String>,
    // group_type: String,
    // group_uses: Vec<String>,
    // object_bounds: Vec<(String, String)>,
}

impl SIF {
    /// Creates a new empty SIF problem.
    #[allow(unused)]
    fn new() -> Self {
        SIF {
            name: String::new(),
            rows: BTreeMap::new(),
            cols: BTreeMap::new(),
            entries: BTreeMap::new(),
            rhs: BTreeMap::new(),
            // ranges: BTreeMap::new(),
            bounds: BTreeMap::new(),
            quadratic: BTreeMap::new(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_rows(&self) -> &BTreeMap<String, RowType> {
        &self.rows
    }

    pub fn get_cols(&self) -> &BTreeMap<String, ColumnType> {
        &self.cols
    }

    pub fn get_entries(&self) -> &BTreeMap<(String, String), f64> {
        &self.entries
    }

    pub fn get_rhs(&self) -> &BTreeMap<String, f64> {
        &self.rhs
    }

    pub fn get_bounds(&self) -> &BTreeMap<String, (BoundType, f64)> {
        &self.bounds
    }

    pub fn get_quadratic(&self) -> &BTreeMap<(String, String), f64> {
        &self.quadratic
    }
}

/// Parses a SIF-formatted string into a [`SIF`] problem description.
///
/// # Errors
///
/// Returns a [`ParseError`] if any section header or data row cannot be
/// decoded according to the SIF fixed-width layout.
///
/// # Example
///
/// ```no_run
/// let input = std::fs::read_to_string("examples/qptest.sif").unwrap();
/// let sif = sif_rs::parse_sif(&input).unwrap();
/// ```
pub fn parse_sif(input: &str) -> Result<SIF, ParseError> {
    SifParser::parse(input)
}

/// Reads a SIF file from disk and parses it into a [`SIF`] problem description.
///
/// This is a convenience wrapper around [`parse_sif`] that handles file I/O.
///
/// # Errors
///
/// Returns a [`ParseError`] if the file cannot be read or if the content
/// cannot be parsed.
///
/// # Example
///
/// ```no_run
/// let sif = sif_rs::parse_file("examples/qptest.sif").unwrap();
/// ```
pub fn parse_file(path: &str) -> Result<SIF, ParseError> {
    let input = std::fs::read_to_string(path).map_err(|e| ParseError {
        message: format!("Failed to read file: {}", e),
    })?;
    parse_sif(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qptest() {
        let input = std::fs::read_to_string("examples/qptest.sif").unwrap();
        let sif = parse_sif(&input).unwrap();

        assert_eq!(sif.name, "QPTEST");
        assert_eq!(sif.rows.len(), 3);
        assert_eq!(sif.cols.len(), 2);
        assert_eq!(sif.entries.len(), 6);
        assert_eq!(sif.rhs.len(), 2);
        assert_eq!(sif.bounds.len(), 1);
        assert_eq!(sif.quadratic.len(), 3);

        assert_eq!(sif.cols.get("c1"), Some(&ColumnType::__));
        assert_eq!(sif.cols.get("c2"), Some(&ColumnType::__));

        assert_eq!(sif.rows.get("obj"), Some(&RowType::N));
        assert_eq!(sif.rows.get("r1"), Some(&RowType::G));
        assert_eq!(sif.rows.get("r2"), Some(&RowType::L));

        assert_eq!(
            sif.entries.get(&("obj".to_string(), "c1".to_string())),
            Some(&1.5)
        );
        assert_eq!(
            sif.entries.get(&("r1".to_string(), "c1".to_string())),
            Some(&2.0)
        );
        assert_eq!(
            sif.entries.get(&("r2".to_string(), "c1".to_string())),
            Some(&-1.0)
        );
        assert_eq!(
            sif.entries.get(&("obj".to_string(), "c2".to_string())),
            Some(&-2.0)
        );
        assert_eq!(
            sif.entries.get(&("r1".to_string(), "c2".to_string())),
            Some(&1.0)
        );
        assert_eq!(
            sif.entries.get(&("r2".to_string(), "c2".to_string())),
            Some(&2.0)
        );

        assert_eq!(sif.rhs.get("r1"), Some(&2.0));
        assert_eq!(sif.rhs.get("r2"), Some(&6.0));

        assert_eq!(sif.bounds.get("c1"), Some(&(BoundType::Up, 20.0)));

        assert_eq!(
            sif.quadratic.get(&("c1".to_string(), "c1".to_string())),
            Some(&8.0)
        );
        assert_eq!(
            sif.quadratic.get(&("c1".to_string(), "c2".to_string())),
            Some(&2.0)
        );
        assert_eq!(
            sif.quadratic.get(&("c2".to_string(), "c2".to_string())),
            Some(&10.0)
        );
    }

    #[test]
    fn test_netlib_lp() {
        let input = std::fs::read_to_string("examples/AFIRO.SIF").unwrap();
        let sif = parse_sif(&input).unwrap();

        assert_eq!(sif.name, "AFIRO");
        assert_eq!(sif.rows.len(), 28);
        assert_eq!(sif.cols.len(), 32);
        assert_eq!(sif.entries.len(), 88);
        assert_eq!(sif.rhs.len(), 7);
        assert_eq!(sif.bounds.len(), 0);
    }

    #[test]
    fn test_blend() {
        let input = std::fs::read_to_string("examples/BLEND.SIF").unwrap();
        let sif = parse_sif(&input).unwrap();

        assert_eq!(sif.name, "BLEND");
    }

    #[test]
    fn test_sierra() {
        let input = std::fs::read_to_string("examples/SIERRA.SIF").unwrap();
        let sif = parse_sif(&input).unwrap();

        assert_eq!(sif.name, "SIERRA");
    }

    #[test]
    fn test_dfl001() {
        let input = std::fs::read_to_string("examples/DFL001.SIF").unwrap();
        let sif = parse_sif(&input).unwrap();

        assert_eq!(sif.name, "DFL001");
    }

    #[test]
    fn test_exdata() {
        let input = std::fs::read_to_string("examples/EXDATA.SIF").unwrap();
        let sif = parse_sif(&input).unwrap();

        assert_eq!(sif.name, "EXDATA");
    }
}
