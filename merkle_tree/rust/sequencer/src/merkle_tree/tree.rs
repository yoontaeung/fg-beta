use openssl::sha;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;
use std::io::{Write, BufReader, BufRead, Error};



const HASH_LENGTH:usize = 32;
const POW_OF_TWO:[u32; 30] = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192,
        16384, 32768, 65536, 131072, 262144, 524288, 1048576, 2097152, 4194304, 
        8388608, 16777216, 33554432, 67108864, 134217728, 268435456, 536870912];
pub struct Node {
    hash:[u8; HASH_LENGTH],
    l: Option<Rc<RefCell<Node>>>,
    r: Option<Rc<RefCell<Node>>>,
}

pub struct Head {
    leaf_cnt : u32,
    pow_of_two_ind : usize,
    root : Rc<RefCell<Node>>,
}

impl Head {
    pub fn new() -> Head{
        Head {
            leaf_cnt : 1,
            pow_of_two_ind : 0, // pow_of_two[ind] =< leaf_cnt < pow_of_two[ind+1]
            root : Rc::new(RefCell::new(Node::new_leaf([0; HASH_LENGTH]))),
        }
    }
    pub fn print_root(&self) {
        print!("root is : ");
        self.root.borrow().print_node();
    }

    pub fn append_leaf(&mut self, commitment:[u8; HASH_LENGTH]) {
        // println!("append leaf : leaf cnt {}, index {}, pow[ind] {}", self.leaf_cnt, self.pow_of_two_ind, POW_OF_TWO[self.pow_of_two_ind]);
        let right_leaf_cnt = self.leaf_cnt - POW_OF_TWO[self.pow_of_two_ind];
        let new_commitment = Rc::new(RefCell::new(Node::new_leaf(commitment)));
        match right_leaf_cnt {
            0 => {
                self.root = Rc::new(RefCell::new(
                                Node::new_node(
                                    self.root.clone(), 
                                    new_commitment.clone(),
                                )
                            ));
            },
            x => { 
                (*self.root).borrow_mut().insert_node(x, self.pow_of_two_ind, new_commitment.clone());
                // (*self.root).borrow_mut().recompute_hash();
            },
        }
        self.leaf_cnt += 1;
        if self.leaf_cnt >= POW_OF_TWO[self.pow_of_two_ind+1] {
            self.pow_of_two_ind += 1;
        }
    }

    pub fn print_tree(&self) {
        self.root.borrow().print_tree();
    }
}

impl Node {
    fn new_leaf(hash_value:[u8; HASH_LENGTH]) -> Node {
        Node { 
            hash : hash_value,
            l : None,
            r : None,
        } 
    }

    fn new_node(l:Rc<RefCell<Node>>, r:Rc<RefCell<Node>>) -> Node{
        let mut concat_hash = [0; HASH_LENGTH * 2];
        let (l_ptr, r_ptr) = (l.borrow(), r.borrow());
        for i in 0..HASH_LENGTH {
            concat_hash[i] = l_ptr.hash[i];
            concat_hash[i+HASH_LENGTH] = r_ptr.hash[i];
        }
        Node {
            hash : sha::sha256(&concat_hash),
            l : Some(l.clone()),
            r : Some(r.clone()),
        }
    }

    fn print_tree(&self){
        if let Some(l) = & self.l {
            l.borrow().print_tree();
        }
        if let Some(r) = & self.r {
            r.borrow().print_tree();
        }
        self.print_node();
    }

    fn print_node(&self) {
        println!("{}", hex::encode(self.hash))
    }

    fn recompute_hash(&mut self) {
        // print!("before recompute : ");
        // self.print_node();
        let mut concat_hash = [0; HASH_LENGTH * 2];
        if let Some(l_ptr) = & self.l{
            if let Some(r_ptr) = & self.r{
                for i in 0..HASH_LENGTH {
                    concat_hash[i] = l_ptr.borrow().hash[i];
                    concat_hash[i+HASH_LENGTH] = r_ptr.borrow().hash[i];
                }
                // println!("                   {}", hex::encode(concat_hash));
                self.hash = sha::sha256(&concat_hash);
                // print!("aafter recompute : ");
            }
        }
        // print!("aafter recompute : ");
        // self.print_node();
    }

    fn insert_node(&mut self, leaf_cnt:u32, index:usize, new_comm:Rc<RefCell<Node>>) {
        // println!("\tinsert node : leaf cnt {}, index {}, pow[ind] {}", leaf_cnt, index, POW_OF_TWO[index]);
        for i in (0..index).rev() {
            if leaf_cnt > POW_OF_TWO[i] {
                if let Some(r) = & self.r {
                    (*r).borrow_mut().insert_node(leaf_cnt-POW_OF_TWO[i], i, new_comm);
                } else { panic!("r should not be None"); }
                self.recompute_hash();
                // println!("\tafter recomput hash()");
                break;
            }
            else if leaf_cnt == POW_OF_TWO[i] {
                // println!("\there");
                self.r = if let Some(r) = & self.r {
                    Some(Rc::new(RefCell::new(
                                Node::new_node(
                                    r.clone(),
                                    new_comm.clone(),
                                )
                    )))
                } else { panic!("oh no"); };
                self.recompute_hash();
                break;
            }
        }
    }
}