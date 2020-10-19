use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

use serde::{Serialize, Deserialize};

use spinach::{Lattice, MergeIntoLattice};
use spinach::merge::{MaxMerge, MinMerge, MapUnionMerge, DominatingPairMerge};

use timely::dataflow::{InputHandle};
use timely::dataflow::operators::{ToStream, Filter};

// To run:
// `cargo test toy_kvs -- --nocapture`

// Define some stuff for the KVS
#[derive(Clone, Serialize, Deserialize)]
pub enum Request {
    Get { key: String},
    Put { key: String, value: String},
}

// Alias for a versioned string with a simple counter clock
// Conflicting versions are resolved by taking the lexicograpjically first
// string (MinMerge).
type VersionedString = Lattice<
    (Lattice<usize, MaxMerge>, Lattice<&'static str, MinMerge>),
    DominatingPairMerge>;


pub fn run() {
    timely::execute_from_args(std::env::args(), |worker| {

        // create input handle
        //let mut input = InputHandle::new();

    
        //worker.dataflow::<usize,_,_>(|scope| {
        //    input.to_stream(scope)
        //});

        let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let req = extract_request(stream);
            //input.send(req);
            //worker.step();
        }
    }).unwrap();
}

fn extract_request(mut stream: TcpStream) -> Request {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    let message = String::from_utf8_lossy(&buffer[..]);
    let request: Request = serde_json::from_str(&message).unwrap();
    request
}
