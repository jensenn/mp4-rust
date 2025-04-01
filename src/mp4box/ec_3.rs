use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::Serialize;
use std::io::{Read, Seek, Write};

use crate::mp4box::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Eac3Box {
    pub data_reference_index: u16,
    pub channelcount: u16,
    pub samplesize: u16,
    pub samplerate: u16,
    pub dec3: Dec3Box,
}

impl Default for Eac3Box {
    fn default() -> Self {
        Self {
            data_reference_index: 0,
            channelcount: 2,
            samplesize: 16,
            samplerate: 48000,
            dec3: Dec3Box::default(),
        }
    }
}

impl Eac3Box {
    pub fn new(config: &Eac3Config) -> Self {
        Self {
            data_reference_index: 1,
            channelcount: 2,
            samplesize: 16,
            samplerate: config.sampling_rate,
            dec3: config.eac3.dec3.clone(),
        }
    }

    pub fn get_type(&self) -> BoxType {
        BoxType::Eac3Box
    }

    pub fn get_size(&self) -> u64 {
        HEADER_SIZE + 28 + self.dec3.box_size()
    }
}

impl Mp4Box for Eac3Box {
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

impl<R: Read + Seek> ReadBox<&mut R> for Eac3Box {
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

        // Find dec3 box
        let mut dec3 = None;
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
                    "ec-3 box contains a box with a larger size than it",
                ));
            }
            if name == BoxType::Dec3Box {
                dec3 = Some(Dec3Box::read_box(reader, s)?);
                break;
            } else {
                // Skip boxes
                let skip_to = current + s;
                skip_bytes_to(reader, skip_to)?;
            }
        }

        skip_bytes_to(reader, end)?;

        let dec3 = dec3.ok_or_else(|| Error::InvalidData("EC3SpecificBox not found"))?;

        Ok(Eac3Box {
            data_reference_index,
            channelcount,
            samplesize,
            samplerate,
            dec3,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for Eac3Box {
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

        self.dec3.write_box(writer)?;

        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Dec3Box {
    pub data_rate: u16,
    pub ind_subs: Vec<IndSub>,
    pub complexity_index_type_a: u8,
}

impl Mp4Box for Dec3Box {
    fn box_type(&self) -> BoxType {
        BoxType::Dec3Box
    }

    fn box_size(&self) -> u64 {
        let mut size = HEADER_SIZE + 2;
        for ind_sub in &self.ind_subs {
            size += ind_sub.size();
        }
        // dolby atmos
        if self.complexity_index_type_a > 0 {
            size += 2;
        }
        size
    }

    fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self).unwrap())
    }

    fn summary(&self) -> Result<String> {
        let s = format!(
            "data_rate={} num_ind_sub={}, complexity_index_type_a={}",
            self.data_rate,
            self.ind_subs.len(),
            self.complexity_index_type_a
        );
        Ok(s)
    }
}

impl<R: Read + Seek> ReadBox<&mut R> for Dec3Box {
    fn read_box(reader: &mut R, size: u64) -> Result<Self> {
        let start = box_start(reader)?;

        let data = reader.read_u16::<BigEndian>()?;
        let data_rate = data >> 3;
        let num_ind_sub = (data & 0x07) as u8 + 1;

        let mut ind_subs = Vec::with_capacity(num_ind_sub as usize);
        for _ in 0..num_ind_sub {
            let ind_sub = IndSub::read(reader)?;
            ind_subs.push(ind_sub);
        }

        // dolby atmos
        let mut complexity_index_type_a = 0;
        let end = start + size;
        let current = reader.stream_position()?;
        if end.saturating_sub(current) >= 2 {
            let flag_ec3_extension_type_a = reader.read_u8()? & 0x01;
            if flag_ec3_extension_type_a != 0 {
                complexity_index_type_a = reader.read_u8()?;
            }
        }

        skip_bytes_to(reader, start + size)?;

        Ok(Dec3Box {
            data_rate,
            ind_subs,
            complexity_index_type_a,
        })
    }
}

