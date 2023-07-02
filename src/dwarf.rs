use std::borrow::{self, Cow};

use object::{File, Object, ObjectSection};

pub struct Dwarf<'a> {
    dwarf: gimli::Dwarf<gimli::EndianSlice<'a, gimli::RunTimeEndian>>,
}

impl<'a> Dwarf<'a> {
    pub fn new(dwarf: gimli::Dwarf<gimli::EndianSlice<'a, gimli::RunTimeEndian>>) -> Dwarf<'a> {
        Dwarf { dwarf }
    }

    pub fn get_source_line_addr(&self, filename: String, line: u64) -> Result<Option<u64>, String> {
        let mut units = self.dwarf.units();

        let line_program = loop {
            if let Some(header) = units
                .next()
                .map_err(|e| format!("failed to get next header of dwarf unit: {}", e))?
            {
                let unit = self
                    .dwarf
                    .unit(header)
                    .map_err(|e| format!("failed to construct dwarf unit from header: {}", e))?;

                if self
                    .get_unit_name(&unit)
                    .map_err(|e| format!("failed to get name of dwarf unit: {}", e))?
                    .filter(|name| name == &filename)
                    .is_some()
                {
                    break unit.line_program;
                }
            } else {
                break None;
            }
        };

        Ok(line_program).and_then(|program| {
            if let Some(mut rows) = program.map(|p| p.rows()) {
                while let Some((_, row)) = rows
                    .next_row()
                    .map_err(|e| format!("failed to get next row of source: {}", e))?
                {
                    if row.is_stmt() && row.line().filter(|l| l.get() == line).is_some() {
                        return Ok(Some(row.address()));
                    }
                }

                return Ok(None);
            }

            Ok(None)
        })
    }

    fn get_unit_name(
        &self,
        unit: &gimli::Unit<gimli::EndianSlice<gimli::RunTimeEndian>, usize>,
    ) -> Result<Option<&str>, String> {
        let mut tree = unit
            .entries_tree(None)
            .map_err(|e| format!("failed to get entries tree: {}", e))?;
        let root = tree
            .root()
            .map_err(|e| format!("failed to get root of entries tree: {}", e))?;
        let offset = root
            .entry()
            .attr_value(gimli::DW_AT_name)
            .map_err(|e| format!("failed to get offset of name: {}", e))?;

        match offset {
            Some(gimli::AttributeValue::DebugLineStrRef(offset)) => Ok(Some(
                self.dwarf
                    .debug_line_str
                    .get_str(offset)
                    .map_err(|e| format!("failed to load name by offset: {}", e))?
                    .to_string()
                    .map_err(|e| format!("failed to parse name: {}", e))?,
            )),
            _ => Ok(None),
        }
    }
}

pub fn load_dwarf(mmap: &[u8]) -> Result<(gimli::Dwarf<Cow<[u8]>>, gimli::RunTimeEndian), String> {
    let object: File =
        object::File::parse(mmap).map_err(|e| format!("failed to parse object file: {}", e))?;

    let load_section = |id: gimli::SectionId| -> Result<borrow::Cow<[u8]>, gimli::Error> {
        match object.section_by_name(id.name()) {
            Some(ref section) => Ok(section
                .uncompressed_data()
                .unwrap_or(borrow::Cow::Borrowed(&[][..]))),
            None => Ok(borrow::Cow::Borrowed(&[][..])),
        }
    };

    let endian = if object.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    Ok((
        gimli::Dwarf::load(&load_section).map_err(|e| format!("failed to load dwarf: {}", e))?,
        endian,
    ))
}

pub fn borrow_section<'a>(
    dwarf: &'a gimli::Dwarf<Cow<[u8]>>,
    endian: gimli::RunTimeEndian,
) -> gimli::Dwarf<gimli::EndianSlice<'a, gimli::RunTimeEndian>> {
    let borrow_section: &dyn for<'b> Fn(
        &'b borrow::Cow<'b, [u8]>,
    ) -> gimli::EndianSlice<'b, gimli::RunTimeEndian> =
        &|section| gimli::EndianSlice::new(&*section, endian);

    dwarf.borrow(borrow_section)
}
