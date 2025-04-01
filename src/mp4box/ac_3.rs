use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ac3Box {
    pub data_reference_index: u16,
    pub channelcount: u16,
    pub samplesize: u16,
    pub samplerate: u16,
    pub dac3: Dac3Box,
}

impl Default for Ac3Box {
    fn default() -> Self {
        Self {
            data_reference_index: 0,
            channelcount: 2,
            samplesize: 16,
            samplerate: 48000,
            dac3: Dac3Box::default(),
        }
    }
}

impl Ac3Box {
    pub fn new(config: &Ac3Config) -> Self {
        Self {
            data_reference_index: 1,
            channelcount: 2,
            samplesize: 16,
            samplerate: config.sampling_rate,
            dac3: config.ac3.dac3.clone(),
        }
    }

    pub fn get_type(&self) -> BoxType {
        BoxType::Ac3Box
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + 28 + self.dac3.box_size()
    }
}

impl Mp4Box for Ac3Box {
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
        let s = format!(
            "channel_count={} sample_size={} sample_rate={}",
            self.channelcount, self.samplesize, self.samplerate
        );
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Ac3Box {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        reader.read_u32::<BigEndian>()?; // reserved
        reader.read_u16::<BigEndian>()?; // reserved
        let data_reference_index = reader.read_u16::<BigEndian>()?;
        reader.read_u64::<BigEndian>()?; // reserved
        let channelcount = reader.read_u16::<BigEndian>()?;
        let samplesize = reader.read_u16::<BigEndian>()?;
        reader.read_u32::<BigEndian>()?; // reserved
        let samplerate = reader.read_u16::<BigEndian>()?;
        reader.read_u16::<BigEndian>()?; // reserved

        // Find dac3 box
        let mut dac3 = None;
        let end = start + size;
        loop {
            let current = reader.stream_position()?;
            if current >= end {
                break;
            }
            let header = BoxHeader::read(reader)?;
            let BoxHeader { name, size: s } = header;
            if s > size {
                return Err(Error::InvalidData(
                    "ac-3 box contains a box with a larger size than it",
                ));
            }
            if name == BoxType::Dac3Box {
                dac3 = Some(Dac3Box::read_box(reader, s)?);
                break;
            } else {
                // Skip boxes
                let skip_to = current + s;
                skip_bytes_to(reader, skip_to)?;
            }
        }

        skip_bytes_to(reader, end)?;

        let dac3 = dac3.ok_or_else(|| Error::InvalidData("AC3SpecificBox not found"))?;

        Ok(Ac3Box {
            data_reference_index,
            channelcount,
            samplesize,
            samplerate,
            dac3,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for Ac3Box {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        writer.write_u32::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.data_reference_index)?;

        writer.write_u64::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.channelcount)?;
        writer.write_u16::<BigEndian>(self.samplesize)?;
        writer.write_u32::<BigEndian>(0)?; // reserved
        writer.write_u16::<BigEndian>(self.samplerate)?;
        writer.write_u16::<BigEndian>(0)?; // reserved

        self.dac3.write_box(writer)?;

        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Dac3Box {
    pub fscod: u8,
    pub bsid: u8,
    pub bsmod: u8,
    pub acmod: u8,
    pub lfeon: u8,
    pub bit_rate_code: u8,
}

impl Mp4Box for Dac3Box {
    fn box_type(&self) -> BoxType {
        BoxType::Dac3Box
    }

    fn box_size(&self) -> u64 {
        HEADER_SIZE + 3
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!(
            "fscod={} bsid={} bsmod={} acmod={} lfeon={} data_rate_code={}",
            self.fscod, self.bsid, self.bsmod, self.acmod, self.lfeon, self.bit_rate_code
        );
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Dac3Box {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let data = reader.read_u24::<BigEndian>()?;
        let fscod = ((data >> 22) & 0x03) as u8;
        let bsid = ((data >> 17) & 0x1F) as u8;
        let bsmod = ((data >> 14) & 0x07) as u8;
        let acmod = ((data >> 11) & 0x07) as u8;
        let lfeon = ((data >> 10) & 0x01) as u8;
        let bit_rate_code = ((data >> 5) & 0x1F) as u8;

        skip_bytes_to(reader, start + size)?;

        Ok(Dac3Box {
            fscod,
            bsid,
            bsmod,
            acmod,
            lfeon,
            bit_rate_code,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for Dac3Box {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        let fscod = (self.fscod & 0x3) as u32;
        let bsid = (self.bsid & 0x1F) as u32;
        let bsmod = (self.bsmod & 0x07) as u32;
        let acmod = (self.acmod & 0x07) as u32;
        let lfeon = (self.lfeon & 0x01) as u32;
        let bit_rate_code = (self.bit_rate_code & 0x1F) as u32;

        let data = (fscod << 22)
            | (bsid << 17)
            | (bsmod << 14)
            | (acmod << 11)
            | (lfeon << 10)
            | (bit_rate_code << 5);
        writer.write_u24::<BigEndian>(data)?;

        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;
    use std::io::Cursor;

    #[test]
    fn test_eac3() {
        let src_box = Ac3Box {
            data_reference_index: 1,
            channelcount: 2,
            samplesize: 16,
            samplerate: 48000,
            dac3: Dac3Box {
                fscod: 0,
                bsid: 8,
                bsmod: 0,
                acmod: 2,
                lfeon: 0,
                bit_rate_code: 8,
            },
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::Ac3Box);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = Ac3Box::read_box(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }
}
