import os
import matplotlib.pyplot as plt

INF = 1000

def read_data(file_path):
    with open(file_path, 'r') as file:
        latency = []
        sent = []
        recv = []
        lines = file.readlines()[2:]  # Skip the header line
        data = [line.split(':')[1].strip() for line in lines]
        for line in data:
            values = line.split()
            if values[0] == 'INF':
                latency.append(int(INF))
            else :
                latency.append(int(values[0]))
            sent.append(int(values[1]) / 1e6) # convert to MB
            recv.append(int(values[2]) / 1e6) # convert to MB

    return (latency, sent, recv)

def plot_throughput(directory, file_prefix):
    all_latency = []
    all_sent = []
    all_recv = []
    
    # Loop through each file in the directory
    for filename in os.listdir(directory):
        if filename.startswith(file_prefix):  
            file_path = os.path.join(directory, filename)
            (latency, sent, recv) = read_data(file_path)
            all_latency.append(latency)
            all_sent.append(sent)
            all_recv.append(recv)

    avg_latency = [sum(round_data) / (len(round_data)) for round_data in zip(*all_latency)]
    avg_sent = [sum(round_data) / (len(round_data)) for round_data in zip(*all_sent)]
    avg_recv = [sum(round_data) / (len(round_data)) for round_data in zip(*all_recv)]
    time = range(len(avg_sent))
    
    fig, ax1 = plt.subplots()
    ax1.plot(time, avg_latency, label='avg delivered latency', color='orange')
    ax1.set_ylabel('average latency of msg delivered (ms)', color='orange')

    ax2 = ax1.twinx()
    ax2.plot(time, avg_sent, label='avg sent (MB)', color='red')
    ax2.plot(time, avg_recv, label='avg recv (MB)', color='blue')
    ax2.set_ylabel('average throughput (MB/sec)')
    
    plt.xlabel('Round (roughly 1 second)')
    plt.title('average throughput and latency')
    plt.savefig('measurement.png')
    plt.savefig('measurement.pdf')


if __name__ == "__main__":
    directory_path = "../eval"  # Replace with the actual path to your files
    file_prefix = "node_"  # Prefix for node files
    plot_throughput(directory_path, file_prefix)
