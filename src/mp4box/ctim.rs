use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct CtimBox {
    pub current_time: String,
}

impl CtimBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::CtimBox
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + self.current_time.len() as u64
    }
}

impl Mp4Box for CtimBox {
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
        let s = format!("current_time={}", self.current_time);
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for CtimBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let buf_size = size
            .checked_sub(HEADER_SIZE)
            .ok_or(Error::InvalidData("ctim size too small"))?;

        let mut buf = vec![0u8; buf_size as usize];
        reader.read_exact(&mut buf)?;
        if let Some(end) = buf.iter().position(|&b| b == b'\0') {
            buf.truncate(end);
        }
        let current_time = String::from_utf8(buf)?;

        Ok(CtimBox { current_time })
    }
}

impl<W: Write> WriteBox<&mut W> for CtimBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        writer.write_all(self.current_time.as_bytes())?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;
    use std::io::Cursor;

    #[test]
    fn test_ctim() {
        let src_box = CtimBox {
            current_time: "10:53:24".into(),
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::CtimBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = CtimBox::read_box(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }
}
