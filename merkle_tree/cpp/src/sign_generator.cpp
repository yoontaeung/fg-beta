#include <iostream>
#include <fstream>
#include <openssl/crypto.h>
#include "ecdsa_sign.hpp"
#include "eddsa_sign.hpp"
#include "sha.hpp"
#include <cstring>

#define TXCOUNT 400000
#define TXOUT "../tx_data/tx_out.tx"
#define EDDSA
#ifdef EDDSA
#define SIGN_OUT "../tx_data/EDDSA_OUT.tx"
#else
#define SIGN_OUT "../tx_data/ECDSA_OUT.tx"
#endif
#define TXMSG "{id : 2, jsonrpc :  2.0 , method :  account_signTransaction , params : [ {gas :  0x55555 , maxFeePerGas :  0x1234 , maxPriorityFeePerGas :  0x1234 , input :  0xabcd , nonce :  0x0 , to :  0x07a565b7ed7d7a678680a4c162885bedbb695fe0 , value :  0x1234 } ] }%010d"

using namespace std;

int
main()
{
    ofstream tx_out, sign_out;
    tx_out.open(TXOUT);
    sign_out.open(SIGN_OUT);

    unsigned char hash[32] = {0}; //, signature[100] = {0};
    char msg[500] = {0};
    for(int i = 0; i < TXCOUNT; i++){
        unsigned char *pk = NULL, *signature;
        sprintf(msg, TXMSG, i);
        tx_out << msg << endl; // tx

        SHA256_wrapper((void*)msg, strlen(msg), hash);

        #ifdef EDDSA
            EVP_PKEY *edkey = eddsa_gen_key();
            int signature_len = eddsa_do_sign((unsigned char*) hash, 32, (unsigned char**)&signature, edkey);
            sign_out << to_string(signature_len);
            sign_out.write((const char*)signature, signature_len);
            int key_len = eddsa_get_pubkey_to_byte(&pk, edkey);
            sign_out.write((const char*)pk, key_len);
        #else
            EC_KEY *eckey = ecdsa_gen_key();
            int signature_len = ecdsa_do_sign((unsigned char*)hash, 32, (unsigned char**)&signature, eckey);
            sign_out << to_string(signature_len); // sign length
            sign_out.write((const char*)signature, signature_len); // sign
            ecdsa_get_pubkey_to_byte(&pk, eckey);
            sign_out.write((const char*)pk, 88); // pk
        #endif

        #ifdef DEBUG
            print_hash(signature, signature_len);
            print_hash(pk, 88);
        #endif
    }
    printf("done gen %d tx\n", TXCOUNT);
}
