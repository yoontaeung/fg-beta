#ifndef MERKLE_TREE_H
#define MERKLE_TREE_H
#define HASH_LEN 32
#include <cstring>
#include <iostream>
#include <unordered_map>
#include "sha.hpp"
using namespace std;

const int pow_of_2[30] ={1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192,
					16384, 32768, 65536, 131072, 262144, 524288, 1048576, 2097152, 4194304, 
                    8388608, 16777216, 33554432, 67108864, 134217728, 268435456, 536870912};
#ifdef POI
    extern ofstream poi;
#endif
#ifndef SHAHPP
#include "sha.hpp"
#endif
class Node;

class Work {
    private:
        unsigned char root[HASH_LEN];
        unsigned char leaf[HASH_LEN];
        unsigned char *path;
        unsigned int path_cnt;
    public:
        Work() {}
        Work(int path_size);
        void append_path(Node* path);
        void add_root(Node* root);
        void add_leaf(Node* leaf);
        void print_leaf();
        void print_root();
        void print_path();
        inline unsigned char* get_root_ptr() { return this->root; }
        inline unsigned char* get_path_ptr() { return this->path; }
        inline unsigned char* get_leaf_ptr() { return this->leaf; }
        inline int get_path_len() { return (this->path_cnt) * HASH_LEN; }
        inline int get_path_cnt() { return this->path_cnt; }
};

class Node{
    public:
        Node *left = NULL, *right = NULL;
        unsigned char hash[HASH_LEN] = {0};
        Node();
        Node(Node *new_left, Node *new_right);
        Node(const unsigned char *str);
        void print_hash();
        void recompute_hash();
        Node* append_leaf(int leaf_cnt, int index, Node* new_leaf);
        Node* append_leaf(int height, Node* new_leaf, Work* node_path);
};

class Tree
{
    private:
        unsigned int leaf_cnt;
        unsigned int pow_2_ind;
        Node* root;
        unordered_map<Node*, unsigned char*> leaf_map;
        deque<Work*> *work_channel;
        sem_t* sem;
        pthread_mutex_t *channel_mutex;
    public:
        Tree(unordered_map<Node*, unsigned char*> leaf_map, 
               sem_t *sem, 
               pthread_mutex_t *channel_mutex, 
               deque<Work*> *work_channel
            );
        void append_leaf (const unsigned char* comm);
        void print_root() { this->root->print_hash(); }
};

/*********** Work ***********/
Work::Work(int path_size)
{
    // printf("work::work, pathsize : %d\n", path_size);
    this->path = (unsigned char*)malloc(sizeof(unsigned char) * (path_size+1) * HASH_LEN);
    path_cnt = 0;
}

void
Work::append_path(Node* path)
{
    // printf("work::append_path, path_cnt %d\n", this->path_cnt);
    for(int i = 0; i < HASH_LEN; i++){
        this->path[(HASH_LEN*this->path_cnt) + i] = path->hash[i];
    }
    this->path_cnt++;
}

void 
Work::add_leaf(Node* leaf)
{
    for(int i = 0; i < HASH_LEN; i++){
        this->leaf[i] = leaf->hash[i];
    }
}
void 
Work::add_root(Node* root)
{
    for(int i = 0; i < HASH_LEN; i++){
        this->root[i] = root->hash[i];
    }
}

void
Work::print_leaf()
{
    for(int i = 0; i < HASH_LEN; i++)
        printf("%02x", this->leaf[i]);
    printf("\n");
}
void
Work::print_root()
{
    for(int i = 0; i < HASH_LEN; i++)
        printf("%02x", this->root[i]);
    printf("\n");
}

void
Work::print_path()
{
    for(int j = 0; j < this->path_cnt; j++){
        for(int i = 0; i < HASH_LEN; i++){
            printf("%02x", this->path[j*HASH_LEN+i]);
        }
        printf("\n");
    }
}

/*********** Node ***********/

