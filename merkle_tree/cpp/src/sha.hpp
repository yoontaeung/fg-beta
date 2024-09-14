#define SHAHPP
#include <openssl/sha.h>
bool
SHA256_wrapper(void* input, unsigned long length, unsigned char* md)
{
	SHA256_CTX context;
	if(!SHA256_Init(&context))
		return false;
	if(!SHA256_Update(&context, (unsigned char*) input, length))
		return false;
	if(!SHA256_Final(md, &context))
		return false;
	return true;
}