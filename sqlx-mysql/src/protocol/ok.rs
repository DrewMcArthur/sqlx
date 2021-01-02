use bytes::{Buf, Bytes};
use sqlx_core::io::Deserialize;
use sqlx_core::Result;

use crate::io::MySqlBufExt;
use crate::protocol::{Capabilities, Status};

// https://dev.mysql.com/doc/internals/en/packet-OK_Packet.html

/// An OK packet is sent from the server to the client to signal successful completion of a command.
/// As of MySQL 5.7.5, OK packes are also used to indicate EOF, and EOF packets are deprecated.
#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub(crate) struct OkPacket {
    pub(crate) affected_rows: u64,
    pub(crate) last_insert_id: u64,
    pub(crate) status: Status,
    pub(crate) warnings: u16,
}

impl Deserialize<'_, Capabilities> for OkPacket {
    fn deserialize_with(mut buf: Bytes, capabilities: Capabilities) -> Result<Self> {
        let tag = buf.get_u8();
        debug_assert!(tag == 0x00 || tag == 0xfe);

        let affected_rows = buf.get_uint_lenenc();
        let last_insert_id = buf.get_uint_lenenc();

        let status =
            if capabilities.intersects(Capabilities::PROTOCOL_41 | Capabilities::TRANSACTIONS) {
                Status::from_bits_truncate(buf.get_u16_le())
            } else {
                Status::empty()
            };

        let warnings =
            if capabilities.contains(Capabilities::PROTOCOL_41) { buf.get_u16_le() } else { 0 };

        Ok(Self { affected_rows, last_insert_id, status, warnings })
    }
}

#[cfg(test)]
mod tests {
    use super::{OkPacket, Capabilities, Deserialize, Status};

    #[test]
    fn test_empty_ok_packet() {
        const DATA: &[u8] = b"\x00\x00\x00\x02@\x00\x00";

        let capabilities = Capabilities::PROTOCOL_41 | Capabilities::TRANSACTIONS;

        let ok = OkPacket::deserialize_with(DATA.into(), capabilities).unwrap();

        assert_eq!(ok.affected_rows, 0);
        assert_eq!(ok.last_insert_id, 0);
        assert_eq!(ok.warnings, 0);
        assert_eq!(ok.status, Status::AUTOCOMMIT | Status::SESSION_STATE_CHANGED);
    }
}
