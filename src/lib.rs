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
use std::{collections::BTreeMap, error::Error, str::FromStr, sync::LazyLock};

use types::{ColumnType, Indicator, Major, RowType};

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
    bounds: Vec<(String, String, f64)>,
    quadratic: Vec<(String, String, f64)>,
}

impl SifParser {
    fn parse_name(&self, input: &str) -> Result<String, ParseError> {
        let input = input.lines().next().ok_or_else(|| ParseError {
            message: "Input is empty, expected NAME section".to_string(),
        })?;
        (&input[..4] == "NAME")
            .then(|| input[4..].trim().to_string())
            .ok_or_else(|| ParseError {
                message: "Invalid Sif format: NAME section missing".to_string(),
            })
    }

    fn parse_rows(&mut self, input: &str) -> Result<&Vec<(String, RowType)>, ParseError> {
        let trimmed = input.lines().clone().next().ok_or_else(|| ParseError {
            message: "ROWS section is empty".to_string(),
        })?;

        let re = Regex::new(r"^(\s+[NGLE]\s+)[a-zA-Z-_0-9]*")
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

    fn parse_columns(&self, input: &str) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_entries(
        &mut self,
        input: &str,
        major: Major,
    ) -> Result<&Vec<(String, String, f64)>, ParseError> {
        let mut entries = Vec::new();

        for row in input.lines() {
            let sep = self.sep.ok_or_else(|| ParseError {
                message: "Separator not set before parsing entries".to_string(),
            })?;
            let row = row[sep as usize..].trim_start();
            let (f1, f2, val1, f4, val2) = parse_sif_row::<String, String, f64, String, f64>(row)?;

            match major {
                Major::Row => {
                    entries.push((f1.clone(), f2, val1));

                    if val2 != 0.0 {
                        entries.push((f1.clone(), f4, val2));
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
            let row = row[sep as usize..].trim_start();
            let (f1, f2, val1, f4, val2) = parse_sif_row::<String, String, f64, String, f64>(row)?;

            rhs.push((f1.clone(), f2, val1));

            if val2 != 0.0 {
                rhs.push((f1.clone(), f4, val2));
            }
        }

        self.rhs = rhs;
        Ok(&self.rhs)
    }

    fn parse_ranges(&self, input: &str) -> Result<Vec<(String, f64)>, ParseError> {
        todo!()
    }

    fn parse_bounds(&mut self, input: &str) -> Result<&Vec<(String, String, f64)>, ParseError> {
        let mut bounds = Vec::new();

        for row in input.lines() {
            let sep = self.sep.ok_or_else(|| ParseError {
                message: "Separator not set before parsing entries".to_string(),
            })?;
            let row = row[sep as usize..].trim_start();
            let (f1, f2, val1, f4, val2) = parse_sif_row::<String, String, f64, String, f64>(row)?;
            bounds.push((f1.clone(), f2, val1));
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
            let (f1, f2, val1, f4, val2) = parse_sif_row::<String, String, f64, String, f64>(row)?;
            qterms.push((f1.clone(), f2.clone(), val1));
        }

        self.quadratic = qterms;
        Ok(&self.quadratic)
    }

    fn parse_start_point(&self, input: &str) -> Result<Vec<(String, f64)>, ParseError> {
        todo!()
    }

    fn parse_element_type(&self, input: &str) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_element_uses(&self, input: &str) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_group_type(&self, input: &str) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_group_uses(&self, input: &str) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_object_bounds(&self, input: &str) -> Result<(), ParseError> {
        todo!()
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
                    } else {
                        sif.parse_entries(content, major.unwrap()).unwrap();
                    }
                    // sif.columns = parse_columns(content, Major::Column);
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
                // Indicator::ElementType => {
                //     sif.parse_element_type(content).unwrap();
                // }
                // Indicator::ElementUses => {
                //     sif.parse_element_uses(content).unwrap();
                // }
                // Indicator::GroupType => {
                //     sif.parse_group_type(content).unwrap();
                // }
                // Indicator::GroupUses => {
                //     sif.parse_group_uses(content).unwrap();
                // }
                // Indicator::ObjectBounds => {
                //     sif.parse_object_bounds(content).unwrap();
                // }
                _ => { /* Ignore other indicators for now */ }
            };
        }

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

        let bounds: BTreeMap<String, f64> = parser
            .bounds
            .iter()
            .map(|(col_name, _bound_type, value)| ((col_name.clone()), *value))
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
    bounds: BTreeMap<String, f64>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qptest() {
        let input = std::fs::read_to_string("examples/qptest.sif").unwrap();
        let sif = parse_sif(&input).unwrap();

        assert_eq!(sif.name, "QPTEST");
        assert_eq!(sif.rows.len(), 3);
        assert_eq!(sif.cols.len(), 3);
        assert_eq!(sif.entries.len(), 6);
        assert_eq!(sif.rhs.len(), 2);
        assert_eq!(sif.bounds.len(), 1);
        assert_eq!(sif.quadratic.len(), 3);
    }
}
