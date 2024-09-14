#ifndef NET_H
#define NET_H
#include <ctime>
#include <sys/time.h>
#include <fstream>
#include <atomic>
#include <set>
#include <utility>
#include <string>
#include <unordered_map>
#include "salticidae/msg.h"
#include "salticidae/network.h"
#include "macro.hpp"
#include "msg_struct.hpp"
#include "eddsa.hpp"

// #define SEND_INTERVAL 1 // second
// #define PAYLOAD_SIZE 1000000 // 1000000 byte = 1MB
#define WAIT_FOR_PEERS_CONNECTION 5 // second

using salticidae::TimerEvent;
using salticidae::MsgNetwork;
using salticidae::NetAddr;
using std::placeholders::_1;
using std::placeholders::_2;
using MsgNetworkByteOp = MsgNetwork<uint8_t>;
using namespace std;

const uint8_t MsgACK::opcode;
const uint8_t MsgSEND::opcode;
const uint8_t MsgECHO::opcode;
const uint8_t MsgFIN::opcode;
const uint8_t MsgSUP::opcode;
#define MSG_SIZE_SIZE 6
int msg_size_ind = 4, round_counter = 0;
// 100kb, 500kb, 1mb, 3mb, 5mb, 7mb, 10mb, 13mb
const int msg_size[MSG_SIZE_SIZE] = {1000000, 3000000, 5000000, 7000000, 10000000, 13000000};


chrono::milliseconds calc_elapsed(chrono::steady_clock::time_point start) {
    return chrono::duration_cast<chrono::milliseconds>(chrono::steady_clock::now() - start);
}

/* credit: http://stackoverflow.com/a/41381479/544806 */
const std::string get_current_datetime() {
    char fmt[64], buf[64];
    struct timeval tv;
    gettimeofday(&tv, nullptr);
    struct tm *tmp = localtime(&tv.tv_sec);
    strftime(fmt, sizeof fmt, "%Y-%m-%d %H:%M:%S.%%06u", tmp);
    snprintf(buf, sizeof buf, fmt, tv.tv_usec);
    return std::string(buf);
}

struct MyNet: public MsgNetworkByteOp {
    const string ip;
    int node_id, peer_cnt, f_cnt, round_interval, payload_size;
    /*thruput*/
    atomic_int recv_send, recv_echo, recv_fin, recv_sup, sent_send, sent_echo, sent_fin, sent_sup;
    vector<int> tot_recv_send, tot_recv_echo, tot_recv_fin, tot_recv_sup;
    vector<int> tot_sent_send, tot_sent_echo, tot_sent_fin, tot_sent_sup;
    vector<chrono::steady_clock::time_point> round_duration;
    vector<chrono::steady_clock::time_point> round_start, fin_start;
    vector<chrono::milliseconds> send2echo, send2fin, fin2fin, send2delivered;

    int dynamic_msg_size;
    string payload;
    // bool isSelfConnAdded = false;
    // conn_t selfConn;

    unordered_set<conn_t> conns;
    unordered_map<int, conn_t> conns_map;
    vector<NetAddr> peers;
    vector<int> echo_ack_list, recved_fin_cnt, recved_sup_cnt;
    vector<bool> sent_final_list, sent_sup_list, connected_peers;
    vector<EVP_PKEY*> peers_pub_key;
    vector<vector<pair<int, vector<unsigned char> > > > signature_lists;
    // peer > {(payload_0), (payload_1), ...}
    vector<vector<string> > tx_lists;
    vector<vector<bool> > delivered;
    vector<vector<int> > sup_message_count;
    TimerEvent timer;
    EVP_PKEY *pri_key;

