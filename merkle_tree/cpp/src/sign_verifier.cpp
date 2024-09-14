#include <iostream>
#include <fstream>
#include <openssl/crypto.h>
#include "ecdsa_sign.hpp"
#include "eddsa_sign.hpp"
#include "IPP.hpp"

#define EDDSA
#define TXOUT "../tx_data/tx_out.tx"
#ifdef EDDSA
#define SIGN_OUT "EDDSA_OUT.tx"
#define KEY_SIZE 44
#else
#define SIGN_OUT "ECDSA_OUT.tx"
#define KEY_SIZE 88
#endif
using namespace std;

int
main()
{
    ifstream tx_in(TXOUT);
    ifstream sign_in(SIGN_OUT);
    string a_tx;
    unsigned char hash_buf[32], sign_buf[80], *pk_buf;
    char sign_len_buf[2];
    int sign_len;
    while(getline(tx_in, a_tx)){
        SHA256_wrapper((void*)a_tx.c_str(), a_tx.length(), hash_buf);

        sign_in.read((char*)sign_len_buf, 2);
        sign_len = (sign_len_buf[0] - '0') * 10 + sign_len_buf[1] - '0';

        sign_in.read((char*)sign_buf, sign_len);

        pk_buf = (unsigned char*)malloc(sizeof(char) * KEY_SIZE);
        sign_in.read((char*)pk_buf, KEY_SIZE);

        #ifdef EDDSA
            EVP_PKEY *edkey = eddsa_get_pubkey_from_byte((const unsigned char**)&pk_buf, KEY_SIZE); 
            if(!eddsa_verify_sign(hash_buf, 32, sign_buf, sign_len, edkey)){
                printf("incorrect signature\n");
            }
        #else
            EC_KEY *pk = ecdsa_get_pubkey_from_byte((const unsigned char**)&pk_buf, KEY_SIZE);
            if(!ecdsa_verify_sign((const unsigned char*)hash_buf, 32, (const unsigned char*)sign_buf, sign_len, pk) != 1){
                printf("incorrect signature\n");
            }
        #endif
    }
    printf("done\n");
}
