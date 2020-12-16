extern crate prost_build;
extern crate serde;

use serde::{Serialize, Deserialize};

// TODO: Serialization for other lattice values
fn main() {
    let mut prost_build = prost_build::Config::new();
    prost_build.type_attribute("crisper.proto.lattice.LWWValue", "#[derive(serde::Serialize, serde::Deserialize)]");
    prost_build.compile_protos(&["proto/shared.proto", "proto/lattice.proto", "proto/reqresp.proto"],
                                &["proto/"]).unwrap();
}
