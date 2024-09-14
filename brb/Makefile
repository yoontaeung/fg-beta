CC = g++
LDFLAGS = -lsalticidae -lssl -lpthread -lcrypto -luv
SRC_DIR = ./src/
BIN_DIR = ./bin/
TARGET_SEQ = sequencer
TARGET_ENC = test_enc

seq:
	$(CC) $(SRC_DIR)$(TARGET_SEQ).cpp -o $(BIN_DIR)$(TARGET_SEQ) $(LDFLAGS)

enc:
	$(CC) $(SRC_DIR)$(TARGET_ENC).cpp -o $(BIN_DIR)$(TARGET_ENC) $(LDFLAGS)

clean: 
	rm bin/* eval/* figure/*.png log/*

del:
	rm eval/* figure/*.png log/*