impl<W: Write> WriteBox<&mut W> for Dec3Box {
    fn write_box(&self, writer: &mut W) -> Result<u64> {
        let size = self.box_size();
        BoxHeader::new(self.box_type(), size).write(writer)?;

        if self.ind_subs.is_empty() {
            return Err(Error::InvalidData("ind_subs is empty"));
        }
        if self.ind_subs.len() > 8 {
            return Err(Error::InvalidData("num_ind_sub > 8"));
        }
        let data_rate = self.data_rate & 0x1FFF;
        let num_ind_sub = (self.ind_subs.len() - 1) as u16;
        let data = (data_rate << 3) | num_ind_sub;
        writer.write_u16::<BigEndian>(data)?;

        for ind_sub in &self.ind_subs {
            ind_sub.write(writer)? as u64;
        }

        // dolby atmos
        if self.complexity_index_type_a > 0 {
            writer.write_u8(0x01)?; // flag_ec3_extension_type_a
            writer.write_u8(self.complexity_index_type_a)?;
        }

        Ok(size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct IndSub {
    pub fscod: u8,
    pub bsid: u8,
    pub bsmod: u8,
    pub acmod: u8,
    pub lfeon: u8,
    pub num_dep_sub: u8,
    pub chan_loc: u16,
}

impl IndSub {
    fn size(&self) -> u64 {
        if self.num_dep_sub > 0 {
            4
        } else {
            3
        }
    }

    fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let data = reader.read_u24::<BigEndian>()?;
        let fscod = ((data >> 22) & 0x03) as u8;
        let bsid = ((data >> 17) & 0x1F) as u8;
        let bsmod = ((data >> 12) & 0x1F) as u8;
        let acmod = ((data >> 9) & 0x07) as u8;
        let lfeon = ((data >> 8) & 0x01) as u8;
        let num_dep_sub = ((data >> 1) & 0x0F) as u8;

        let mut chan_loc = 0;
        if num_dep_sub > 0 {
            let data_hi = (data & 0x01) as u16;
            let data_lo = reader.read_u8()? as u16;
            chan_loc = data_hi << 8 | data_lo;
        }

        Ok(IndSub {
            fscod,
            bsid,
            bsmod,
            acmod,
            lfeon,
            num_dep_sub,
            chan_loc,
        })
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<u32> {
        let fscod = (self.fscod & 0x3) as u32;
        let bsid = (self.bsid & 0x1F) as u32;
        let bsmod = (self.bsmod & 0x1F) as u32;
        let acmod = (self.acmod & 0x07) as u32;
        let lfeon = (self.lfeon & 0x01) as u32;
        let num_dep_sub = (self.num_dep_sub & 0x0F) as u32;
        let chan_loc = (self.chan_loc & 0x01FF) as u32;

        let data = (fscod << 22)
            | (bsid << 17)
            | (bsmod << 12)
            | (acmod << 9)
            | (lfeon << 8)
            | (num_dep_sub << 1)
            | (chan_loc >> 8);
        writer.write_u24::<BigEndian>(data)?;

        if self.num_dep_sub > 0 {
            writer.write_u8(chan_loc as u8)?;
            Ok(4)
        } else {
            Ok(3)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mp4box::BoxHeader;
    use std::io::Cursor;

    #[test]
    fn test_eac3() {
        let src_box = Eac3Box {
            data_reference_index: 1,
            channelcount: 2,
            samplesize: 16,
            samplerate: 48000,
            dec3: Dec3Box {
                data_rate: 128,
                ind_subs: vec![IndSub {
                    fscod: 0,
                    bsid: 16,
                    bsmod: 0,
                    acmod: 2,
                    lfeon: 0,
                    num_dep_sub: 0,
                    chan_loc: 0,
                }],
                complexity_index_type_a: 0,
            },
        };
        let mut buf = Vec::new();
        src_box.write_box(&mut buf).unwrap();
        assert_eq!(buf.len(), src_box.box_size() as usize);

        let mut reader = Cursor::new(&buf);
        let header = BoxHeader::read(&mut reader).unwrap();
        assert_eq!(header.name, BoxType::Eac3Box);
        assert_eq!(src_box.box_size(), header.size);

        let dst_box = Eac3Box::read_box(&mut reader, header.size).unwrap();
        assert_eq!(src_box, dst_box);
    }
}