Node::Node() 
{
    this->left = this->right = NULL;
    for(int i = 0; i < HASH_LEN; i++)
        this->hash[i] = 0;
}
Node::Node(Node *new_left, Node *new_right)
{
    this->left = new_left;
    this->right = new_right;
    this->recompute_hash();
}
Node::Node(const unsigned char *str) {
    this->left = this->right = NULL;
    for(int i = 0; i < HASH_LEN; i++)
        this->hash[i] = str[i];
}
void
Node::print_hash()
{
    for(int i = 0; i < HASH_LEN; i++)
        printf("%02x ", this->hash[i]);
    printf("\n");
}

void
Node::recompute_hash()
{
    char concat_hash[HASH_LEN*2] = {0};
    for(int i = 0; i < HASH_LEN; i++){
        concat_hash[i] = this->left->hash[i];
        concat_hash[i+HASH_LEN] = this->right->hash[i];
    }
    SHA256_wrapper((void*)concat_hash, HASH_LEN*2, this->hash);
}

Node *
Node::append_leaf(int leaf_cnt, int index, Node *new_leaf)
{
    for(int i = index; i >= 0; i--){
        if(leaf_cnt > pow_of_2[i]){
            this->right = this->right->append_leaf(leaf_cnt - pow_of_2[i], i-1, new_leaf);
            break;
        }
        else if(leaf_cnt == pow_of_2[i]){
            Node *new_node = new Node(this, new_leaf);
            return new_node;
        }
    }
    this->recompute_hash();
    return this;
}

inline Node *
Node::append_leaf(int height, Node* new_leaf, Work* node_path)
{
    // printf("node::append_leaf, height : %d\n", height);
    if(height != 0) {
        this->right = this->right->append_leaf(height-1, new_leaf, node_path);
    }
    else {
        Node* new_node = new Node(this, new_leaf);
        node_path->append_path(this);
        node_path->add_leaf(new_leaf);
        return new_node;
    }
    this->recompute_hash();
    node_path->append_path(this->left);
    return this;
}

/*********** Tree ***********/
Tree::Tree(unordered_map<Node*, unsigned char*> leaf_map, 
        sem_t *sem, 
        pthread_mutex_t *channel_mutex, 
        deque<Work*> *work_channel
    )
{
    this->leaf_map = leaf_map;
    this->sem = sem;
    this->leaf_cnt = 1;
    this->pow_2_ind = 0;
    this->root = new Node();
    this->work_channel = work_channel;
    this->channel_mutex = channel_mutex;
}

void
Tree::append_leaf (const unsigned char* comm)
{
    Work* node_path;
    Node* new_leaf = new Node(comm);
    int curr_ind = this->pow_2_ind, curr_leaf_cnt = this->leaf_cnt;
    int path_cnt = 0;

    for(int i = curr_ind; i >= 0; i--){
        if(curr_leaf_cnt > pow_of_2[i]){
            path_cnt++; 
            curr_leaf_cnt -= pow_of_2[i];
        }
        else if(curr_leaf_cnt == pow_of_2[i]){
            break;
        }
    }
    node_path = new Work(path_cnt);
    // printf("curr height : %d, curr leaf_cnt : %d\n", path_cnt, this->leaf_cnt);

    // printf("main thread: ");
    // new_leaf->print_hash();
    // this->root = this->root->append_leaf(this->leaf_cnt, this->pow_2_ind, new_leaf);
    this->root = this->root->append_leaf(path_cnt, new_leaf, node_path);
    node_path->add_root(this->root);
    // printf("Tree::append_leaf, after node::append_leaf\n");

    this->leaf_cnt++;
    if(pow_of_2[this->pow_2_ind+1] <= this->leaf_cnt) this->pow_2_ind++; 

    pthread_mutex_lock(this->channel_mutex);
        this->work_channel->push_back(node_path);
    pthread_mutex_unlock(this->channel_mutex);
    sem_post(this->sem); 
}

#endif