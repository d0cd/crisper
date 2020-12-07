use spinach::{Lattice, LatticeMap};
use spinach::merge::{ MaxMerge };

use std::cmp::Ordering;

use crate::proto::lattice::{LwwValue};

// LWW stuff

impl Ord for LwwValue {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl PartialOrd for LwwValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for LwwValue {}

pub type LWWKVS = LatticeMap<String, LwwValue, MaxMerge>;

