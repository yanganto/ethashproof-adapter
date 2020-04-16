package main

import "C"

//export EthashProof
func EthashProof(input uint64)  *C.char {
	return C.CString("[]")
}

func main() {}
