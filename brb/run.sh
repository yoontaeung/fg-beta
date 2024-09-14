#!/bin/bash
rm log/* eval/*
config_file="ip.config"

first_line=$(head -n 1 "$config_file")
read -r node_cnt time_interval msg_size <<< "$(echo "$first_line" | awk '{print $1, $2, $3}')"
echo "running evaluation with config $node_cnt, $time_interval, $msg_size"

# Path to the program
program_path="./bin/sequencer"
starting_node_index=0
ending_node_index=2

# Loop to run the program with different node indices
for ((i=$starting_node_index; i<$ending_node_index; i++)); do
    # Command to run the program with the current node index
    cmd="$program_path $i"

    # Log file for each instance
    log_file="log/node_$i.log"

    # Run the command and redirect output and error to the log file
    $cmd > $log_file 2>&1 &

    # Print a message indicating that the program is running
    echo "Node $i is running. Log file: $log_file"

done

# Wait for all background processes to finish
tail -f log/node_$starting_node_index.log
