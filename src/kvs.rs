use spinach::{Lattice, LatticeMap};
use spinach::merge::{MaxMerge};

use prost::Message;

use std::collections::HashMap;

use timely::dataflow::channels::pact::Pipeline;
use timely::dataflow::operators::{Map, Capability};
use timely::dataflow::operators::generic::operator::{source};
use timely::dataflow::operators::generic::operator::Operator;
use timely::scheduling::Scheduler;

//TODO: Need to figure out rust imports
use crisper::proto::reqresp::{KeyRequest, RequestType, KeyResponse, KeyTuple};
use crisper::proto::lattice::{LwwValue};

use crisper::lattice::{LWWKVS};


pub fn run() {
    timely::execute_from_args(std::env::args(), |worker| {

        worker.dataflow::<usize,_,_>(|scope| {

            let context = zmq::Context::new();

            source(scope, "ZMQPullSocket", |mut capability: Capability<_>, info| {
               
                let activator = scope.activator_for(&info.address[..]);

                let request_puller = context.socket(zmq::PULL).unwrap();
                request_puller.bind("tcp://*:6200").unwrap();
                println!("Listening on port 6200...");
                
                move |output| {

                    // Using poll pattern for multiple sockets even though we only have one
                    let mut items = [
                        request_puller.as_poll_item(zmq::POLLIN),
                    ];

                    // Timeout of 10ms
                    println!("Checking for new requests...");
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
            })
            // Serve the request
            .map(|req: KeyRequest| {

                let mut lww_kvs: LWWKVS = HashMap::new();

                // TODO: Think about using a subscope in the dataflow
                // TODO: Various consistency levels (maybe we don't need for this experiment)
                let resp : Option<KeyResponse> = match req.r#type {
                    // RtUnspecified
                    0 => None, // TODO: Log Unknown request type
                    // Get 
                    1 => Some(process_get(&lww_kvs, req)),
                    // Put
                    2 => Some(process_put(&mut lww_kvs, req)),
                    // Any other value is not allowd
                    _ => None
                };
                resp
            })
            // Send the response
            .sink(Pipeline, "ZMQPushSocket", move |input| {
                // (TODO) Create a socket cache
                
                while let Some((_, data)) = input.next() {
                    for datum in data.iter() {
                        match datum {
                            Some(resp) => {
                                // Encode the message
                                let mut buf: Vec<u8> = Vec::new();
                                resp.encode(&mut buf).unwrap();
                                
                                let response_pusher = context.socket(zmq::PUSH).unwrap();
                                response_pusher.connect(resp.response_id.as_str()).unwrap();
                                response_pusher.send(buf, 0).unwrap();
                            }
                            None => ()
                        }
                    }
                }
            })
        });

        
        // Step the worker 
        worker.step_while(|| true);
                
    }).unwrap();
}

fn process_get(kvs: &LWWKVS, req: KeyRequest) -> KeyResponse {
    let mut resp_tuples: Vec<KeyTuple> = Vec::new();

    for key_tuple in req.tuples {
        let resp_tuple = if kvs.contains_key(&key_tuple.key) {
            let val: LwwValue = kvs.get(&key_tuple.key).unwrap().reveal().clone();
            KeyTuple {
                key: key_tuple.key,
                lattice_type: key_tuple.lattice_type,
                error: 0, // No error
                payload: val.value,
                address_cache_size: key_tuple.address_cache_size,
                invalidate: false,
            }
        } else {
            KeyTuple {
                key: key_tuple.key,
                lattice_type: key_tuple.lattice_type,
                error: 1, // KeyDne
                payload: Vec::new(),
                address_cache_size: key_tuple.address_cache_size,
                invalidate: false,
            }
        };
        resp_tuples.push(resp_tuple);
    }

    KeyResponse {
        r#type: req.r#type,
        tuples: resp_tuples,
        response_id: req.request_id,
        error: 0
    }
}


fn process_put(kvs: &mut LWWKVS, req: KeyRequest) -> KeyResponse {
    let mut resp_tuples: Vec<KeyTuple> = Vec::new();

    for key_tuple in req.tuples {
        if kvs.contains_key(&key_tuple.key) {
            let existing_lattice = kvs.get_mut(&key_tuple.key).unwrap();
            let lww_val = LwwValue {
                timestamp: existing_lattice.reveal().timestamp + 1,
                value: key_tuple.payload,
            };
            let new_lattice: Lattice<LwwValue, MaxMerge> = Lattice::new(lww_val);
            existing_lattice.merge(new_lattice);
        } else {
            let new_lattice: Lattice<LwwValue, MaxMerge> = Lattice::new(
                LwwValue {
                    timestamp: 0,
                    value: key_tuple.payload,
                }
            );
            
            kvs.insert(key_tuple.key.clone(), new_lattice);
        }
        // TODO: Could we send back an empty string so that we don't need to clone
        resp_tuples.push(KeyTuple {
                key: key_tuple.key,
                lattice_type: key_tuple.lattice_type,
                error: 0, // No error
                payload: Vec::new(),
                address_cache_size: key_tuple.address_cache_size,
                invalidate: false,
        });
    }

    KeyResponse {
        r#type: req.r#type,
        tuples: resp_tuples,
        response_id: req.request_id,
        error: 0
    }
}



