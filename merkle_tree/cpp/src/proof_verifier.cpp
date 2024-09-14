#include <iostream>
#include <fstream>
#include <openssl/crypto.h>
#include "ecdsa.hpp"
#include "eddsa.hpp"
#include "sha.hpp"

#define SIGN_IN "../tx_data/signed_proof.tx"
#define EDDSA
#define KEY_SIZE 44
#define WORK_CNT 100000
#define SIGN_LEN 64
#define HASH_LEN 32
using namespace std;

int
main()
{
    ifstream sign_in(SIGN_IN);
    unsigned char *pk_buf, sign_buf[SIGN_LEN], root_buf[HASH_LEN];
    unsigned char leaf_buf[HASH_LEN], path_buf[HASH_LEN], concat_buf[HASH_LEN*2];
    char path_cnt_buf[2];
    int path_cnt;

    pk_buf = (unsigned char*)malloc(sizeof(unsigned char) * KEY_SIZE);

    sign_in.read((char*)pk_buf, KEY_SIZE);
    EVP_PKEY *edkey = eddsa_get_pubkey_from_byte((const unsigned char**)&pk_buf, KEY_SIZE); 

    for(int i = 0; i < WORK_CNT; i++){
        sign_in.read((char*)path_cnt_buf, 2);
        path_cnt = (path_cnt_buf[0] - '0') * 10 + path_cnt_buf[1] - '0';
        sign_in.read((char*)sign_buf, SIGN_LEN);
        sign_in.read((char*)root_buf, HASH_LEN);
        sign_in.read((char*)leaf_buf, HASH_LEN);
        for(int j = 0; j < path_cnt; j++){
            sign_in.read((char*)path_buf, HASH_LEN);
            for(int k = 0; k < HASH_LEN; k++){
                concat_buf[k] = path_buf[k];
                concat_buf[k+HASH_LEN] = leaf_buf[k];
            }
            SHA256_wrapper(concat_buf, HASH_LEN*2, leaf_buf);
        }

        for(int j = 0; j < HASH_LEN; j++){ 
            if(root_buf[j] != leaf_buf[j]){
                printf("wrong root!\n");
                for(int j = 0; j < HASH_LEN; j++){ printf("%02x", root_buf[j]); }
                printf("\n");
                for(int j = 0; j < HASH_LEN; j++){ printf("%02x", leaf_buf[j]); }
                printf("\n");
                printf("at %d th root\n", i);
                return 0;
            }
        }
        if(!eddsa_verify_sign(root_buf, HASH_LEN, sign_buf, SIGN_LEN, edkey)){
            printf("wrong signature!\n");
            return 0;
        }
    }
    printf("all correct\n");
}
