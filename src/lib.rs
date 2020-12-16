pub mod proto {
    pub mod reqresp {
        include!(concat!(env!("OUT_DIR"), "/crisper.proto.reqresp.rs"));
    }
    pub mod lattice {
        include!(concat!(env!("OUT_DIR"), "/crisper.proto.lattice.rs"));
    }
    pub mod shared {
        include!(concat!(env!("OUT_DIR"), "/crisper.proto.shared.rs"));
    }
}

pub mod lattice;
pub mod socketcache;

