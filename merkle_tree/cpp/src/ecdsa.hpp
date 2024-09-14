#ifndef ECDSA
#include <openssl/ecdsa.h>
#include <openssl/ct.h>
#include <openssl/crypto.h>
#endif
#define ECDSA

void
print_hash(const unsigned char *msg, int msg_len)
{
	printf("print_hash(): msg_len : %d\n", msg_len);
	for(int i = 0; i < msg_len; i++)
		printf("%02x", msg[i]);
	printf("\n");
}

EC_KEY *
ecdsa_gen_key()
{
	EC_KEY *eckey;
	eckey = EC_KEY_new_by_curve_name(714); // secp256k1

	if(eckey == NULL) {
		printf("key gen error\n"); return NULL;
	}
	if(!EC_KEY_generate_key(eckey)) {
		printf("real key gen error error\n"); return NULL;
	}
	return eckey;
}

unsigned int
ecdsa_get_pubkey_to_byte(unsigned char **pk, EC_KEY *eckey)
{
	return i2d_EC_PUBKEY(eckey, pk);
}

EC_KEY *
ecdsa_get_pubkey_from_byte(const unsigned char **pk_buf, unsigned int pk_buf_len)
{
	#ifdef DEBUG
		print_hash(*pk_buf, pk_buf_len);
	#endif
	EC_KEY * ret = d2i_EC_PUBKEY(NULL, pk_buf, pk_buf_len);
	return ret;
}

int
ecdsa_do_sign(const unsigned char *dgst, int dgstlen, unsigned char **signature, EC_KEY *eckey)
{
	unsigned int signature_len = ECDSA_size(eckey);
	*signature = (unsigned char*)OPENSSL_malloc(signature_len);

	if(!ECDSA_sign(0, dgst, dgstlen, *signature, &signature_len, eckey)){
		printf("error\n"); return -1;
	}
	return signature_len;
}

int
ecdsa_verify_sign(const unsigned char *dgst, int dgst_len, 
				  const unsigned char *sign, int sign_len, EC_KEY *pk)
{
	#ifdef DEBUG
		print_hash(dgst, dgst_len);
		print_hash(sign, sign_len);
	#endif
	int ret = ECDSA_verify(0, dgst, 32, sign, sign_len, pk);

	if(ret == -1){
		#ifdef DEBUG
			printf("verify err\n"); 
		#endif
		return -1;
	}
	else if(ret == 0){
		#ifdef DEBUG
			printf("incorrect signature\n");
		#endif
		return 0;
	}
	else if(ret == 1){
		#ifdef DEBUG
			printf("correct signature\n");
		#endif
		return 1;
	}
	return 0;
}