    MyNet(const salticidae::EventContext &ec, const string ip, 
                    int node_id,  const vector<NetAddr> &peers, 
                    int peer_cnt, int round_interval, int payload_size, int dynamic_msg_size,
                    EVP_PKEY *pri_key, vector<EVP_PKEY*> &peers_pub_key, MsgNetwork<uint8_t>::Config &config):
            ip(ip), 
            peers(peers), 
            node_id(node_id), 
            round_interval(round_interval),
            payload_size(payload_size),
            dynamic_msg_size(dynamic_msg_size),
            peer_cnt(peer_cnt),
            pri_key(pri_key), 
            peers_pub_key(peers_pub_key),
            MsgNetwork<uint8_t>(ec, config),
            timer(ec, [this](TimerEvent &) {start_round(); })
            // isSelfConnAdded(false)
    {
        connected_peers.reserve(peer_cnt);
        for(int i = 0; i < peer_cnt; i++) connected_peers[i] = false;
        f_cnt = (peer_cnt - 1) / 3;
        timer.add(WAIT_FOR_PEERS_CONNECTION);
        recv_send = recv_echo = recv_fin = recv_sup = sent_send = sent_echo = sent_fin = sent_sup = 0;

        if(dynamic_msg_size == 1) 
            payload = string(msg_size[msg_size_ind], (char)('0'+node_id)); 
        else payload = string(payload_size, (char)('0'+node_id));

        // taeung: make a tx_lists for each peer. Store a tx_list for every peer for each round. 
        // taeung: make a delivered double vector. To see if a message is delivered for each peer and round. 
        for(int i=0; i<peer_cnt; i++) {
            tx_lists.push_back(vector<string>());
            delivered.push_back(vector<bool>());
            sup_message_count.push_back(vector<int>());
        }

        reg_handler(salticidae::generic_bind(&MyNet::on_receive_ack, this, _1, _2));
        reg_handler(salticidae::generic_bind(&MyNet::on_receive_send, this, _1, _2));
        reg_handler(salticidae::generic_bind(&MyNet::on_receive_echo, this, _1, _2));
        reg_handler(salticidae::generic_bind(&MyNet::on_receive_fin, this, _1, _2));
        reg_handler(salticidae::generic_bind(&MyNet::on_receive_sup, this, _1, _2));

        reg_conn_handler([this](const ConnPool::conn_t &conn, bool connected) {
            if (connected){
                print_debug("[%s] connected to %s\n", this->ip.c_str(), string(*conn).c_str());
                conn_t conn_wrapper = salticidae::static_pointer_cast<Conn>(conn);
                send_msg(MsgACK(this->node_id), conn_wrapper);
            }
            return true;
        });
    }

    void append_new_thruput(){
        // race exists here!!
        tot_recv_send.push_back(recv_send); recv_send = 0;
        tot_recv_echo.push_back(recv_echo); recv_echo = 0;
        tot_recv_fin.push_back(recv_fin); recv_fin = 0; 
        tot_recv_sup.push_back(recv_sup); recv_sup = 0;
        tot_sent_send.push_back(sent_send); sent_send = 0;
        tot_sent_echo.push_back(sent_echo); sent_echo = 0; 
        tot_sent_fin.push_back(sent_fin); sent_fin = 0;
        tot_sent_sup.push_back(sent_sup); sent_sup = 0;
        round_duration.push_back(chrono::steady_clock::now());
    }

    int idx() { return node_id; }
    
    void connectToPeers() {
        for (const auto &peer : peers) connect_sync(peer);
    }
    
    void increase_ack_echo(int round) { 
        if(round <= echo_ack_list.size()) echo_ack_list[round]++; 
    }
    
    bool received_enough_echo(int round) { 
        // TODO: change the threshold 
        // if(round <= echo_ack_list.size() && echo_ack_list[round] > (peer_cnt+f_cnt)/2)
        // wait for all peers' echo
        if(round <= echo_ack_list.size() && echo_ack_list[round] >= (peer_cnt)) {
            return true;
        }
        else return false;
    }
    
    void append_new_round(){
        echo_ack_list.push_back(0);
        sent_final_list.push_back(false);
        sent_sup_list.push_back(false);
        signature_lists.push_back(vector<pair<int, vector<unsigned char> > >());
    }
    
    void append_new_signature(int round, int node_id, vector<unsigned char>&signature){
        signature_lists[round].push_back(make_pair(node_id, signature));
        increase_ack_echo(round);
    }

    int get_next_round(){
        if(echo_ack_list.size() > 0) return echo_ack_list.size()-1;
        else return -1;
    }

    EVP_PKEY* get_sk() { return pri_key; }
    EVP_PKEY* get_peer_pk(int index) { return peers_pub_key[index]; }

