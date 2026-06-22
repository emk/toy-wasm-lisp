#![allow(unsafe_op_in_unsafe_fn)]

pub mod nodes {
    include!(concat!(env!("OUT_DIR"), "/nodes.rs"));
}

pub mod queries {
    include!(concat!(env!("OUT_DIR"), "/queries.rs"));
}
