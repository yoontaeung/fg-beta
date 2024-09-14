# BRB implementation

Dependencies
----------
- CMAKE
- C++14
- libuv >= 1.10.0
- openssl >= 1.1.0

Instructions
----------
```
# make sure you have those installed
sudo apt-get update
sudo apt-get install make
sudo apt-get install g++
sudo apt-get instasll libuv1.dev
sudo apt-get install libssl-dev
sudo apt-get install cmake

git clone fg-beta

# download and install salticidae
cd fg-beta/salticidae
git submodule update --init --recursive
cmake .
make
sudo make install

# compile sequencer
cd ../brb
mkdir bin eval log figure
make seq
```

then run `./bin/sequencer <node_index> ` on individual terminals like `./bin/sequencer 0 ` or run ` ./run.sh `

List every <ip:port> where you want to execute the sequencer in ip.config. 

ip.config
---
`ip.config` file should contain number of total nodes, round interval, message size seperated by commas and ip of each nodes seperated by new lines.
the format of the file is like the following:
```
<number_of_nodes> <round_interval> <size_of_message_in_byte>
<ip:port1>
<ip:port2>
...
```

run.sh
---
`run.sh` reads number of nodes from `ip.config` and runs that many processes.

if you are running nodes in seperate machines, then you have to explicitly change the node id in `run.sh`.

for example, if you are running node 0 ~ 4 in machine A and node 5 ~ 9 in machine B, then `starting_node_index` and `ending_node_index` of `run.sh` in machine A should be 0 and 5 repectively. 
Also `starting_node_index` and `ending_node_index` in machine B should be 5 and 10 repectively.

