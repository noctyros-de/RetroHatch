// SPDX-License-Identifier: GPL-3.0-only
//! User-provided serial-flash accessories. Never executable main firmware.
use super::super::state::{hex, require, sha256};
#[derive(Clone, Debug)]
pub struct Figure {
    pub bytes: Vec<u8>,
    pub hash: [u8; 32],
    pub name: String,
    pub id: u8,
    pub debug: bool,
}
impl Figure {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        require(
            [349536, 524288, 4194304, 4194312].contains(&bytes.len()),
            "Unsupported Tama-Go figure size",
        )?;
        require(
            bytes.get(1..3) == Some(&[0xaa, 0x55]) && bytes.get(17..19) == Some(&[0xaa, 0x55]),
            "Missing Tama-Go accessory headers",
        )?;
        let id = bytes[20];
        let debug = bytes[19] == 3 && id == 0xfe;
        require(
            debug || (bytes[19] == 3 && (3..=14).contains(&id)),
            "Unrecognized Tama-Go accessory identifier",
        )?;
        // The 4-MiB-plus-eight captures preserve the ordinary dump prefix. Eight
        // trailing capture bytes are not SPI address zero and must never shift the image.
        let flash = bytes[..bytes.len().min(4194304)].to_vec();
        let name = if debug {
            "Debug cartridge"
        } else {
            [
                "Mametchi",
                "Memetchi",
                "Kuchipatchi",
                "Chamametchi",
                "Violetchi",
                "Ringotchi",
                "Kuromametchi",
                "Sebiretchi",
                "Ponytchi",
                "Ichigotchi",
                "Shimashimatchi",
                "Makiko",
            ][(id - 3) as usize]
        };
        Ok(Self {
            bytes: flash,
            hash: sha256(bytes),
            name: format!(
                "{name}{}",
                if bytes.len() >= 4194304 {
                    " (full dump)"
                } else {
                    ""
                }
            ),
            id,
            debug,
        })
    }
    pub fn identity(&self) -> String {
        hex(&self.hash)
    }
}
