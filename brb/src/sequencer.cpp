#include <fstream>
#include <openssl/crypto.h>
#include "salticidae/event.h"
#include "salticidae/network.h"
#include "eddsa.hpp"
#include "msg_struct.hpp"
#include "macro.hpp"
#include "net.hpp"

#define SIGN_LEN 64

using salticidae::NetAddr;
using namespace std;


int 
main(int argc, char* argv[]) 
{ // ./a.out 0
    char pem_file_name[32];
    int curr_node_idx, node_cnt, round_interval, msg_size, dynamic_msg_size;
    salticidae::EventContext ec;
    NetAddr curr_node_ip;
    EVP_PKEY *pri_key;
    unique_ptr<MyNet> curr_node;
    string config_buf, curr_node_ip_str;
    vector<EVP_PKEY*> peers_pub_key;
    vector<NetAddr> curr_peers;
    ifstream ip_config("ip.config");

    if(!getline(ip_config, config_buf)) { fprintf(stderr, "ip.config peer cnt error\n"); return 0; }
    curr_node_idx = std::stoi(argv[1]);
    // node_cnt = stoi(config_buf);
    if(sscanf(config_buf.c_str(), "%d %d %d %d", &node_cnt, &round_interval, &msg_size, &dynamic_msg_size) != 4) { 
        fprintf(stderr, "not enough argument in ip.config\n"); 
        return 0; 
    }
    printf("config is %d %d %d %s\n", node_cnt, round_interval, msg_size, dynamic_msg_size==1?"true":"false");

    sprintf(pem_file_name, "./pem/priv-%02d.pem", curr_node_idx);
    pri_key = eddsa_read_pri_from_pem(pem_file_name);
    for(int i = 0; i < node_cnt; i++){
        sprintf(pem_file_name, "./pem/pub-%02d.pem", i);
        peers_pub_key.push_back(eddsa_read_pub_from_pem(pem_file_name));
    }

    for(int i = 0; i < node_cnt; i++){
        if(!getline(ip_config, config_buf)) { fprintf(stderr, "ip.config ip error\n"); return 0; }
        NetAddr addr(config_buf);
        if(i == curr_node_idx) { curr_node_ip = addr; curr_node_ip_str = config_buf; }
        else curr_peers.push_back(addr);
    }

    ip_config.close();

    MsgNetwork<uint8_t>::Config config;
    config.max_msg_size(40*1024*1024);
    config.nworker(4);
    config.max_recv_buff_size(1000000);

    curr_node = std::make_unique<MyNet>(ec, 
                                        curr_node_ip_str, 
                                        curr_node_idx, 
                                        curr_peers, 
                                        node_cnt, 
                                        round_interval, 
                                        msg_size, 
                                        dynamic_msg_size,
                                        pri_key, 
                                        peers_pub_key,
                                        config);
    curr_node->start();
    curr_node->listen(curr_node_ip);
    curr_node->connectToPeers();

    auto shutdown = [&](int) {ec.stop();};
    salticidae::SigEvent ev_sigint(ec, shutdown);
    salticidae::SigEvent ev_sigterm(ec, shutdown);
    ev_sigint.add(SIGINT);
    ev_sigterm.add(SIGTERM);
    ec.dispatch();

    char eval_file_name[32];
    sprintf(eval_file_name, "eval/send2echo_%02d.eval", curr_node_idx);
    ofstream send2echo_time(eval_file_name);
    for(int i = 0; i < curr_node->send2echo.size(); i++) send2echo_time << i << ": " << curr_node->send2echo[i].count() << "\n";
    send2echo_time.close();

    sprintf(eval_file_name, "eval/send2fin_%02d.eval", curr_node_idx);
    ofstream send2fin_time(eval_file_name);
    for(int i = 0; i < curr_node->send2fin.size(); i++) send2fin_time << i << ": " << curr_node->send2fin[i].count() << "\n";
    send2fin_time.close();

    sprintf(eval_file_name, "eval/fin2fin_%02d.eval", curr_node_idx);
    ofstream fin2fin_time(eval_file_name);
    for(int i = 0; i < curr_node->fin2fin.size(); i++) fin2fin_time << i << ": " << curr_node->fin2fin[i].count() << "\n";
    fin2fin_time.close();

    sprintf(eval_file_name, "eval/send2delivered_%02d.eval", curr_node_idx);
    ofstream send2delivered_time(eval_file_name);
    for(int i = 0; i < curr_node->send2delivered.size(); i++) send2delivered_time << i << ": " << curr_node->send2delivered[i].count() << "\n";
    send2delivered_time.close();

    sprintf(eval_file_name, "eval/thruput_%02d.eval", curr_node_idx);
    ofstream thruput(eval_file_name);
    for(int i = 1; i < curr_node->round_duration.size(); i++){
        thruput << chrono::duration_cast<chrono::milliseconds>(curr_node->round_duration[i] - curr_node->round_duration[i-1]).count();
        thruput << " " << curr_node->tot_recv_send[i-1] << " ";
        thruput << curr_node->tot_recv_echo[i-1] << " ";
        thruput << curr_node->tot_recv_fin[i-1] << " ";
        thruput << curr_node->tot_recv_sup[i-1] << " ";
        thruput << curr_node->tot_sent_send[i-1] << " ";
        thruput << curr_node->tot_sent_echo[i-1] << " ";
        thruput << curr_node->tot_sent_fin[i-1] << " ";
        thruput << curr_node->tot_sent_sup[i-1] << "\n";
    }
    thruput.close();

    return 0;
}