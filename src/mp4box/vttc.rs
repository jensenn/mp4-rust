use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct VttcBox {
    pub source_id: Option<VsidBox>,
    pub cue_id: Option<IdenBox>,
    pub cue_time: Option<CtimBox>,
    pub cue_settings: Option<SttgBox>,
    pub payload: PaylBox,
}

impl VttcBox {
    pub fn get_type(&self) -> BoxType {
        BoxType::VttcBox
    }

    pub fn get_size(&self) -> u64 {
        let mut size = HEADER_SIZE;
        if let Some(ref source_id) = self.source_id {
            size += source_id.box_size();
        }
        if let Some(ref cue_id) = self.cue_id {
            size += cue_id.box_size();
        }
        if let Some(ref cue_time) = self.cue_time {
            size += cue_time.box_size();
        }
        if let Some(ref cue_settings) = self.cue_settings {
            size += cue_settings.box_size();
        }
        size += self.payload.box_size();
        size
    }
}

impl Mp4Box for VttcBox {
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
        let s = String::new();
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for VttcBox {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let mut vsid = None;
        let mut iden = None;
        let mut ctim = None;
        let mut sttg = None;
        let mut payl = None;

        let mut current = reader.stream_position()?;
        let end = start + size;
        while current < end {
            // Get box header.
            let header = BoxHeader::read(reader)?;
            let BoxHeader { name, size: s } = header;
            if s > size {
                return Err(Error::InvalidData(
                    "vttc box contains a box with a larger size than it",
                ));
            }

            match name {
                BoxType::PaylBox => {
                    payl = Some(PaylBox::read_box(reader, s)?);
                }
                BoxType::VsidBox => {
                    vsid = Some(VsidBox::read_box(reader, s)?);
                }
                BoxType::IdenBox => {
                    iden = Some(IdenBox::read_box(reader, s)?);
                }
                BoxType::CtimBox => {
                    ctim = Some(CtimBox::read_box(reader, s)?);
                }
                BoxType::SttgBox => {
                    sttg = Some(SttgBox::read_box(reader, s)?);
                }
                _ => {
                    // XXX warn!()
                    skip_box(reader, s)?;
                }
            }

            current = reader.stream_position()?;
        }

        if payl.is_none() {
            return Err(Error::BoxNotFound(BoxType::PaylBox));
        }

        skip_bytes_to(reader, start + size)?;

        Ok(VttcBox {
            source_id: vsid,
            cue_id: iden,
            cue_time: ctim,
            cue_settings: sttg,
            payload: payl.unwrap(),
        })
    }
}

impl<W: Write> WriteBox<&mut W> for VttcBox {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        if let Some(ref source_id) = self.source_id {
            source_id.write_box(writer)?;
        }
        if let Some(ref cue_id) = self.cue_id {
            cue_id.write_box(writer)?;
        }
        if let Some(ref cue_time) = self.cue_time {
            cue_time.write_box(writer)?;
        }
        if let Some(ref cue_settings) = self.cue_settings {
            cue_settings.write_box(writer)?;
        }
        self.payload.write_box(writer)?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;
    use std::io::Cursor;

    #[test]
    fn test_vttc() {
        let src_box = VttcBox {
            source_id: Some(VsidBox { source_id: 1234 }),
            cue_id: Some(IdenBox { cue_id: "1".into() }),
            cue_time: Some(CtimBox {
                current_time: "10:53:24".into(),
            }),
            cue_settings: Some(SttgBox {
                settings: "align:center".into(),
            }),
            payload: PaylBox {
                cue_text: "test me".into(),
            },
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::VttcBox);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = VttcBox::read_box(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }
}
