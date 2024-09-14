#ifndef MSGSTRUCT_H
#define MSGSTRUCT_H
#include "macro.hpp"
#include "salticidae/stream.h"

#define SIGN_LEN 64 // byte

using salticidae::DataStream;
using salticidae::htole;
using salticidae::letoh;
using namespace std;

/* TODO : struct is allocated in stack? stack overflow might happen?*/
struct MsgACK {
    static const uint8_t opcode = 0x0;
    DataStream serialized;
    int sender_idx;
    MsgACK(int sender_idx) { serialized << sender_idx; }
    MsgACK(DataStream &&s) { s >> sender_idx; }
    void print() { print_debug("msg ack from %02dn", sender_idx); }
};

struct MsgSEND {
    static const uint8_t opcode = 0x1;
    DataStream serialized;
    string ip, payload;
    int sender_idx, round_number;
    
    MsgSEND(const string &ip, int sender_idx, 
            int round_number, const string &payload) 
    {
        serialized << sender_idx << round_number;
        serialized << htole((uint32_t)ip.length());
        serialized << ip << payload;
    }
    MsgSEND(DataStream &&s) {
        uint32_t len;
        s >> sender_idx >> round_number >> len;
        len = letoh(len);
        ip = string((const char *)s.get_data_inplace(len), len);
        len = s.size();
        payload = string((const char *)s.get_data_inplace(len), len);
    }
    void print(){
        print_debug("ip %s, node_index %d, round# %d, payload %s, payload len %ld\n", 
                ip.c_str(), sender_idx, round_number, payload.c_str(), payload.length());
    }
    void print_no_payload(){
        print_debug("ip %s, node_index %d, round# %d, payload len %ld\n", 
                ip.c_str(), sender_idx, round_number, payload.length());
    }
    int size(){
        return ip.length()+payload.length()+8;
    }
};

struct MsgECHO {
    static const uint8_t opcode = 0x2;
    DataStream serialized;
    string ip, payload;
    unsigned char signature[SIGN_LEN];
    int sender_idx, round_number;
    
    MsgECHO(const string &ip, int sender_idx, 
            int round_number, const string &payload, unsigned char* signature)
    {
        serialized << sender_idx << round_number;
        serialized << htole((uint32_t)ip.length());
        serialized << ip;
        serialized.put_data(signature, signature+SIGN_LEN); 
        serialized << payload;
    }
    MsgECHO(DataStream &&s) {
        uint32_t len;
        s >> sender_idx >> round_number >> len;
        len = letoh(len);
        ip = string((const char *)s.get_data_inplace(len), len);
        for(int _i = 0; _i < SIGN_LEN; _i++) {
            signature[_i] = *(s.get_data_inplace(1));
        }
        len = s.size();
        payload = string((const char *)s.get_data_inplace(len), len);
    }
    void print(){
        print_debug("ip %s, node_index %d, round# %d, payload %s, payload len %ld\n", 
                ip.c_str(), sender_idx, round_number, payload.c_str(), payload.length());
    }
    void print_no_payload(){
        print_debug("ip %s, node_index %d, round# %d, payload len %ld\n", 
                ip.c_str(), sender_idx, round_number, payload.length());
    }
    int size(){
        return ip.length()+payload.length()+8+SIGN_LEN;
    }
};

struct MsgFIN {
    static const uint8_t opcode = 0x3;
    DataStream serialized;
    string ip, payload;
    vector<pair<int, vector<unsigned char> > > signature_list;
    int sender_idx, round_number, signature_cnt;
    
    MsgFIN(const string &ip, int sender_idx, int round_number, const string &payload, 
                vector<pair<int, vector<unsigned char> > > *signature_list)
    {
        serialized << sender_idx << round_number;
        serialized << htole((uint32_t)ip.length());
        serialized << ip << (int)signature_list->size();
        for(int _i = 0; _i < signature_list->size(); _i++){
            serialized << signature_list->at(_i).first;
            serialized.put_data(signature_list->at(_i).second.data(), signature_list->at(_i).second.data()+SIGN_LEN);
        }
        serialized << payload;
    }

    MsgFIN(DataStream &&s) {
        uint32_t len; // signature_list->size() might not be 32bit uint?
        int signature_node_index;
        s >> sender_idx >> round_number >> len;
        len = letoh(len);
        ip = string((const char *)s.get_data_inplace(len), len);
        s >> signature_cnt;
        signature_list.reserve(signature_cnt);
        for(int _i = 0; _i < signature_cnt; _i++){
            vector<unsigned char> signature(SIGN_LEN);
            s >> signature_node_index;
            for(int _j = 0; _j < SIGN_LEN; _j++) {
                signature[_j] = *(s.get_data_inplace(1));
            }
            signature_list.push_back(make_pair(signature_node_index, signature));
        }
        len = s.size();
        payload = string((const char *)s.get_data_inplace(len), len);
    }
    void print(){
        print_debug("ip %s, node_index %d, round# %d, payload %s, payload len %ld\n", 
                ip.c_str(), sender_idx, round_number, payload.c_str(), payload.length());
    }
    void print_no_payload(){
        print_debug("ip %s, node_index %d, round# %d, payload len %ld\n", 
                ip.c_str(), sender_idx, round_number, payload.length());
    }
    int size(){
        return ip.length()+payload.length()+12+signature_cnt*(4+SIGN_LEN);
    }
};

struct MsgSUP {
    static const uint8_t opcode = 0x4;
    DataStream serialized;
    string ip, payload;
    vector<pair<int, vector<unsigned char> > > signature_list;
    int sender_idx, round_number, signature_cnt, original_sender;
    
    MsgSUP(const string &ip, int sender_idx, int round_number, const string &payload, 
                vector<pair<int, vector<unsigned char> > > *signature_list, int original_sender)
    {
        serialized << sender_idx << round_number << original_sender;
        serialized << htole((uint32_t)ip.length());
        serialized << ip << (int)signature_list->size();
        for(int _i = 0; _i < signature_list->size(); _i++){
            serialized << signature_list->at(_i).first;
            serialized.put_data(signature_list->at(_i).second.data(), signature_list->at(_i).second.data()+SIGN_LEN);
        }
        serialized << payload;
    }

    MsgSUP(DataStream &&s) {
        uint32_t len; // signature_list->size() might not be 32bit uint?
        int signature_node_index;
        s >> sender_idx >> round_number >> original_sender >> len;
        len = letoh(len);
        ip = string((const char *)s.get_data_inplace(len), len);
        s >> signature_cnt;
        signature_list.reserve(signature_cnt);
        for(int _i = 0; _i < signature_cnt; _i++){
            vector<unsigned char> signature(SIGN_LEN);
            s >> signature_node_index;
            for(int _j = 0; _j < SIGN_LEN; _j++) {
                signature[_j] = *(s.get_data_inplace(1));
            }
            signature_list.push_back(make_pair(signature_node_index, signature));
        }
        len = s.size();
        payload = string((const char *)s.get_data_inplace(len), len);
    }
    void print(){
        print_debug("ip %s, node_index %d, round# %d, original sender %d, payload %s, payload len %ld\n", 
                ip.c_str(), sender_idx, round_number, original_sender, payload.c_str(), payload.length());
    }
    void print_no_payload(){
        print_debug("ip %s, node_index %d, round# %d, original sender %d, payload len %ld\n", 
                ip.c_str(), sender_idx, round_number, original_sender, payload.length());
    }
    int size(){
        return ip.length()+payload.length()+20+(signature_cnt*(4+SIGN_LEN)); // Adjusted size for additional int
    }
};

#endif