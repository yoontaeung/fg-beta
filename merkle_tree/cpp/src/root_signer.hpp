#ifndef ROOT_SIGNER_H
#define ROOT_SIGNER_H

#include "merkle_tree.hpp"
#include <semaphore.h>
#include <deque>
#include <openssl/crypto.h>
#include "eddsa.hpp"
#ifdef EDDSA
	#include "../src/eddsa.hpp"
	#define PK_SIZE 44
	#define INPUT_SIGN_FILE "../tx_data/EDDSA_OUT.tx"
	EVP_PKEY *edkey;
#else
	#include "../src/ecdsa.hpp"
	#define PK_SIZE 88
	#define INPUT_SIGN_FILE "../tx_data/ECDSA_OUT.tx"
	EC_KEY *eckey;
#endif

extern sem_t sem;
extern pthread_mutex_t channel_mutex;
extern deque<Work*> work_channel;
extern EVP_PKEY *edkey;
extern pthread_mutex_t fp_mutex;
extern ofstream proof_out;
extern atomic_int work_remaining;

void*
root_signer(void* arg)
{
    unsigned char* signature;
    char path_len[3];
    while(work_remaining != 0){
        sem_wait(&sem);
        // if(sem_trywait(&sem) != 0){ continue; }

        pthread_mutex_lock(&channel_mutex);
            Work* work = work_channel.front(); 
            work_channel.pop_front();
        pthread_mutex_unlock(&channel_mutex);

        signature = NULL;
        int sign_len = eddsa_do_sign(work->get_root_ptr(), HASH_LEN, &signature, edkey);

        pthread_mutex_lock(&fp_mutex);
            // sprintf(path_len, "%02d", work->get_path_cnt());
            proof_out.write((const char*)path_len, 2);
            proof_out.write((const char*)signature, sign_len);
            proof_out.write((const char*)work->get_root_ptr(), HASH_LEN);
            proof_out.write((const char*)work->get_leaf_ptr(), HASH_LEN);
            proof_out.write((const char*)work->get_path_ptr(), work->get_path_len());
            work_remaining--;
        pthread_mutex_unlock(&fp_mutex);
    }
}

#endif