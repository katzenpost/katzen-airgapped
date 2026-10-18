use std::collections::HashMap;

pub mod sandbox;
pub mod sphinx;

pub const MAGIC: [u8; 4] = *b"KPAG";
pub const PROTOCOL_VERSION: u8 = 1;
pub const DEFAULT_CHUNK_SIZE: usize = 90; // Compact Version 4-5 QR code: ~37x20 chars in terminal

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    SphinxOut = 0x01,
    SurbDelegate = 0x02,
    ReplyIn = 0x03,
    PkiConsensusUpdate = 0x04,
    HotStatus = 0x05,
}

impl MsgType {
    pub fn from_u8(b: u8) -> Result<Self, String> {
        match b {
            0x01 => Ok(Self::SphinxOut),
            0x02 => Ok(Self::SurbDelegate),
            0x03 => Ok(Self::ReplyIn),
            0x04 => Ok(Self::PkiConsensusUpdate),
            0x05 => Ok(Self::HotStatus),
            other => Err(format!("Unknown MsgType: 0x{:02x}", other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KpagHeader {
    pub magic: [u8; 4],
    pub version: u8,
    pub msg_type: MsgType,
    pub seq_id: u32,
    pub part_index: u16,
    pub total_parts: u16,
    pub payload_len: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KpagChunk {
    pub header: KpagHeader,
    pub data: Vec<u8>,
    pub checksum: [u8; 4],
}

impl KpagChunk {
    pub fn encode_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(16 + self.data.len() + 4);
        buf.extend_from_slice(&self.header.magic);
        buf.push(self.header.version);
        buf.push(self.header.msg_type as u8);
        buf.extend_from_slice(&self.header.seq_id.to_be_bytes());
        buf.extend_from_slice(&self.header.part_index.to_be_bytes());
        buf.extend_from_slice(&self.header.total_parts.to_be_bytes());
        buf.extend_from_slice(&(self.data.len() as u16).to_be_bytes());
        buf.extend_from_slice(&self.data);

        let hash = blake3::hash(&buf);
        let checksum: [u8; 4] = hash.as_bytes()[0..4].try_into().unwrap();
        buf.extend_from_slice(&checksum);
        buf
    }

    pub fn decode_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 20 {
            return Err("Packet too short (minimum 20 bytes for header + checksum)".to_string());
        }

        let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if magic != MAGIC {
            return Err("Invalid magic bytes, expected 'KPAG'".to_string());
        }

        let version = bytes[4];
        if version != PROTOCOL_VERSION {
            return Err(format!("Unsupported protocol version: {}", version));
        }

        let msg_type = MsgType::from_u8(bytes[5])?;
        let seq_id = u32::from_be_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]);
        let part_index = u16::from_be_bytes([bytes[10], bytes[11]]);
        let total_parts = u16::from_be_bytes([bytes[12], bytes[13]]);
        let payload_len = u16::from_be_bytes([bytes[14], bytes[15]]) as usize;

        if bytes.len() != 16 + payload_len + 4 {
            return Err(format!(
                "Payload length mismatch: header says {} bytes, total packet size is {}",
                payload_len,
                bytes.len()
            ));
        }

        let data = bytes[16..16 + payload_len].to_vec();
        let expected_checksum = &bytes[16 + payload_len..];

        let hash = blake3::hash(&bytes[0..16 + payload_len]);
        let calculated_checksum = &hash.as_bytes()[0..4];

        if expected_checksum != calculated_checksum {
            return Err("BLAKE3 checksum validation failed".to_string());
        }

        let mut checksum = [0u8; 4];
        checksum.copy_from_slice(expected_checksum);

        Ok(Self {
            header: KpagHeader {
                magic,
                version,
                msg_type,
                seq_id,
                part_index,
                total_parts,
                payload_len: payload_len as u16,
            },
            data,
            checksum,
        })
    }

    pub fn to_qr_string(&self) -> String {
        format!("kpag://{}", hex::encode(self.encode_bytes()))
    }

    pub fn from_qr_string(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let raw = if let Some(stripped) = s.strip_prefix("kpag://") {
            stripped
        } else if let Some(stripped) = s.strip_prefix("KPAG://") {
            stripped
        } else {
            s
        };

        let bytes = hex::decode(raw).map_err(|e| format!("Hex decode error: {}", e))?;
        Self::decode_bytes(&bytes)
    }
}

pub fn fragment_payload(
    msg_type: MsgType,
    seq_id: u32,
    payload: &[u8],
    chunk_size: usize,
) -> Vec<KpagChunk> {
    if payload.is_empty() {
        let chunk = KpagChunk {
            header: KpagHeader {
                magic: MAGIC,
                version: PROTOCOL_VERSION,
                msg_type,
                seq_id,
                part_index: 0,
                total_parts: 1,
                payload_len: 0,
            },
            data: Vec::new(),
            checksum: [0; 4],
        };
        return vec![chunk];
    }

    let chunks: Vec<&[u8]> = payload.chunks(chunk_size).collect();
    let total_parts = chunks.len() as u16;

    chunks
        .into_iter()
        .enumerate()
        .map(|(i, slice)| {
            let mut chunk = KpagChunk {
                header: KpagHeader {
                    magic: MAGIC,
                    version: PROTOCOL_VERSION,
                    msg_type,
                    seq_id,
                    part_index: i as u16,
                    total_parts,
                    payload_len: slice.len() as u16,
                },
                data: slice.to_vec(),
                checksum: [0; 4],
            };
            // compute checksum
            let bytes = chunk.encode_bytes();
            chunk.checksum.copy_from_slice(&bytes[bytes.len() - 4..]);
            chunk
        })
        .collect()
}

pub struct Reassembler {
    pending: HashMap<u32, (MsgType, u16, HashMap<u16, Vec<u8>>)>,
}

impl Reassembler {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    pub fn add_chunk(&mut self, chunk: KpagChunk) -> Result<Option<(MsgType, u32, Vec<u8>)>, String> {
        let seq = chunk.header.seq_id;
        let total = chunk.header.total_parts;
        let idx = chunk.header.part_index;
        let mtype = chunk.header.msg_type;

        let entry = self
            .pending
            .entry(seq)
            .or_insert_with(|| (mtype, total, HashMap::new()));

        if entry.1 != total || entry.0 != mtype {
            return Err(format!(
                "Inconsistent chunk metadata for sequence ID {}",
                seq
            ));
        }

        entry.2.insert(idx, chunk.data);

        if entry.2.len() == total as usize {
            let mut full_payload = Vec::new();
            for i in 0..total {
                if let Some(part) = entry.2.get(&i) {
                    full_payload.extend_from_slice(part);
                } else {
                    return Err(format!("Missing chunk {} for sequence {}", i, seq));
                }
            }
            self.pending.remove(&seq);
            Ok(Some((mtype, seq, full_payload)))
        } else {
            Ok(None)
        }
    }

    pub fn progress(&self, seq_id: u32) -> Option<(usize, usize)> {
        self.pending.get(&seq_id).map(|(_, total, parts)| (parts.len(), *total as usize))
    }
}

pub fn render_qr_terminal(content: &str) -> Result<String, String> {
    use qrcode::QrCode;
    use qrcode::render::unicode::Dense1x2;
    let code = QrCode::new(content.as_bytes()).map_err(|e| format!("QR generation error: {}", e))?;
    let string = code
        .render::<Dense1x2>()
        .quiet_zone(true)
        .build();
    Ok(string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_encode_decode() {
        let data = b"Sphinx packet payload 1024 bytes dummy test";
        let chunks = fragment_payload(MsgType::SphinxOut, 42, data, 16);
        assert_eq!(chunks.len(), 3);

        let mut reassembler = Reassembler::new();
        let mut completed = None;

        for chunk in chunks {
            let encoded = chunk.encode_bytes();
            let decoded = KpagChunk::decode_bytes(&encoded).unwrap();
            if let Some(res) = reassembler.add_chunk(decoded).unwrap() {
                completed = Some(res);
            }
        }

        let (mtype, seq, reassembled) = completed.expect("Should reassemble");
        assert_eq!(mtype, MsgType::SphinxOut);
        assert_eq!(seq, 42);
        assert_eq!(reassembled, data);
    }
}
