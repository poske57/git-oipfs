package main

import (
	"fmt"
	"log"
	git "github.com/go-git/go-git/v6"
)

func main() {
    hash, err := headHash()
	if err != nil {
		log.Fatal(err)
	}
	fmt.Println(hash)
}

func headHash() (string, error) {
	r, err := git.PlainOpen("/home/poske/Projects/git-oipfs/")
	if err != nil {
		return "", err
	}

	ref, err := r.Head()
	if err != nil {
		return "", err
	}

	return ref.Hash().String(), nil
}
