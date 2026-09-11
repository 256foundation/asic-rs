// Example: scan a subnet and print each discovered miner.
//
//	ASIC_SUBNET=192.168.1.0/24 go run ./examples/scan
package main

import (
	"fmt"
	"log"
	"os"

	"github.com/256foundation/asic-rs/go/asicrs"
)

func main() {
	subnet := os.Getenv("ASIC_SUBNET")
	if subnet == "" {
		subnet = "192.168.1.0/24"
		fmt.Fprintf(os.Stderr, "ASIC_SUBNET not set; trying %s\n", subnet)
	}

	factory, err := asicrs.NewFactoryFromSubnet(subnet)
	if err != nil {
		log.Fatal(err)
	}
	defer factory.Close()
	factory.WithConcurrentLimit(2500)

	miners, err := factory.Scan()
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("found %d miner(s)\n", len(miners))
	for _, miner := range miners {
		summary, _ := miner.Summary()
		fmt.Println(summary)
		miner.Close()
	}
}
