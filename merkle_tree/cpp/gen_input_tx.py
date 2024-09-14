f = open("input_tx", "w")
for i in range(1_000_000):
	f.write("hello world"+str(i)+"\n")
f.close()
