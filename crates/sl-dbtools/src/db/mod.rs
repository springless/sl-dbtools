use std::io::Write;

use crate::error::DbToolsError;

pub mod pg;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy)]
pub enum DataSection {
    PreData,
    Data,
    PostData,
}

/// Common prefix to use when dumping/reading a sectioned SQL file to find
/// the relevant section
const SECTION_PREFIX: &str = "-- SECTION:";
const PRE_DATA_MARKER: &str = "PRE-DATA";
const DATA_MARKER: &str = "DATA";
const POST_DATA_MARKER: &str = "POST-DATA";

/// If `line` is a section marker, returns the section enum
pub fn line_section(line: &str) -> Option<DataSection> {
    let section_name = line.strip_prefix(SECTION_PREFIX).map(str::trim)?;

    match section_name {
        PRE_DATA_MARKER => Some(DataSection::PreData),
        DATA_MARKER => Some(DataSection::Data),
        POST_DATA_MARKER => Some(DataSection::PostData),
        _ => None,
    }
}

/// Write the section comment used by the schema loader
pub fn write_data_section<W: Write>(writer: &mut W, section: DataSection) -> Result<(), DbToolsError> {
    writeln!(writer, "{} {}\n", SECTION_PREFIX, match section {
        DataSection::PreData  => PRE_DATA_MARKER,
        DataSection::Data     => DATA_MARKER,
        DataSection::PostData => POST_DATA_MARKER,
    })?;
    Ok(())
}