    bool has_sent_final(int round){ return sent_final_list[round]; }
    void set_sent_final(int round){ sent_final_list[round] = true; }

    bool has_sent_sup(int round){ return sent_sup_list[round]; }
    void set_sent_sup(int round){ sent_sup_list[round] = true; }

    /*
    void addSelfConn(int nid) {
        if(!isSelfConnAdded) {
            // Splitting the ip string into address and port
            print_info("ip: %s", ip.c_str());
            std::string ip_addr_port = ip.c_str(); // Assuming ip is a string that contains "143.248.47.28:12341"
            size_t colon_pos = ip_addr_port.find(':');
            if (colon_pos != std::string::npos) {
                // Extract the IP address and port
                std::string addr = ip_addr_port.substr(0, colon_pos);
                int port = std::stoi(ip_addr_port.substr(colon_pos + 1));

                // Create NetAddr with separated IP and port
                NetAddr selfAddr(addr, port);
                selfConn = connect_sync(selfAddr);
                conns_map.insert(make_pair(node_id, selfConn));
                connected_peers[node_id] = true;
                isSelfConnAdded = true;
            } else {
                print_err("Invalid IP address and port format");
            }
        }
    }
    */

    void start_round(){
        vector<unsigned char>vec_u8(SIGN_LEN);
        unsigned char* signature = NULL;
        print_err("%s msg size : %d, %d, round : %d", get_current_datetime().c_str(), payload.length(), msg_size[msg_size_ind], round_counter);
        round_counter++;
        if(dynamic_msg_size == 1 && round_counter == 60 && msg_size_ind+1 < MSG_SIZE_SIZE){
            round_counter = 0;
            msg_size_ind++;
            payload = string (msg_size[msg_size_ind], (char)('0'+node_id));
        }
        // taeung: to check the different payload for every round.
        payload = string(payload_size, (char)('0'+round_counter));

        append_new_round();
        round_start.push_back(chrono::steady_clock::now());
        send2echo.push_back(chrono::milliseconds());

        // taeung: only a specific node(i.e., 1) sends a message. 
        // TODO: Change the payload string

        // if(!isSelfConnAdded) addSelfConn(node_id);
        
        for (const auto &peer_conn : conns) {
            // if (peer_conn != selfConn) send_msg(MsgSEND(ip, node_id, get_next_round(), payload), peer_conn);
            send_msg(MsgSEND(ip, node_id, get_next_round(), payload), peer_conn);
        }
        // send_msg(MsgSEND(ip, node_id, get_next_round(), payload), selfConn);

        sent_send += conns.size() * payload.length();

        // taeung: no need to put its own signature because it sends a SEND message to itself. 
        eddsa_do_sign((unsigned char*)payload.c_str(), payload.length(), &signature, pri_key);
        for(int i = 0; i < SIGN_LEN; i++){ vec_u8[i] = signature[i]; }
        append_new_signature(get_next_round(), node_id, vec_u8);
        tx_lists[node_id].push_back(payload);

        free(signature);
        append_new_thruput();
        timer.add(round_interval);
    }

    void on_receive_ack(MsgACK &&msg, const MyNet::conn_t &conn){
        print_debug("received ack from %02d", msg.sender_idx);
        if(connected_peers[msg.sender_idx] == false){
            conn_t conn_wrapper = salticidae::static_pointer_cast<Conn>(conn);
            conns.insert(conn_wrapper);
            conns_map.insert(make_pair(msg.sender_idx, conn_wrapper));
            connected_peers[msg.sender_idx] = true;
        }
    }

    void on_receive_send(MsgSEND &&msg, const MyNet::conn_t &conn) {
        int msg_sender = msg.sender_idx, msg_r = msg.round_number;
        print_debug("[ %02d ] received send from %02d at round %d", node_id, msg_sender, msg_r);
        msg.print_no_payload();

        unsigned char* signature = NULL;
        eddsa_do_sign((unsigned char*)msg.payload.c_str(), msg.payload.length(), &signature, get_sk());
        MsgECHO echo_msg(ip, node_id, msg.round_number, "", signature);
        send_msg(echo_msg, conn);
        recv_send += msg.size(); 
        sent_echo += echo_msg.serialized.size();
        // TODO: round check
        tx_lists[msg_sender].push_back(msg.payload);
        
        free(signature);
    }

