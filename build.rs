extern crate prost_build;

fn main() {
    prost_build::compile_protos(&["proto/shared.proto", "proto/lattice.proto", "proto/reqresp.proto"],
                                &["proto/"]).unwrap();
}
