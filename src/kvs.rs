use spinach::{Lattice, LatticeMap};
use spinach::merge::{MaxMerge, MapUnionMerge};

use prost::Message;

use std::collections::HashMap;
use std::time::{Instant, Duration};

use timely::dataflow::InputHandle;
use timely::dataflow::channels::pact::Pipeline;
use timely::dataflow::operators::{Input, Map, Feedback, Branch, Broadcast, ConnectLoop, Filter};
use timely::dataflow::operators::generic::operator::{source};
use timely::dataflow::operators::generic::operator::Operator;
use timely::scheduling::Scheduler;
use timely::ExchangeData;

//TODO: Need to figure out rust imports
use crisper::proto::reqresp::{KeyRequest, RequestType, KeyResponse, KeyTuple};
use crisper::proto::lattice::{LwwValue};

use crisper::lattice::{LWWKVS};
use crisper::socketcache::SocketCache;

#[derive(Clone)]
enum BinaryOutput {
    Response(String, Option<KeyResponse>),
    Gossip(usize, Lattice<LWWKVS, MapUnionMerge>),
}



//TODO: Code clean up for KVS
//TODO: Various consistency levels
//TODO: Command line args

pub fn run() {
    timely::execute_from_args(std::env::args(), |worker| {
        let args: Vec<String> = std::env::args().collect();
        let bind_addr = args[args.len() - 1].as_str();
        let context = zmq::Context::new();
        let request_puller = context.socket(zmq::PULL).unwrap();
        let worker_index = worker.index();
        request_puller.bind(bind_addr).unwrap();

        let mut socket_cache = SocketCache::new(context, zmq::PUSH);

        // Using poll pattern for multiple sockets even though we only have one
        let mut items = [
            request_puller.as_poll_item(zmq::POLLIN),
        ];


        let mut input = InputHandle::new();

        worker.dataflow::<usize,_,_>(|scope| {


            let request_stream = scope.input_from(&mut input)
            // Convert byte stream into a stream of requests; TODO: How to safely handle bad
            // requests?
            .map::<KeyRequest,_>(|bytes: Vec<u8>| {
                let req : KeyRequest = prost::Message::decode(bytes.as_slice()).unwrap();
                req
            });

            let (gossip_handle, gossip_stream) = scope.feedback(1);

            // Serve the request
            let (request_branch, gossip_branch) = request_stream
                .binary(&gossip_stream, Pipeline, Pipeline, "GossipAndRequestService", |cap, _info| {

                    let mut delta: Lattice<LWWKVS, MapUnionMerge> = Lattice::new(HashMap::new());
                    let mut lww_kvs: Lattice<LWWKVS, MapUnionMerge> = Lattice::new(HashMap::new());

                    let mut req_vec    = Vec::new();
                    let mut gossip_vec = Vec::new();

                    move |request_input, gossip_input, output| {

                        // Drain gossip_input and merge state
                        gossip_input.for_each(|_, data| {
                            data.swap(&mut gossip_vec);
                            for state in gossip_vec.drain(..) {
                                if let Some((index, lat)) = state {
                                    if index != worker_index {
                                        println!("Received state from: {}", index);
                                        let first = Instant::now();
                                        lww_kvs.merge(lat);
                                        let second = Instant::now();
                                        println!("Merged state from: {}, took {}", index, second.duration_since(first).as_millis());
                                    }
                                }
                            }
                        });

                        
                        request_input.for_each(|time, data| {

                            data.swap(&mut req_vec);
                            let mut session = output.session(&time);
                            for req in req_vec.drain(..) {
                                // TODO: Think about using a subscope in the dataflow
                                // TODO: Various consistency levels (maybe we don't need for this experiment)
                                let resp_addr = req.response_address.clone();
                                let resp : Option<KeyResponse> = match req.r#type {
                                    // RtUnspecified
                                    0 => {
                                        session.give(BinaryOutput::Gossip(worker_index, delta.clone()));
                                        delta = Lattice::new(HashMap::new());
                                        None // TODO: Log Unknown request type
                                    }
                                    // Get 
                                    1 => Some(process_get(&lww_kvs.reveal(), req)),
                                    // Put
                                    2 => {
                                        Some(process_put(&mut lww_kvs, &mut delta, req))
                                    }
                                    // Any other value is not allowd
                                    _ => None
                                };
                                session.give(BinaryOutput::Response(resp_addr, resp));
                            }
                        });
                    }
                })
                .branch(|_time, bin_out| match bin_out {
                    BinaryOutput::Gossip(_, _) => true,
                    _ => false
                });

            // Broadcast state to other workers
            gossip_branch
                .map(|bin_out: BinaryOutput|  {
                    match bin_out {
                        BinaryOutput::Gossip(index, lat) => Some((index, lat)),
                        _ => None
                    }
                })
                .broadcast()
                .connect_loop(gossip_handle);

            // Send responses over the network
            request_branch
                .sink(Pipeline, "ZMQPushSocket", move |input| {
                    while let Some((_, data)) = input.next() {
                        for datum in data.iter() {
                            match datum {
                                BinaryOutput::Response(resp_addr, Some(resp)) => {
                                    let mut buf: Vec<u8> = Vec::new();
                                    resp.encode(&mut buf).unwrap();
                                    
                                    let response_pusher = socket_cache.get(resp_addr);
                                    response_pusher.send(buf, zmq::DONTWAIT);
                                }
                                _ => ()
                            }
                        }
                    }
                })
        });

                
        let mut time = 0;
        let mut last_advance = Instant::now();
        let mut last_gossip = Instant::now();
        loop {
            // Timeout of 10ms
            zmq::poll(&mut items, 10).unwrap();
            if items[0].is_readable() {
                let bytes = request_puller.recv_bytes(0).unwrap();
                input.send(bytes);
            }
            if Instant::now().duration_since(last_advance).as_millis() > 100 {
                time = time + 1;
                input.advance_to(time);
                last_advance = Instant::now();
            }
            if Instant::now().duration_since(last_gossip).as_millis() > 10000 {
                // Use RtUnspecified to signal gossip
                let gossip_req = KeyRequest {
                    r#type: 0,
                    tuples: Vec::new(),
                    response_address: String::from(""),
                    request_id: String::from(""),
                };

                let mut buf: Vec<u8> = Vec::new();
                gossip_req.encode(&mut buf).unwrap();
                input.send(buf);
                last_gossip = Instant::now();
            }
            worker.step();

        }
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


fn process_put(kvs: &mut Lattice<LWWKVS, MapUnionMerge>, cache: &mut Lattice<LWWKVS, MapUnionMerge>, req: KeyRequest) -> KeyResponse {
    let mut resp_tuples: Vec<KeyTuple> = Vec::new();
    for key_tuple in req.tuples {
        let lww_val: LwwValue = prost::Message::decode(key_tuple.payload.as_slice()).unwrap();
        let mut map = HashMap::new();
        map.insert(key_tuple.key.clone(), Lattice::new(lww_val));
        let single_kvs: Lattice<LWWKVS, MapUnionMerge> = Lattice::new(map);
        kvs.merge(single_kvs.clone());
        cache.merge(single_kvs);
        
        // TODO: Could we send back an empty string so that we don't need to clone tuple key
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



