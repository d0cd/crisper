use std::collections::HashMap;
use zmq::{Context, Socket, SocketType};


//TODO: Size management, this is grow only
//TODO: Make this a lattice
pub struct SocketCache {
    context: Context,
    cache: HashMap<String, Socket>,
    socket_type: SocketType,
}

impl SocketCache {
    pub fn new(con: Context, typ: SocketType) -> Self {
        SocketCache {
            context: con,
            cache: HashMap::new(),
            socket_type: typ,
        }
    }

    pub fn get(&mut self, addr: &String) -> &Socket {
        if self.cache.contains_key(addr) {
            self.cache.get(addr).unwrap()
        } else {
            let socket = self.context.socket(self.socket_type).unwrap();
            socket.connect(addr.as_str()).unwrap();
            self.cache.insert(addr.to_string(), socket);
            self.cache.get(addr).unwrap()
        }
    }
}