    void on_receive_echo(MsgECHO &&msg, const MyNet::conn_t &conn) {
        int msg_sender = msg.sender_idx, msg_r = msg.round_number;
        print_debug("[ %02d ] received echo from %02d round %02d", node_id, msg.sender_idx, msg.round_number);
        msg.print_no_payload();
        recv_echo += msg.size();

        if(eddsa_verify_sign((const unsigned char*)msg.payload.c_str(), msg.payload.length(), 
                                msg.signature, SIGN_LEN, get_peer_pk(msg_sender)))
        {
            print_debug("[ %d ] ECHO:: correct signature from %d\n", node_id, msg_sender);

            if(msg_r < signature_lists.size()){
                vector<unsigned char>vec_u8(SIGN_LEN);
                for(int i = 0; i < SIGN_LEN; i++){ vec_u8[i] = msg.signature[i]; }
                append_new_signature(msg_r, msg_sender, vec_u8);
            } 
            else print_err("[ %d ] not enough vector space signature lists\n", node_id);
                
            if(received_enough_echo(msg_r) && !has_sent_final(msg_r)){
                set_sent_final(msg_r);
                MsgFIN final_msg(ip, node_id, msg_r, "", &(signature_lists[msg_r])); 
                // send to self
                // send_msg(MsgFIN(ip, node_id, msg_r, "", &(signature_lists[msg_r])), selfConn);
                // send_msg(final_msg, selfConn);

                // send to peers
                for(const auto &peer_conn : conns) {
                    // if (peer_conn != selfConn) send_msg(final_msg, peer_conn);
                    send_msg(final_msg, peer_conn);
                }
                sent_fin += ((conns.size() + 1) * final_msg.serialized.size());

                MsgSUP sup_msg(ip, node_id, msg_r, "", &(signature_lists[msg_r]), node_id);
                for(const auto &peer_conn : conns) {
                    send_msg(sup_msg, peer_conn);
                }

                send2echo[msg_r] = calc_elapsed(round_start[msg_r]);
                check_all_fin_arrived(msg_r);
            }
        } 
        else print_err("[ %d ] incorrect signature from %d\n", node_id, msg_sender);
    }

    void on_receive_fin(MsgFIN &&msg, const MyNet::conn_t &conn) {
        int msg_sender = msg.sender_idx, msg_r = msg.round_number;
        print_debug("%s [ %02d ] received final from [ %02d ] round %02d with %02d signs", 
                    get_current_datetime().c_str(), node_id, msg.sender_idx, msg.round_number, msg.signature_cnt);
        
        std::set<int> nodes_in_signature_list;

        for (const auto &sig : msg.signature_list) {
            nodes_in_signature_list.insert(sig.first); // taeung: node_id in signature_list
        }

        // taeung: Print out nodes not included in the signature_list among all peers
        print_debug("Nodes not included in signature_list for round %d: ", msg_r);
        for (int i = 0; i < peer_cnt; ++i) {
            if (nodes_in_signature_list.find(i) == nodes_in_signature_list.end()) {
                print_debug("%d ", i);
                auto iter = conns_map.find(i);
                if (iter != conns_map.end())
                    print_debug("%d conns map", iter->second->get_addr().port);
            }
        }

        msg.print_no_payload();

        recv_fin += msg.size();

        if(msg.signature_cnt < (2 * f_cnt)) { 
            print_err("[ %d ] FIN:: not enought signature from %d\n", node_id, msg.sender_idx); 
            return; 
        }

        for(int i = 0; i < msg.signature_list.size(); i++){
            if(eddsa_verify_sign((const unsigned char*)msg.payload.c_str(), msg.payload.length(), 
                            msg.signature_list[i].second.data(), SIGN_LEN, get_peer_pk(msg.signature_list[i].first)))
            {
                // TODO: check 2f+1 sigs in signature_list
                print_debug("[ %d ] FIN:: correct signature from %d\n", node_id, msg.signature_list[i].first);
                // check_all_fin_arrived(msg_r);
            }
            else {
                print_err("[ %d ] incorrect signature from %d\n", node_id, msg.signature_list[i].first); // do something?
                return;
            }
        }
        // TODO: check 2f+1 sigs in signature_list
        MsgSUP sup_msg(ip, node_id, msg_r, "", &(msg.signature_list), msg_sender);
        for(const auto &peer_conn : conns){
            send_msg(sup_msg, peer_conn);
        } 
        // set_sent_sup(msg_r);
        sent_sup += conns.size() * sup_msg.serialized.size();
        // check_all_fin_arrived(msg.round_number);
        // do something?
    }

