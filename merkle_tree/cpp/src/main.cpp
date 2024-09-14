#include <stdio.h>
#include <string.h>
#include <iostream>
#include <fstream>
#include <chrono>
#include <deque>
#include <pthread.h>
#include <semaphore.h>
#include <openssl/crypto.h>
#include <fstream>
#include <signal.h> // pthread_kill()
#include <unistd.h> // sleep()
#include <atomic> // atomic_int
#include "merkle_tree.hpp"
#include "root_signer.hpp"
#include "eddsa.hpp"


#define INPUT_TX_FILE "../tx_data/tx_out.tx"
#define PROOF_OUT "../tx_data/signed_proof.tx"
#define CHILD_THRD_CNT 40

using namespace std;

/********** thread shared variables **********/
unordered_map<Node*, unsigned char*> leaf_map;
sem_t sem;
pthread_mutex_t channel_mutex = PTHREAD_MUTEX_INITIALIZER;
deque<Work*> work_channel;
EVP_PKEY *edkey;
pthread_mutex_t fp_mutex = PTHREAD_MUTEX_INITIALIZER;
ofstream proof_out;
atomic_int work_remaining;

int 
main()
{
	ifstream input_tx(INPUT_TX_FILE);
	string a_tx;
	pthread_t pid[CHILD_THRD_CNT];
	Tree *tree = new Tree(leaf_map, &sem, &channel_mutex, &work_channel);
	unsigned char comm_buf[HASH_LEN];
	unsigned char *pk = NULL;

	printf("input tx size: ");
	scanf("%d", &work_remaining);
	cout << work_remaining << endl;
	proof_out.open(PROOF_OUT);

	edkey = eddsa_gen_key();
	int key_len = eddsa_get_pubkey_to_byte(&pk, edkey);
	proof_out.write((const char*)pk, key_len);
	
	if(sem_init(&sem, 0, 0) != 0){
		perror("semaphore failed to init\n");
		return -1;
	}

	for(int i = 0; i < CHILD_THRD_CNT; i++){
		pthread_create(&(pid[i]), NULL, &root_signer, NULL);
	}
	usleep(1000000); 

	auto start = chrono::high_resolution_clock::now();

	while(getline(input_tx, a_tx)){
		SHA256_wrapper((void*)a_tx.c_str(), strlen(a_tx.c_str()), comm_buf);
		tree->append_leaf((unsigned char*) comm_buf);
	}
	tree->print_root();

	// lock may be required here... 
	auto stop_insert = chrono::high_resolution_clock::now();
	auto duration_insert = chrono::duration_cast<chrono::milliseconds>(stop_insert - start);
	cout << "tree insertion took " << duration_insert.count() << " millisecond\n"
			<< work_remaining << " works remain\n";
	while(1){
		if(work_remaining == 0){ usleep(10); break; }
		else { 
			// cout << work_remaining << " works remain\n"; // 100,000 - 87,536 = 12,464
			usleep(10000); 
		}
	}
	// for(int i = 0; i < CHILD_THRD_CNT; i++){ pthread_join(pid[i], NULL); }
	// proof_out.close();

	auto stop = chrono::high_resolution_clock::now();
	auto duration = chrono::duration_cast<chrono::milliseconds>(stop - start);
	cout << "it took " << duration.count() << " millisecond\n";

	return 0;
}
