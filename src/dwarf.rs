use std::borrow::{self, Cow};

use object::{File, Object, ObjectSection};

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
