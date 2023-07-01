use std::borrow::{self, Cow};

use object::{File, Object, ObjectSection};

pub struct Dwarf<'a> {
    dwarf: gimli::Dwarf<gimli::EndianSlice<'a, gimli::RunTimeEndian>>,
}

impl<'a> Dwarf<'a> {
    pub fn new(dwarf: gimli::Dwarf<gimli::EndianSlice<'a, gimli::RunTimeEndian>>) -> Dwarf<'a> {
        Dwarf { dwarf }
    }

    pub fn get_source_line_addr(&self, filename: String, line: u64) -> Option<u64> {
        let mut units = self.dwarf.units();

        let line_program = loop {
            if let Some(header) = units.next().expect("failed to iter over dwarf units") {
                let unit = self
                    .dwarf
                    .unit(header)
                    .expect("failed to construct dwarf unit");

                if self.get_unit_name(&unit).filter(|name| name == &filename).is_some() {
                    break unit.line_program;
                }
            } else {
                break None;
            }
        };

        line_program.and_then(|program| {
            let mut rows = program.rows();

            while let Some((_, row)) = rows.next_row().expect("failed to get next source row") {
                if row.is_stmt() && row.line().filter(|l| l.get() == line).is_some() {
                    return Some(row.address());
                }
            }

            None
        })
    }

    fn get_unit_name(
        &self,
        unit: &gimli::Unit<gimli::EndianSlice<gimli::RunTimeEndian>, usize>,
    ) -> Option<&str> {
        let mut tree = unit
            .entries_tree(None)
            .expect("failed to get dwarf entries tree");
        let root = tree
            .root()
            .expect("failed to get root of dwarf entries tree");
        let offset = root
            .entry()
            .attr_value(gimli::DW_AT_name)
            .expect("failed to get offset of unit name");

        match offset {
            Some(gimli::AttributeValue::DebugLineStrRef(offset)) => Some(
                self.dwarf
                    .debug_line_str
                    .get_str(offset)
                    .expect("failed to get unit name")
                    .to_string()
                    .expect("failed to parse unit name"),
            ),
            _ => None,
        }
    }
}

pub fn load_dwarf(mmap: &[u8]) -> (gimli::Dwarf<Cow<[u8]>>, gimli::RunTimeEndian) {
    let object: File = object::File::parse(mmap).unwrap();

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

    (
        gimli::Dwarf::load(&load_section).expect("failed to load dwarf"),
        endian,
    )
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