    void on_receive_sup(MsgSUP &&msg, const MyNet::conn_t &conn) {
        int msg_sender = msg.sender_idx, msg_r = msg.round_number;
        // TODO: send a SUP message with payload to nodes missing in Sigs
        // TODO: write a logic for SUP. only 2f+1 for optimistic case.
        // taeung: Assume that it has its own SUP message. So there should be n-1 SUP messages in optimisitic case. 
        print_debug("%s [ %02d ] received sup from [ %02d ] round %02d with %02d signs. origin from %d", 
                    get_current_datetime().c_str(), node_id, msg.sender_idx, msg.round_number, msg.signature_cnt, msg.original_sender);
        msg.print_no_payload();

        recv_sup += msg.size();

        if(msg.signature_cnt < (2 * f_cnt)) { 
            print_err("[ %d ] not enough signature from %d\n", node_id, msg.sender_idx); 
            return; 
        }

        for(int i = 0; i < msg.signature_list.size(); i++){
            if(eddsa_verify_sign((const unsigned char*)msg.payload.c_str(), msg.payload.length(), 
                            msg.signature_list[i].second.data(), SIGN_LEN, get_peer_pk(msg.signature_list[i].first)))
            {
                print_debug("[ %d ] SUP:: correct signature from %d\n", node_id, msg.signature_list[i].first);
            }
            else {
                print_err("[ %d ] incorrect signature from %d\n", node_id, msg.signature_list[i].first); // do something?
                return;
            }
        }

        // Ensure the vector for each peer is initialized for the current round
        if (msg.original_sender < sup_message_count.size()) {
            while (sup_message_count[msg.original_sender].size() <= msg_r) {
                sup_message_count[msg.original_sender].push_back(0);
            }
            // Increment the count for the original sender of the message in the current round
            sup_message_count[msg.original_sender][msg_r]++;
        }

        // taeung: Add a msg.original_sender in struct MsgSUP to track the original sender of a message. 
        print_debug("sup msg count %d", sup_message_count[msg.original_sender][msg_r]);
        if(sup_message_count[msg.original_sender][msg_r] >= (peer_cnt-1)) {
            // delivered[msg.original_sender].push_back(true);
            print_info("Message payload %d for round %d from sender %d is Delivered!", tx_lists[msg.original_sender][msg_r].length(), msg_r, msg.original_sender);
            if(msg.original_sender == node_id) my_sup_delivered(msg.round_number);
        }
        // do something?
    }

    void check_all_fin_arrived(int msg_r){
            while(recved_fin_cnt.size() <= msg_r){
                recved_fin_cnt.push_back(0);
                send2fin.push_back(chrono::milliseconds());
                fin2fin.push_back(chrono::milliseconds());
                fin_start.push_back(chrono::time_point<chrono::steady_clock>());
            }
            recved_fin_cnt[msg_r]++;
            if(recved_fin_cnt[msg_r] == 1) fin_start[msg_r] = chrono::steady_clock::now();
            else if(recved_fin_cnt[msg_r] == peer_cnt) {
                send2fin[msg_r] = calc_elapsed(round_start[msg_r]);
                fin2fin[msg_r] = calc_elapsed(fin_start[msg_r]);
            }
    }

    // TODO
    void my_sup_delivered(int msg_r){
        while(send2delivered.size() <= msg_r) send2delivered.push_back(chrono::milliseconds());
        send2delivered[msg_r] = calc_elapsed(round_start[msg_r]);
    }
}; // end of struct MyNet

#endif