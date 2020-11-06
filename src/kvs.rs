use std::io::prelude::*;
use tokio::net::{TcpListener, TcpStream};
use tokio::net::{signal};
use tokio::sync::{broadcase, mpsc};

use spinach::{Lattice, MergeIntoLattice};
use spinach::merge::{MaxMerge, MinMerge, MapUnionMerge, DominatingPairMerge};

use timely::dataflow::{InputHandle};
use timely::dataflow::operators::{ToStream, Filter};



// Alias for a versioned string with a simple counter clock
// Conflicting versions are resolved by taking the lexicograpjically first
// string (MinMerge).
type VersionedString = Lattice<
    (Lattice<usize, MaxMerge>, Lattice<&'static str, MinMerge>),
    DominatingPairMerge>;


pub async fn run() {
    timely::execute_from_args(std::env::args(), |worker| {

        // create input handle
        let mut input = InputHandle::new();

    
        worker.dataflow::<usize,_,_>(|scope| {
            input.to_stream(scope)
        });


        let context = zmq::Context::new();
        let request_puller = contet.socker(zmq::PULL).unwrap();
        request_puller.bind("tcp://*:6200").unwrap();

        let round = 0;
        
        loop {
            // Using poll pattern for multiple sockets even though we only have one
            let mut items = [
                request_puller.as_poll_item(zmq::POLLIN),
            ];
            // Timeout of 10ms
            zmq::poll(&mut items, 10).unwrap();

            if items[0].is_readable() {
                let bytes = request_puller.recv_bytes(0).unwrap();
                input.send(bytes);
            }
            input.advance_to(round++);
            worker.step();
        }
        
    }).unwrap();
}


