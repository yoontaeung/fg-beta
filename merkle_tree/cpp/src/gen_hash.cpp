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
#include "sha.hpp"

using namespace std;

#define HASH_LEN 32
int 
main()
{
    ofstream hash_out("../tx_data/tx_out.tx");
	unsigned char comm_buf[HASH_LEN];
    char input_str[64];
    int work_cnt;



    printf("input size of transactions : ");
    scanf("%d", &work_cnt);

	printf("%d input transactions", work_cnt);

    for(int i = 0 ; i < work_cnt; i++){
        sprintf(input_str, "hello world%d", i);
        // printf("%s", input_str);
        SHA256_wrapper((void*)input_str, strlen(input_str), comm_buf);
        hash_out.write((const char*)comm_buf, HASH_LEN);
    }

	hash_out.close();

	return 0;
}
