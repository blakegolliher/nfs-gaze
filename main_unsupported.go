//go:build !linux

package main

import (
	"fmt"
	"os"
)

func main() {
	fmt.Println("This tool is only supported on Linux.")
	os.Exit(1)
}
