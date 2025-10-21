use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct VlabBox {
    pub source_label: String,
}

impl VlabBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::VlabBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + self.source_label.len() as u64
    }
}

impl Mp4Box for VlabBox {
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
        let s = format!("source_label={}", self.source_label);
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for VlabBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let buf_size = size
            .checked_sub(HEADER_SIZE)
            .ok_or(Error::InvalidData("vlab size too small"))?;

        let mut buf = vec![0u8; buf_size as usize];
        reader.read_exact(&mut buf)?;
        if let Some(end) = buf.iter().position(|&b| b == b'\0') {
            buf.truncate(end);
        }
        let source_label = String::from_utf8(buf)?;

        Ok(VlabBox { source_label })
    }
}

impl<W: Write> WriteBox<&mut W> for VlabBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        writer.write_all(self.source_label.as_bytes())?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;
    use std::io::Cursor;

    #[test]
    fn test_vlab() {
        let src_box = VlabBox {
            source_label: "source_label".into(),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::VlabBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = VlabBox::read_box(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }
}
