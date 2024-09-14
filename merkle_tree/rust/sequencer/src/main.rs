mod merkle_tree; 

use openssl::sha;
use std::fs::File;
use std::time::{Duration, SystemTime};

pub use crate::merkle_tree::tree::*;
fn main() {
    let mut tree_head = Head::new();
    println!("print root");
    tree_head.print_root();
    let now = SystemTime::now();
    for i in 0..100_000{
        let comm = sha::sha256(format!("{}{}", "hello world", i).as_bytes());
        tree_head.append_leaf(comm);
        //tree_head.print_root();
    }
    tree_head.print_root();


    match now.elapsed() {
        Ok(elapsed) => println!("elapsed : {}", elapsed.as_millis()),
        Err(e) => println!("Error : {e:?}"), 
    }

}

