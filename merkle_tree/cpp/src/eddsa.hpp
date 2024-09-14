#ifndef EDDSA_H
#define EDDSA_H
#include <openssl/evp.h>
#include <openssl/ct.h>

EVP_PKEY *
eddsa_gen_key()
{
    EVP_PKEY *pkey = NULL;
    EVP_PKEY_CTX *pctx = EVP_PKEY_CTX_new_id(EVP_PKEY_ED25519, NULL);
    if(!pctx)
        { fprintf(stderr, "error pctx\n"); return NULL; }
    if(EVP_PKEY_keygen_init(pctx) <= 0) 
        { fprintf(stderr, "keygen init error\n"); return NULL; }
    if(EVP_PKEY_keygen(pctx, &pkey) <= 0) 
        { fprintf(stderr, "keygen error\n"); return NULL; }
    EVP_PKEY_CTX_free(pctx);
    return pkey;
}

/*
 * read edkey and write it to pk
 * returns the length of pk
 */
int
eddsa_get_pubkey_to_byte(unsigned char **pk, EVP_PKEY *edkey)
{ return  i2d_PUBKEY(edkey, pk); }


/*
 * read pk and len of pk, and create EVP_PKEY
 * returns the created instance of EVP_PKEY
 */
EVP_PKEY * 
eddsa_get_pubkey_from_byte(const unsigned char **pk, long len)
{ return d2i_PUBKEY(NULL, pk, len); }

int
eddsa_do_sign(unsigned char *msg, size_t msg_len, unsigned char **sig, EVP_PKEY *edkey)
{
    size_t sig_len;
    EVP_MD_CTX *md_ctx = EVP_MD_CTX_new();

    EVP_DigestSignInit(md_ctx, NULL, NULL, NULL, edkey);
    /* Calculate the requires size for the signature by passing a NULL buffer */
    EVP_DigestSign(md_ctx, NULL, &sig_len, msg, msg_len);
    *sig = (unsigned char*)OPENSSL_zalloc(sig_len);
    EVP_DigestSign(md_ctx, *sig, &sig_len, msg, msg_len);
    EVP_MD_CTX_free(md_ctx);
    return (int)sig_len;
}

/*
 * return 1 on correct signature, incorrect otherwise
 */
int
eddsa_verify_sign(const unsigned char* msg, size_t msg_len, 
                  const unsigned char *sig, size_t sig_len, EVP_PKEY *edkey)
{
    EVP_MD_CTX *md_ctx = EVP_MD_CTX_new();
    EVP_DigestVerifyInit(md_ctx, NULL, NULL, NULL, edkey);
    int ret = EVP_DigestVerify(md_ctx, sig, sig_len, msg, msg_len);
    EVP_MD_CTX_free(md_ctx);
    return ret;
}
#endif
