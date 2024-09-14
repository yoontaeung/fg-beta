import os
import matplotlib.pyplot as plt

def read_data_latency(file_path):
    with open(file_path, 'r') as file:
        data = [int(line.split(':')[1].strip())/1000 for line in file]
    return data

def plot_mean_values_latency(directory, file_name):
    all_data = []
    
    # Loop through each file in the directory
    for filename in os.listdir(directory):
        if filename.startswith(file_name):  
            file_path = os.path.join(directory, filename)
            data = read_data_latency(file_path)
            all_data.append(data)

    # Calculate mean values for each round
    '''
    for round_data in zip(*all_data):
        print(round_data)
    '''
    mean_values = [sum(round_data) / len(round_data) for round_data in zip(*all_data)]
    # print((mean_values))
    # Plot the data
    rounds = list(range(len(mean_values)))
    plt.plot(rounds, mean_values)
    plt.xlabel('Round')
    plt.ylabel('average latency (sec)')
    plt.title('average latency of '+file_name)
    # plt.show()
    plt.savefig('image_'+file_name+'.png')
    plt.clf()


def plot_mean_values_thruput(directory, file_name):
    all_data = []
    all_thruputs = []
    all_final_thruputs = []
    all_round_latency = []

    for filename in os.listdir(directory):
        if filename.startswith(file_name):
            file_path = os.path.join(directory, filename)
            with open(file_path, 'r') as file:
                lines = file.readlines()
            data = [list(map(int, line.strip().split())) for line in lines]
            #all_data.append(data)
            thruput = [((sum(data_point[1:])/1000000) / (data_point[0]/1000)) for data_point in data]
            final_thruput = [((sum(data_point[1:5])/1000000) / (data_point[0]/1000)) for data_point in data]
            round_latency = [(data_point[0]/1000) for data_point in data]
            all_thruputs.append(thruput)
            all_final_thruputs.append(final_thruput)
            all_round_latency.append(round_latency)
    
    avg_thruput = [sum(thruput) / len(thruput) for thruput in zip(*all_thruputs)]
    avg_final_thruput = [sum(final_thruput) / len(final_thruput) for final_thruput in zip(*all_final_thruputs)]
    avg_round_latency = [sum(round_latency) / len(round_latency) for round_latency in zip(*all_round_latency)]
    time_series = range(len(avg_thruput))

    fig, ax1 = plt.subplots()
    ax1.plot(time_series, avg_round_latency, label='avg round latency', color='orange')
    ax1.set_ylabel('Average latency of each round (sec)', color='orange')

    ax2 = ax1.twinx()
    ax2.plot(time_series, avg_thruput, label='avg total thruput', color='red')
    ax2.plot(time_series, avg_final_thruput, label='avg recv thruput', color='blue')
    ax2.set_ylabel('Average Throughput (MB/sec)')

    # Add labels and title
    plt.xlabel('Round')
    plt.title('Average Throughput of each round')

    plt.savefig('image_thruput.png') 
    

if __name__ == "__main__":
    directory_path = "../eval"  # Replace with the actual path to your files
    plot_mean_values_latency(directory_path, "send2echo")
    plot_mean_values_latency(directory_path, "send2fin")
    plot_mean_values_latency(directory_path, "fin2fin")
    plot_mean_values_latency(directory_path, "send2delivered")
    plot_mean_values_thruput(directory_path, "thruput")
