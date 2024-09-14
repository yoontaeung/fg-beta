CC = g++
LDFLAGS=-lcrypto 
MAIN_TARGET=src/main.cpp
TEMP_MAIN_TARGET=src/hash_consuming_main.cpp
GEN_TARGET=src/gen_signature.cpp
VER_TARGET=src/verify_signature.cpp
PROOF_TARGET=src/proof_verifier.cpp
HASH_TARGET=src/gen_hash.cpp

main:
	$(CC) $(MAIN_TARGET) -D POI -o bin/a.out -O3 $(LDFLAGS) -lpthread

temp_main:
	$(CC) $(TEMP_MAIN_TARGET) -D POI -o bin/temp_main -O3 $(LDFLAGS) -lpthread

gen:
	$(CC) $(GEN_TARGET) -o bin/gen_sig $(LDFLAGS)

ver:
	$(CC) $(VER_TARGET) -o bin/ver_sig $(LDFLAGS)

proof:
	$(CC) $(PROOF_TARGET) -o bin/proof_ver $(LDFLAGS)

hash:
	$(CC) $(HASH_TARGET) -o bin/gen_hash $(LDFLAGS)

clean:
	rm ./bin/* ./tx_data/*
