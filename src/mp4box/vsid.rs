use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct VsidBox {
    pub source_id: u32,
}

impl VsidBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::VsidBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + 4
    }
}

impl Mp4Box for VsidBox {
    fn box_type(&self) -> BoxType {
        self.get_type()
    }

    fn box_size(&self) -> u64 {
        self.get_size()
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!("source_id={}", self.source_id);
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for VsidBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let source_id = reader.read_u32::<BigEndian>()?;

        skip_bytes_to(reader, start + size)?;

        Ok(VsidBox { source_id })
    }
}

impl<W: Write> WriteBox<&mut W> for VsidBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        writer.write_u32::<BigEndian>(self.source_id)?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;
    use std::io::Cursor;

    #[test]
    fn test_vsid() {
        let src_box = VsidBox {
            source_id: 1234,
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::VsidBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = VsidBox::read_box(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }
}
