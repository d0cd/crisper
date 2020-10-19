extern crate spinach;
extern crate timely;

use std::collections::HashMap;

use spinach::{Lattice};
use spinach::merge::{MaxMerge, MinMerge, MapUnionMerge, DominatingPairMerge};

use timely::dataflow::{InputHandle};
use timely::dataflow::operators::{Filter};
use timely::dataflow::operators::generic::operator::{Operator};
use timely::dataflow::channels::pact::Pipeline;

// To run:
// `cargo test actor_test -- --nocapture`


// Alias for a versioned string with a simple counter clock
// Conflicting versions are resolved by taking the lexicograpjically first
// string (MinMerge).
type VersionedString = Lattice<
    (Lattice<usize, MaxMerge>, Lattice<&'static str, MinMerge>),
    DominatingPairMerge>;


#[test]
fn actor_test() {
    timely::execute_from_args(std::env::args(), |worker| {

        // create input handle
        let mut input = InputHandle::new();

        worker.dataflow::<usize,_,_>(|scope| {
            input.to_stream(scope)
                .filter(|&(_, _, v) : &(&'static str, usize, &'static str)| !v.contains("Christ"))
                .sink(Pipeline, "merge", |input| {

                    // KVS for mapping string to `VersionedString`s
                    let mut kvs: Lattice<HashMap<&'static str, VersionedString>, MapUnionMerge>
                        = Default::default();

                    let mut vector = Vec::new();

                    while let Some((_, data)) = input.next() {
                        data.swap(&mut vector);
                        for (k, t, v) in vector.drain(..) {
                            let mut y: HashMap<_,_> = Default::default();
                            y.insert(k, (t.into(), v.into()).into());
                            kvs.merge_in(y);
                            
                        }
                        // Show state of KVS at each step (there's really only one)
                        println!("{:#?}", kvs.reveal());
                    }
                });
        });

        // feed the dataflow with data.
        input.send(("chancellor", 2017, "Carol T. Christ"));
        input.send(("chancellor", 2004, "Robert J. Birgeneau"));
        input.send(("trillion_usd_company", 2018, "AAPL"));
        input.send(("chancellor", 2013, "Nicholas B. Dirks"));
        input.send(("trillion_usd_company", 2018, "AMZN"));
        // let the worker run
        worker.step();

    }).unwrap();
}
