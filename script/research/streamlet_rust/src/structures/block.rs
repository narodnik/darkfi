use std::hash::{Hash, Hasher};

use super::metadata::Metadata;

use darkfi::{tx::Transaction, util::serial::Encodable};

/// This struct represents a tuple of the form (st, sl, txs, metadata).
/// Each blocks parent hash h may be computed simply as a hash of the parent block.
#[derive(Debug, Clone)]
pub struct Block {
    /// Previous block hash
    pub st: String,
    /// Slot uid, generated by the beacon
    pub sl: u64,
    /// Transactions payload
    pub txs: Vec<Transaction>,
    /// Additional block information
    pub metadata: Metadata,
}

impl Block {
    pub fn new(
        st: String,
        sl: u64,
        txs: Vec<Transaction>,
        proof: String,
        r: String,
        s: String,
    ) -> Block {
        Block { st, sl, txs, metadata: Metadata::new(proof, r, s) }
    }

    pub fn signature_encode(&self) -> Vec<u8> {
        let mut encoded_block = Vec::new();
        let mut len = 0;
        len += self.st.encode(&mut encoded_block).unwrap();
        len += self.sl.encode(&mut encoded_block).unwrap();
        len += self.txs.encode(&mut encoded_block).unwrap();
        assert_eq!(len, encoded_block.len());
        encoded_block
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.st == other.st && self.sl == other.sl && self.txs == other.txs
    }
}

impl Hash for Block {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        format!("{:?}{:?}{:?}", self.st, self.sl, self.txs).hash(hasher);
    }
}
