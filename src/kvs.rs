use spinach::{Lattice, };
use spinach::merge::{MaxMerge, MinMerge, DominatingPairMerge};

use timely::dataflow::operators::{Map, Capability};
use timely::dataflow::operators::generic::operator::source;
use timely::scheduling::Scheduler;

use crisper::proto::reqresp::{KeyRequest};



// Alias for a versioned string with a simple counter clock
// Conflicting versions are resolved by taking the lexicograpjically first
// string (MinMerge).
type VersionedString = Lattice<
    (Lattice<usize, MaxMerge>, Lattice<&'static str, MinMerge>),
    DominatingPairMerge>;


pub fn run() {
    timely::execute_from_args(std::env::args(), |worker| {

        worker.dataflow::<usize,_,_>(|scope| {
            source(scope, "ZMQPullSocket", |mut capability: Capability<_>, info| {
               
                let activator = scope.activator_for(&info.address[..]);

                let context = zmq::Context::new();
                let request_puller = context.socket(zmq::PULL).unwrap();
                request_puller.bind("tcp://*:6200").unwrap();

                
                move |output| {

                    // Using poll pattern for multiple sockets even though we only have one
                    let mut items = [
                        request_puller.as_poll_item(zmq::POLLIN),
                    ];

                    // Timeout of 10ms
                    zmq::poll(&mut items, 10).unwrap();
                    if items[0].is_readable() {
                        let bytes = request_puller.recv_bytes(0).unwrap();
                        let time = capability.time().clone();
                        let mut session = output.session(&capability);

                        // downgrade capability to current time 
                        session.give(bytes);
                        capability.downgrade(&(time + 1));
                        activator.activate();
                    }

                }
            })
            // Convert byte stream into a stream of requests; TODO: How to safely handle bad
            // requests?
            .map::<KeyRequest,_>(|bytes: Vec<u8>| {
                let req : KeyRequest = prost::Message::decode(bytes.as_slice()).unwrap();
                req
            });
                
        });

        
        // TODO: Step the worker
                
    }).unwrap();
}



